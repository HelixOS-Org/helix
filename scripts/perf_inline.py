#!/usr/bin/env python3
"""
NEXUS Performance: Add #[inline] to EMA/smoothing functions.

Scans for functions that contain EMA update patterns (alpha * new + (1-alpha) * old)
and adds #[inline] annotation if not already present.

Also adds #[repr(align(64))] to hot structs in conscious/ and scheduler/ modules.
"""

import os
import re
import sys

NEXUS_SRC = os.path.join(os.path.dirname(__file__), '..', 'subsystems', 'nexus', 'src')

stats = {
    'files_scanned': 0,
    'files_modified': 0,
    'inline_added': 0,
    'align_added': 0,
}

# EMA patterns: alpha * val + (1-alpha) * old  OR  old + alpha * (val - old)
EMA_PATTERNS = [
    re.compile(r'self\.\w+\s*=\s*[\w.]+\s*\*\s*[\w.]+\s*\+\s*\(1\.0\s*-'),  # alpha*x + (1-alpha)*y
    re.compile(r'self\.ema\s*='),
    re.compile(r'self\.ewma\s*='),
    re.compile(r'self\.smoothed\s*=.*\*'),
    re.compile(r'self\.avg\s*=.*alpha'),
    re.compile(r'self\.average\s*=.*\*.*\+.*\*'),
    re.compile(r'self\.\w+_ema\s*='),
    re.compile(r'EMA_ALPHA\s*\*'),
    re.compile(r'ema_alpha\s*\*'),
    re.compile(r'self\.load_avg\s*='),
    re.compile(r'self\.cpu_ema\s*='),
    re.compile(r'self\.memory_ema\s*='),
    re.compile(r'self\.latency_avg\s*=.*\*'),
    re.compile(r'self\.moving_avg\s*='),
    re.compile(r'self\.trend\s*=.*\*.*\+.*\*'),
    re.compile(r'alpha\s*\*\s*\w+\s*\+\s*\(1\.0\s*-\s*alpha\)'),
]

# Hot struct patterns - structs in performance-critical paths
HOT_STRUCT_DIRS = ['conscious', 'scheduler', 'bridge', 'workqueue']


def has_ema_pattern(line: str) -> bool:
    """Check if a line contains an EMA computation pattern."""
    for pat in EMA_PATTERNS:
        if pat.search(line):
            return True
    return False


def add_inline_to_ema_functions(filepath: str) -> bool:
    """Add #[inline] to functions containing EMA patterns."""
    with open(filepath, 'r') as f:
        lines = f.readlines()

    modified = False
    # Find lines with EMA patterns
    ema_lines = set()
    for i, line in enumerate(lines):
        if has_ema_pattern(line):
            ema_lines.add(i)

    if not ema_lines:
        return False

    # For each EMA line, find the enclosing function and add #[inline]
    # Walk backwards to find the fn declaration
    fn_lines_to_annotate = set()
    for ema_line in ema_lines:
        for i in range(ema_line, max(ema_line - 50, -1), -1):
            line = lines[i].strip()
            if line.startswith('pub fn ') or line.startswith('fn ') or \
               line.startswith('pub(crate) fn ') or line.startswith('pub(super) fn '):
                # Check if #[inline] is already there
                if i > 0 and '#[inline' in lines[i-1]:
                    break
                if i > 1 and '#[inline' in lines[i-2]:
                    break
                fn_lines_to_annotate.add(i)
                break

    if not fn_lines_to_annotate:
        return False

    # Add #[inline] before each function (insert from bottom to top to preserve indices)
    new_lines = list(lines)
    offset = 0
    for fn_line in sorted(fn_lines_to_annotate):
        idx = fn_line + offset
        indent = len(new_lines[idx]) - len(new_lines[idx].lstrip())
        inline_line = ' ' * indent + '#[inline]\n'
        new_lines.insert(idx, inline_line)
        offset += 1
        stats['inline_added'] += 1
        modified = True

    if modified:
        with open(filepath, 'w') as f:
            f.writelines(new_lines)
        stats['files_modified'] += 1

    return modified


def add_cache_align_to_hot_structs(filepath: str) -> bool:
    """Add #[repr(align(64))] to hot structs in performance-critical modules."""
    # Only process files in hot directories
    rel = os.path.relpath(filepath, NEXUS_SRC)
    parts = rel.split(os.sep)
    if not parts or parts[0] not in HOT_STRUCT_DIRS:
        return False

    with open(filepath, 'r') as f:
        content = f.read()

    # Find structs that have EMA/tick/update fields (hot data)
    hot_field_patterns = [
        'ema', 'load', 'cpu_usage', 'tick_count', 'latency',
        'smoothed', 'average', 'ewma', 'trend',
    ]

    lines = content.split('\n')
    modified = False
    new_lines = []
    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()

        # Check if this is a struct definition
        if stripped.startswith('pub struct ') and '{' in stripped:
            # Look ahead to see if it has hot fields
            has_hot_field = False
            for j in range(i + 1, min(i + 30, len(lines))):
                inner = lines[j].strip().lower()
                if inner == '}':
                    break
                for pat in hot_field_patterns:
                    if pat in inner:
                        has_hot_field = True
                        break
                if has_hot_field:
                    break

            # Check if already has repr(align)
            if has_hot_field and i > 0:
                prev_lines = '\n'.join(lines[max(0, i-3):i])
                if '#[repr(align' not in prev_lines:
                    indent = len(line) - len(line.lstrip())
                    align_line = ' ' * indent + '#[repr(align(64))]'
                    new_lines.append(align_line)
                    stats['align_added'] += 1
                    modified = True

        new_lines.append(line)
        i += 1

    if modified:
        with open(filepath, 'w') as f:
            f.write('\n'.join(new_lines))
        stats['files_modified'] += 1

    return modified


def main():
    dry_run = '--dry-run' in sys.argv
    nexus_src = os.path.abspath(NEXUS_SRC)

    print(f"{'DRY RUN: ' if dry_run else ''}Scanning {nexus_src}...")
    print(f"Phase 1: Adding #[inline] to EMA functions...")

    for root, dirs, files in os.walk(nexus_src):
        for fname in files:
            if not fname.endswith('.rs'):
                continue
            filepath = os.path.join(root, fname)
            stats['files_scanned'] += 1

            if not dry_run:
                add_inline_to_ema_functions(filepath)

    inline_count = stats['inline_added']
    inline_files = stats['files_modified']

    print(f"Phase 2: Adding #[repr(align(64))] to hot structs...")

    for root, dirs, files in os.walk(nexus_src):
        for fname in files:
            if not fname.endswith('.rs'):
                continue
            filepath = os.path.join(root, fname)

            if not dry_run:
                add_cache_align_to_hot_structs(filepath)

    print(f"\n{'='*60}")
    print(f"NEXUS #[inline] + Cache Alignment Migration {'(DRY)' if dry_run else 'COMPLETE'}")
    print(f"{'='*60}")
    print(f"  Files scanned:          {stats['files_scanned']}")
    print(f"  Files modified:         {stats['files_modified']}")
    print(f"  #[inline] added:        {stats['inline_added']}")
    print(f"  #[repr(align(64))] added: {stats['align_added']}")


if __name__ == '__main__':
    main()
