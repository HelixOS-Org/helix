#!/usr/bin/env python3
"""
Performance Migration Script for NEXUS
Converts Vec-based sliding windows to VecDeque for O(1) pop_front.

Pattern detected:
  if self.X.len() >= N { self.X.remove(0); }
  self.X.push(val);

Converted to:
  if self.X.len() >= N { self.X.pop_front(); }
  self.X.push_back(val);

Also converts the field declaration and constructor.
"""

import os
import re
import sys

NEXUS_SRC = os.path.join(os.path.dirname(__file__), '..', 'subsystems', 'nexus', 'src')

# Files already fixed - skip them
SKIP_FILES = {
    'bridge/sched_ext.rs',
    'bridge/futex.rs',
    'coop/futex.rs',
    'holistic/futex_tracker.rs',
    'scheduler/classifier.rs',
    'scheduler/affinity.rs',
    'scheduler/preemption.rs',
    'scheduler/load.rs',
    'workqueue/predictor.rs',
    'workqueue/latency.rs',
    'fast/ring_buffer.rs',  # Our own module
}

stats = {
    'files_scanned': 0,
    'files_modified': 0,
    'remove0_fixed': 0,
    'push_fixed': 0,
    'vec_new_fixed': 0,
    'imports_added': 0,
    'field_decls_fixed': 0,
    'last_fixed': 0,
}


def find_queue_field_names(content: str) -> set:
    """Find all field names that have .remove(0) called on them."""
    # Match patterns like self.something.remove(0) or field.remove(0)
    pattern = re.compile(r'(?:self\.)?(\w+)\.remove\(0\)')
    fields = set()
    for m in pattern.finditer(content):
        fields.add(m.group(1))
    return fields


