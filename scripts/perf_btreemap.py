#!/usr/bin/env python3
"""
NEXUS BTreeMap Eliminator
=========================
Automatically replaces BTreeMap with stack-allocated alternatives.

Strategy:
  1. BTreeMap<u32, V> where V is primitive → ArrayMap<V, 32>
     (32 slots covers all enum discriminant maps in NEXUS)
  2. BTreeMap::new() init → ArrayMap::new(0) / LinearMap::new()
  3. .entry(x as u32).or_insert(0) += 1 → .inc(x as usize)
  4. .entry(x).or_insert(default) += val → .add(x, val)

Usage:
    python3 scripts/perf_btreemap.py [--dry-run]
"""

import os
import re
import sys
from pathlib import Path

NEXUS_SRC = "subsystems/nexus/src"
DRY_RUN = "--dry-run" in sys.argv

PRIMITIVE_TYPES = {'u64', 'u32', 'i64', 'i32', 'usize', 'f32', 'f64', 'u8', 'u16', 'bool'}

# ===== Pattern: BTreeMap<u32, primitive> in struct fields =====
# Matches: field_name: BTreeMap<u32, u64>,
FIELD_PATTERN = re.compile(
    r'(\s*)(pub\s+)?(\w+):\s*BTreeMap<u32,\s*(' + '|'.join(PRIMITIVE_TYPES) + r')>'
)

# ===== Pattern: BTreeMap<u32, primitive> in type aliases / fn returns =====
TYPE_PATTERN = re.compile(
    r'BTreeMap<u32,\s*(' + '|'.join(PRIMITIVE_TYPES) + r')>'
)

# ===== Pattern: BTreeMap::new() in field initialization =====
INIT_PATTERN = re.compile(r'(\w+):\s*BTreeMap::new\(\)')

# ===== Pattern: *self.field.entry(x as u32).or_insert(0) += 1 =====
ENTRY_INC_PATTERN = re.compile(
    r'\*self\.(\w+)\.entry\((\w+)(\s+as\s+u32)?\)\.or_insert\(0\)\s*\+=\s*1'
)

# ===== Pattern: *self.field.entry(x as u32).or_insert(default) += val =====
ENTRY_ADD_PATTERN = re.compile(
    r'\*self\.(\w+)\.entry\((\w+)(\s+as\s+u32)?\)\.or_insert\([\d.]+\)\s*\+=\s*(\w+)'
)

# ===== Pattern: self.field.get(&key) =====
GET_PATTERN = re.compile(
    r'self\.(\w+)\.get\(&(\w+)(\s+as\s+u32)?\)'
)

# ===== Pattern: use alloc::collections::BTreeMap; =====
IMPORT_PATTERN = re.compile(r'use alloc::collections::BTreeMap;')


def migrate_file(file_path, content):
    """Migrate a single file's BTreeMap<u32, V> patterns to ArrayMap."""

    # First, find all fields that are BTreeMap<u32, primitive>
    u32_fields = set()
    for m in FIELD_PATTERN.finditer(content):
        field_name = m.group(3)
        u32_fields.add(field_name)

    if not u32_fields:
        return content, 0

    changes = 0
    lines = content.split('\n')
    new_lines = []
    added_import = False

    for i, line in enumerate(lines):
        new_line = line

        # Replace BTreeMap<u32, V> field declarations with ArrayMap<V, 32>
        fm = FIELD_PATTERN.search(line)
        if fm:
            val_type = fm.group(4)
            default_val = '0' if val_type in ('u64', 'u32', 'i64', 'i32', 'usize', 'u8', 'u16') else '0.0' if val_type in ('f32', 'f64') else 'false'
            new_line = FIELD_PATTERN.sub(
                lambda m: f"{m.group(1)}{m.group(2) or ''}{m.group(3)}: ArrayMap<{m.group(4)}, 32>",
                line
            )
            changes += 1

        # Replace BTreeMap::new() for known u32 fields
        im = INIT_PATTERN.search(line)
        if im and im.group(1) in u32_fields:
            field = im.group(1)
            # Determine value type from our field scan
            for fm2 in FIELD_PATTERN.finditer(content):
                if fm2.group(3) == field:
                    vtype = fm2.group(4)
                    default = '0' if vtype in ('u64', 'u32', 'i64', 'i32', 'usize', 'u8', 'u16') else '0.0' if vtype in ('f32', 'f64') else 'false'
                    new_line = line.replace('BTreeMap::new()', f'ArrayMap::new({default})')
                    changes += 1
                    break

        # Replace *self.field.entry(x as u32).or_insert(0) += 1
        em = ENTRY_INC_PATTERN.search(line)
        if em and em.group(1) in u32_fields:
            field = em.group(1)
            key_expr = em.group(2)
            indent = len(line) - len(line.lstrip())
            new_line = ' ' * indent + f'self.{field}.inc({key_expr} as usize);'
            changes += 1

        # Replace *self.field.entry(x as u32).or_insert(default) += val
        am = ENTRY_ADD_PATTERN.search(line)
        if am and am.group(1) in u32_fields:
            field = am.group(1)
            key_expr = am.group(2)
            val_expr = am.group(4)
            indent = len(line) - len(line.lstrip())
            new_line = ' ' * indent + f'self.{field}.add({key_expr} as usize, {val_expr});'
            changes += 1

        # Replace self.field.get(&key) for u32 fields
        gm = GET_PATTERN.search(line)
        if gm and gm.group(1) in u32_fields:
            field = gm.group(1)
            key_expr = gm.group(2)
            new_line = line.replace(
                gm.group(0),
                f'self.{field}.try_get({key_expr} as usize)'
            )
            changes += 1

        # Add ArrayMap import if we made changes and haven't added it yet
        if changes > 0 and not added_import and IMPORT_PATTERN.search(line):
            new_lines.append(line)
            new_lines.append('use crate::fast::array_map::ArrayMap;')
            added_import = True
            continue

        new_lines.append(new_line)

    if changes > 0 and not added_import:
        # Add import at the top (after existing use statements)
        for i, line in enumerate(new_lines):
            if line.startswith('use ') or line.strip().startswith('use '):
                new_lines.insert(i, 'use crate::fast::array_map::ArrayMap;')
                break

    return '\n'.join(new_lines), changes


def main():
    print("=== NEXUS BTreeMap<u32, V> → ArrayMap Migrator ===")
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

            if 'BTreeMap<u32,' not in content:
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
