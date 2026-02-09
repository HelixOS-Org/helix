#!/usr/bin/env python3
"""
NEXUS BTreeMap<u64, V> → LinearMap Migrator
============================================
Converts BTreeMap<u64, primitive> to LinearMap<primitive, 64> where:
- The field is in a struct (not a local variable)
- The value type is a primitive
- The map is used with simple get/insert/remove/entry patterns

This does NOT convert:
- BTreeMap<u64, ComplexType> (non-primitive values)
- BTreeMap<u64, V> used in iteration with ordering requirements
- BTreeMap with .range() or .iter() that depend on key ordering

Usage:
    python3 scripts/perf_btreemap_u64.py [--dry-run]
"""

import os
import re
import sys

NEXUS_SRC = "subsystems/nexus/src"
DRY_RUN = "--dry-run" in sys.argv

PRIMITIVE_TYPES = {'u64', 'u32', 'i64', 'i32', 'usize', 'f32', 'f64', 'u8', 'u16', 'bool'}

# Field pattern: name: BTreeMap<u64, primitive>
FIELD_RE = re.compile(
    r'(\s*)(pub\s+)?(\w+):\s*BTreeMap<u64,\s*(' + '|'.join(PRIMITIVE_TYPES) + r')>'
)

# Init: field: BTreeMap::new()
INIT_RE = re.compile(r'(\w+):\s*BTreeMap::new\(\)')

# .entry(key).or_insert(0) += 1
ENTRY_INC_RE = re.compile(
    r'\*self\.(\w+)\.entry\((\w+)\)\.or_insert\(0\)\s*\+=\s*1'
)

# .entry(key).or_insert(default) += val
ENTRY_ADD_RE = re.compile(
    r'\*self\.(\w+)\.entry\((\w+)\)\.or_insert\([\d.]+\)\s*\+=\s*(\w+)'
)

# self.field.get(&key) → self.field.get(key)
GET_REF_RE = re.compile(r'self\.(\w+)\.get\(&(\w+)\)')

# self.field.insert(key, val) — stays the same
# self.field.remove(&key) → self.field.remove(key)
REMOVE_REF_RE = re.compile(r'self\.(\w+)\.remove\(&(\w+)\)')

# self.field.contains_key(&key) → self.field.contains_key(key)
CONTAINS_RE = re.compile(r'self\.(\w+)\.contains_key\(&(\w+)\)')

# SKIP if file uses .range() or ordered iteration
ORDERED_PATTERNS = ['.range(', '.iter().rev()']

IMPORT_RE = re.compile(r'use alloc::collections::BTreeMap;')


def migrate_file(file_path, content):
    """Migrate BTreeMap<u64, primitive> → LinearMap."""
    # Skip files that use ordered features
    for pat in ORDERED_PATTERNS:
        if pat in content:
            return content, 0

    # Find u64 fields with primitive values
    u64_fields = {}  # name -> value_type
    for m in FIELD_RE.finditer(content):
        u64_fields[m.group(3)] = m.group(4)

    if not u64_fields:
        return content, 0

    lines = content.split('\n')
    new_lines = []
    changes = 0
    added_import = False

    for i, line in enumerate(lines):
        new_line = line

        # Replace field declaration
        fm = FIELD_RE.search(line)
        if fm:
            new_line = FIELD_RE.sub(
                lambda m: f"{m.group(1)}{m.group(2) or ''}{m.group(3)}: LinearMap<{m.group(4)}, 64>",
                line
            )
            changes += 1

        # Replace BTreeMap::new() for known fields
        im = INIT_RE.search(new_line)
        if im and im.group(1) in u64_fields:
            new_line = new_line.replace('BTreeMap::new()', 'LinearMap::new()')
            changes += 1

        # Replace entry(key).or_insert(0) += 1
        em = ENTRY_INC_RE.search(line)
        if em and em.group(1) in u64_fields:
            field = em.group(1)
            key = em.group(2)
            indent = len(line) - len(line.lstrip())
            new_line = ' ' * indent + f'self.{field}.inc({key});'
            changes += 1

        # Replace entry(key).or_insert(d) += val
        am = ENTRY_ADD_RE.search(line)
        if am and am.group(1) in u64_fields:
            field = am.group(1)
            key = am.group(2)
            val = am.group(3)
            indent = len(line) - len(line.lstrip())
            new_line = ' ' * indent + f'self.{field}.add({key}, {val});'
            changes += 1

        # Replace get(&key) → get(key)
        gm = GET_REF_RE.search(new_line)
        if gm and gm.group(1) in u64_fields:
            new_line = new_line.replace(gm.group(0), f'self.{gm.group(1)}.get({gm.group(2)})')
            changes += 1

        # Replace remove(&key) → remove(key)
        rm = REMOVE_REF_RE.search(new_line)
        if rm and rm.group(1) in u64_fields:
            new_line = new_line.replace(rm.group(0), f'self.{rm.group(1)}.remove({rm.group(2)})')
            changes += 1

        # Replace contains_key(&key) → contains_key(key)
        cm = CONTAINS_RE.search(new_line)
        if cm and cm.group(1) in u64_fields:
            new_line = new_line.replace(cm.group(0), f'self.{cm.group(1)}.contains_key({cm.group(2)})')
            changes += 1

        # Add import after existing BTreeMap import
        if changes > 0 and not added_import and IMPORT_RE.search(line):
            new_lines.append(new_line)
            new_lines.append('use crate::fast::linear_map::LinearMap;')
            added_import = True
            continue

        new_lines.append(new_line)

    if changes > 0 and not added_import:
        for i, line in enumerate(new_lines):
            if line.startswith('use ') or line.strip().startswith('use '):
                new_lines.insert(i, 'use crate::fast::linear_map::LinearMap;')
                break

    return '\n'.join(new_lines), changes


def main():
    print("=== NEXUS BTreeMap<u64, V> → LinearMap<V, 64> Migrator ===")
    print(f"Dry run: {DRY_RUN}")
    print()

    total_changes = 0
    total_files = 0

    for root, dirs, files in os.walk(NEXUS_SRC):
        dirs[:] = [d for d in dirs if d != 'tests']
        for fname in files:
            if not fname.endswith('.rs'):
                continue
            fpath = os.path.join(root, fname)

            try:
                with open(fpath, 'r', encoding='utf-8') as f:
                    content = f.read()
            except (UnicodeDecodeError, IOError):
                continue

            if 'BTreeMap<u64,' not in content:
                continue

            new_content, changes = migrate_file(fpath, content)
            if changes > 0:
                total_files += 1
                total_changes += changes
                if DRY_RUN:
                    print(f"  {fpath}: {changes} changes")
                else:
                    with open(fpath, 'w', encoding='utf-8') as f:
                        f.write(new_content)
                    print(f"  ✓ {fpath}: {changes} changes")

    print(f"\n=== RESULTS ===")
    print(f"Files modified: {total_files}")
    print(f"Total changes:  {total_changes}")


if __name__ == '__main__':
    main()
