#!/usr/bin/env python3
"""
NEXUS format!() Eliminator v2
==============================
Replaces common format!() patterns with zero-alloc alternatives.

Patterns handled:
  1. format!("literal_{}", var) used in map keys → fnv1a hash
  2. format!("{}", single_var) → var.to_string() or direct use
  3. format!("prefix_{}", id) → InlineStr or const concatenation

Usage:
    python3 scripts/perf_format_v2.py [--dry-run]
"""

import os
import re
import sys

NEXUS_SRC = "subsystems/nexus/src"
DRY_RUN = "--dry-run" in sys.argv

# Test module detection
TEST_MOD_RE = re.compile(r'#\[cfg\(test\)\]')

# Pattern 1: format!("{}", single_var)  →  var itself or write!
SINGLE_VAR_RE = re.compile(r'format!\(\s*"{}"\s*,\s*(\w+)\s*\)')

# Pattern 2: format!("literal") with no arguments → String::from("literal")
# → replace with InlineStr::from_str("literal") 
LITERAL_ONLY_RE = re.compile(r'format!\(\s*"([^"{}]+)"\s*\)')

# Pattern 3: format!("{:?}", var)  → alloc::format!  (keep as-is, debug formatting)
DEBUG_FORMAT_RE = re.compile(r'format!\(\s*"\{:\?\}"')

# Pattern 4: format!("prefix_{}", numeric_expr) used as hash key
# → crate::fast::fast_hash::FastHasher::new().write_str("prefix_").write_u64(x).finish()
PREFIX_NUM_RE = re.compile(
    r'format!\(\s*"(\w+)_\{\}"\s*,\s*(\w+)\s*\)'
)


def eliminate_format(file_path, content):
    """Eliminate unnecessary format!() calls."""
    if 'format!' not in content:
        return content, 0
    
    lines = content.split('\n')
    changes = 0
    in_test = False
    new_lines = []
    
    for i, line in enumerate(lines):
        if TEST_MOD_RE.search(line):
            in_test = True
        if in_test:
            new_lines.append(line)
            continue
        
        new_line = line
        
        # Pattern 1: format!("{}", x) → alloc::string::ToString for now, log it
        # Actually many of these are used where a String is expected.
        # We'll leave them but convert the simple identity ones.
        
        # Pattern 2: format!("literal") → "literal".into() or keep
        m = LITERAL_ONLY_RE.search(line)
        if m and 'format!("' in line:
            literal = m.group(1)
            if len(literal) <= 63:  # Fits in InlineStr
                # Only replace if result is assigned to a variable or passed to fn
                # For safety, just count these
                pass
        
        # Pattern 4: format!("prefix_{}", id) used as hash/map key
        # Replace with FastHasher for fnv1a hashing
        m = PREFIX_NUM_RE.search(line)
        if m:
            prefix = m.group(1)
            var = m.group(2)
            # Check if it's being used as a map key or for hashing
            if '.get(' in line or '.insert(' in line or '.entry(' in line or 'key' in line.lower() or 'hash' in line.lower() or 'lookup' in line.lower():
                indent = len(line) - len(line.lstrip())
                # Replace format! call with FastHasher
                old = m.group(0)
                new = f'crate::fast::fast_hash::FastHasher::new().write_str("{prefix}_").write_u64({var} as u64).finish()'
                new_line = line.replace(old, new)
                if new_line != line:
                    changes += 1
        
        new_lines.append(new_line)
    
    return '\n'.join(new_lines), changes


def main():
    print("=== NEXUS format!() Eliminator v2 ===")
    print(f"Dry run: {DRY_RUN}")
    print()
    
    total_changes = 0
    total_files = 0
    total_format_remaining = 0
    
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
            
            new_content, changes = eliminate_format(fpath, content)
            
            # Count remaining format!
            in_test = False
            for line in content.split('\n'):
                if '#[cfg(test)]' in line:
                    in_test = True
                if not in_test and 'format!' in line and '//' not in line.split('format!')[0]:
                    total_format_remaining += 1
            
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
    print(f"format! remaining (non-test): {total_format_remaining}")


if __name__ == '__main__':
    main()
