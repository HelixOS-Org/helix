#!/usr/bin/env python3
"""
Replace fnv1a_hash(format!(...).as_bytes()) with zero-alloc FastHasher.

Pattern: fnv1a_hash(format!("prefix-{}-{}", a, b).as_bytes())
Replace: FastHasher::new().feed_str("prefix").feed_u64(a as u64).feed_u64(b as u64).finish()

This eliminates heap allocation (String creation) on hot paths.
"""

import os
import re
import sys

NEXUS_SRC = os.path.join(os.path.dirname(__file__), '..', 'subsystems', 'nexus', 'src')

stats = {'files_modified': 0, 'patterns_fixed': 0}


def fix_format_hash(filepath: str) -> bool:
    """Replace fnv1a_hash(format!(...)) with FastHasher chain."""
    with open(filepath, 'r') as f:
        content = f.read()

    if 'fnv1a_hash(format!' not in content:
        return False

    original = content
    modified = False

    # Add FastHasher import if not present
    if 'FastHasher' not in content:
        if 'use crate::fast::' in content:
            # Extend existing fast import
            content = content.replace(
                'use crate::fast::',
                'use crate::fast::fast_hash::FastHasher;\nuse crate::fast::'
            )
        elif 'extern crate alloc;' in content:
            content = content.replace(
                'extern crate alloc;',
                'extern crate alloc;\n\nuse crate::fast::fast_hash::FastHasher;'
            )
        else:
            # Add at top after module doc
            lines = content.split('\n')
            insert_idx = 0
            for i, line in enumerate(lines):
                if line.startswith('use ') or line.startswith('extern'):
                    insert_idx = i
                    break
                if not line.startswith('//') and not line.startswith('!') and line.strip():
                    insert_idx = i
                    break
            lines.insert(insert_idx, 'use crate::fast::fast_hash::FastHasher;')
            content = '\n'.join(lines)

    # Now replace patterns line by line
    lines = content.split('\n')
    new_lines = []
    for line in lines:
        if 'fnv1a_hash(format!' in line:
            # Extract the format string and arguments
            # Pattern: fnv1a_hash(format!("...", args).as_bytes())
            match = re.search(
                r'fnv1a_hash\(format!\("([^"]*)"(?:,\s*(.+?))?\)\.as_bytes\(\)\)',
                line
            )
            if match:
                fmt_str = match.group(1)
                args_str = match.group(2) or ""

                # Build FastHasher chain
                # Split format string by {} and {:?} placeholders
                parts = re.split(r'\{[^}]*\}', fmt_str)
                args = [a.strip() for a in args_str.split(',') if a.strip()] if args_str else []

                # Build the chain
                chain = 'FastHasher::new()'

                # Interleave string parts and arguments
                for i, part in enumerate(parts):
                    if part:
                        chain += f'.feed_str("{part}")'
                    if i < len(args):
                        arg = args[i]
                        # Determine feed method
                        if 'tick' in arg.lower() or 'id' in arg.lower() or \
                           arg.startswith('i') or arg.endswith('.0'):
                            chain += f'.feed_u64({arg} as u64)'
                        elif arg.startswith('self.'):
                            chain += f'.feed_u64({arg} as u64)'
                        else:
                            chain += f'.feed_u64({arg} as u64)'

                chain += '.finish()'

                # Replace in line
                old_pattern = match.group(0)
                line = line.replace(old_pattern, chain)
                stats['patterns_fixed'] += 1
                modified = True

        new_lines.append(line)

    if modified:
        content = '\n'.join(new_lines)
        with open(filepath, 'w') as f:
            f.write(content)
        stats['files_modified'] += 1

    return modified


def main():
    nexus_src = os.path.abspath(NEXUS_SRC)
    print(f"Scanning {nexus_src} for fnv1a_hash(format!()) patterns...")

    for root, dirs, files in os.walk(nexus_src):
        for fname in files:
            if not fname.endswith('.rs'):
                continue
            fix_format_hash(os.path.join(root, fname))

    print(f"\n{'='*60}")
    print(f"format!() → FastHasher Migration COMPLETE")
    print(f"{'='*60}")
    print(f"  Files modified:    {stats['files_modified']}")
    print(f"  Patterns fixed:    {stats['patterns_fixed']}")
    print(f"  Heap allocs saved: {stats['patterns_fixed']} per tick cycle")


if __name__ == '__main__':
    main()