def fix_file(filepath: str, dry_run: bool = False) -> bool:
    """Fix a single file. Returns True if modified."""
    with open(filepath, 'r') as f:
        original = f.read()
    
    content = original
    queue_fields = find_queue_field_names(content)
    
    if not queue_fields:
        return False
    
    modified = False
    
    # 1. Fix .remove(0) → .pop_front() for queue fields
    for field in queue_fields:
        # self.field.remove(0) → self.field.pop_front()
        # Handle both Some(self.field.remove(0)) and standalone
        old = f'self.{field}.remove(0)'
        if old in content:
            # Check if return value is used (wrapped in Some() or assigned)
            # If it's in pattern: Some(self.X.remove(0)) → self.X.pop_front()
            content = content.replace(f'Some({old})', f'self.{field}.pop_front()')
            # If standalone or in if-let: self.X.remove(0) → self.X.pop_front().unwrap()
            # But be careful - if it's just for discarding (no assignment), use pop_front()
            # Check remaining instances
            lines = content.split('\n')
            new_lines = []
            for line in lines:
                if old in line:
                    stripped = line.strip()
                    # If the remove(0) result is discarded (statement by itself or in if block)
                    if stripped.startswith(f'self.{field}.remove(0);'):
                        line = line.replace(f'self.{field}.remove(0);', f'self.{field}.pop_front();')
                    elif f'let ' in stripped and old in stripped:
                        # let x = self.field.remove(0); → let x = self.field.pop_front().unwrap();
                        line = line.replace(old, f'self.{field}.pop_front().unwrap()')
                    elif f'= {old}' in line:
                        # assignment: x = self.field.remove(0) → x = self.field.pop_front().unwrap()
                        line = line.replace(old, f'self.{field}.pop_front().unwrap()')
                    else:
                        # Default: just use pop_front() (discard value)
                        line = line.replace(old, f'self.{field}.pop_front()')
                    stats['remove0_fixed'] += 1
                    modified = True
                new_lines.append(line)
            content = '\n'.join(new_lines)
        
        # Also handle non-self patterns: field.remove(0)
        plain_old = f'{field}.remove(0)'
        if plain_old in content and f'self.{field}' not in plain_old:
            lines = content.split('\n')
            new_lines = []
            for line in lines:
                if plain_old in line and 'self.' not in line.split(plain_old)[0].split('.')[-1:][0] if '.' in line.split(plain_old)[0] else True:
                    if f'Some({plain_old})' in line:
                        line = line.replace(f'Some({plain_old})', f'{field}.pop_front()')
                    elif line.strip().startswith(f'{plain_old};') or line.strip() == f'{plain_old};':
                        line = line.replace(f'{plain_old};', f'{field}.pop_front();')
                    else:
                        line = line.replace(plain_old, f'{field}.pop_front().unwrap()')
                    stats['remove0_fixed'] += 1
                    modified = True
                new_lines.append(line)
            content = '\n'.join(new_lines)
    
    # 2. Fix .push(x) → .push_back(x) for queue fields ONLY
    for field in queue_fields:
        # Match self.field.push(anything) but NOT self.field.push_back or push_front
        pattern = re.compile(
            rf'(self\.{re.escape(field)}\.push)\((?!_back|_front)'
        )
        new_content = pattern.sub(rf'self.{field}.push_back(', content)
        if new_content != content:
            count = len(pattern.findall(content))
            stats['push_fixed'] += count
            content = new_content
            modified = True
    
    # 3. Fix .last() → .back() for queue fields
    for field in queue_fields:
        old_last = f'self.{field}.last()'
        new_last = f'self.{field}.back()'
        if old_last in content:
            content = content.replace(old_last, new_last)
            stats['last_fixed'] += 1
            modified = True
    
    # 4. Fix Vec<T> → VecDeque<T> in field declarations for queue fields
    for field in queue_fields:
        # Match: pub field: Vec<Something> or field: Vec<Something>
        pattern = re.compile(
            rf'((?:pub\s+)?{re.escape(field)}\s*:\s*)Vec<([^>]+)>'
        )
        new_content = pattern.sub(rf'\1VecDeque<\2>', content)
        if new_content != content:
            stats['field_decls_fixed'] += 1
            content = new_content
            modified = True
    
    # 5. Fix Vec::new() → VecDeque::new() in constructors for queue fields
    for field in queue_fields:
        # Match: field: Vec::new()
        pattern = re.compile(
            rf'({re.escape(field)}\s*:\s*)Vec::new\(\)'
        )
        new_content = pattern.sub(rf'\1VecDeque::new()', content)
        if new_content != content:
            stats['vec_new_fixed'] += 1
            content = new_content
            modified = True
    
    # 6. Add VecDeque import if not present and we made changes
    if modified and 'VecDeque' not in original:
        # Find existing alloc imports to add alongside
        if 'use alloc::collections::BTreeMap;' in content:
            content = content.replace(
                'use alloc::collections::BTreeMap;',
                'use alloc::collections::BTreeMap;\nuse alloc::collections::VecDeque;'
            )
            stats['imports_added'] += 1
        elif 'use alloc::vec::Vec;' in content:
            content = content.replace(
                'use alloc::vec::Vec;',
                'use alloc::collections::VecDeque;\nuse alloc::vec::Vec;'
            )
            stats['imports_added'] += 1
        elif 'use alloc::string::String;' in content:
            content = content.replace(
                'use alloc::string::String;',
                'use alloc::collections::VecDeque;\nuse alloc::string::String;'
            )
            stats['imports_added'] += 1
        elif 'extern crate alloc;' in content:
            content = content.replace(
                'extern crate alloc;',
                'extern crate alloc;\n\nuse alloc::collections::VecDeque;'
            )
            stats['imports_added'] += 1
    
    if modified and not dry_run:
        with open(filepath, 'w') as f:
            f.write(content)
        stats['files_modified'] += 1
    
    return modified


def main():
    dry_run = '--dry-run' in sys.argv
    nexus_src = os.path.abspath(NEXUS_SRC)
    
    if dry_run:
        print("=== DRY RUN MODE ===\n")
    
    print(f"Scanning {nexus_src}...")
    
    for root, dirs, files in os.walk(nexus_src):
        for fname in files:
            if not fname.endswith('.rs'):
                continue
            
            filepath = os.path.join(root, fname)
            relpath = os.path.relpath(filepath, nexus_src)
            
            if relpath in SKIP_FILES:
                continue
            
            stats['files_scanned'] += 1
            
            if fix_file(filepath, dry_run=dry_run):
                print(f"  {'[DRY] ' if dry_run else ''}Fixed: {relpath}")
    
    print(f"\n{'='*60}")
    print(f"NEXUS Performance Migration {'(DRY RUN)' if dry_run else 'COMPLETE'}")
    print(f"{'='*60}")
    print(f"  Files scanned:      {stats['files_scanned']}")
    print(f"  Files modified:     {stats['files_modified']}")
    print(f"  .remove(0) → .pop_front():  {stats['remove0_fixed']}")
    print(f"  .push() → .push_back():     {stats['push_fixed']}")
    print(f"  Vec::new() → VecDeque:       {stats['vec_new_fixed']}")
    print(f"  Field decls converted:       {stats['field_decls_fixed']}")
    print(f"  Imports added:               {stats['imports_added']}")
    print(f"  .last() → .back():           {stats['last_fixed']}")


if __name__ == '__main__':
    main()
