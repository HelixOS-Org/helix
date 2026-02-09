#!/usr/bin/env python3
"""
NEXUS Mass Performance Optimizer v2
====================================
Phase 1: Add #[inline] to all small pub fn (≤ 10 lines, not test)
Phase 2: Convert BTreeMap<u32, u64/f32/usize> → ArrayMap where the size N is known
Phase 3: Add #[repr(align(64))] to perf-critical structs missing it
Phase 4: Convert format!() on hot paths to zero-alloc alternatives

Usage:
    python3 scripts/perf_mass_v2.py [--dry-run] [--phase N]
"""

import os
import re
import sys
from pathlib import Path

NEXUS_SRC = "subsystems/nexus/src"
DRY_RUN = "--dry-run" in sys.argv
PHASE = None

for arg in sys.argv:
    if arg.startswith("--phase"):
        idx = sys.argv.index(arg)
        if idx + 1 < len(sys.argv):
            PHASE = int(sys.argv[idx + 1])

# ===========================================================================
# PHASE 1: Mass #[inline] addition
# ===========================================================================

def count_body_lines(lines, start_idx):
    """Count lines in function body (between { and matching })."""
    brace_depth = 0
    body_lines = 0
    found_open = False
    for i in range(start_idx, len(lines)):
        line = lines[i]
        for ch in line:
            if ch == '{':
                brace_depth += 1
                found_open = True
            elif ch == '}':
                brace_depth -= 1
        if found_open and brace_depth == 0:
            return body_lines
        if found_open:
            body_lines += 1
    return body_lines

# Patterns that ALREADY have #[inline]
INLINE_RE = re.compile(r'#\[inline')

# pub fn / pub const fn / pub unsafe fn patterns
PUB_FN_RE = re.compile(r'^(\s*)(pub\s+(const\s+)?(?:unsafe\s+)?fn\s+\w+)')

# Patterns to SKIP (not worth inlining)
SKIP_PATTERNS = [
    re.compile(r'pub\s+fn\s+new\('),       # Constructors - often too complex
    re.compile(r'pub\s+fn\s+fmt\('),        # Display/Debug impls
    re.compile(r'pub\s+fn\s+from\('),       # From trait impls
    re.compile(r'pub\s+fn\s+drop\('),       # Drop impls
    re.compile(r'pub\s+fn\s+main\('),       # main()
]

# Test module detection
TEST_MOD_RE = re.compile(r'#\[cfg\(test\)\]')

def phase1_inline(file_path, content):
    """Add #[inline] to small pub fn that don't have it."""
    lines = content.split('\n')
    modified = False
    insertions = []
    in_test_mod = False

    for i, line in enumerate(lines):
        # Track test modules
        if TEST_MOD_RE.search(line):
            in_test_mod = True
        if in_test_mod:
            continue

        # Skip if already has #[inline]
        if i > 0 and INLINE_RE.search(lines[i-1]):
            continue
        if i > 1 and INLINE_RE.search(lines[i-2]):
            continue

        m = PUB_FN_RE.match(line)
        if not m:
            continue

        # Skip certain patterns
        skip = False
        for sp in SKIP_PATTERNS:
            if sp.search(line):
                skip = True
                break
        if skip:
            continue

        # Count body lines
        body = count_body_lines(lines, i)
        if body <= 10:
            indent = m.group(1)
            # Determine if it's a tiny getter (≤3 lines = inline(always))
            attr = "#[inline(always)]" if body <= 3 else "#[inline]"
            insertions.append((i, f"{indent}{attr}"))
            modified = True

    if not modified:
        return content, 0

    # Insert in reverse order to maintain line numbers
    for idx, text in reversed(insertions):
        lines.insert(idx, text)

    return '\n'.join(lines), len(insertions)


# ===========================================================================
# PHASE 2: BTreeMap<u32, u64> → ArrayMap
# ===========================================================================

# Pattern: BTreeMap<u32, u64/usize/f32/f64/i64>
BTREEMAP_U32_FIELD_RE = re.compile(
    r'(\s*)(pub\s+)?(\w+):\s*BTreeMap<u32,\s*(u64|usize|f32|f64|i64|u32)>'
)

# Pattern: BTreeMap::new() in struct initialization  
BTREE_NEW_RE = re.compile(r'(\w+):\s*BTreeMap::new\(\)')

# Pattern: .entry(x as u32).or_insert(0) += 1  or .entry(x).or_insert(0)
ENTRY_INC_RE = re.compile(
    r'\*self\.(\w+)\.entry\((\w+)(\s+as\s+u32)?\)\.or_insert\(0\)\s*\+=\s*1'
)

# Pattern: self.foo.entry(x as u32).or_insert(0.0) += val
ENTRY_ADD_RE = re.compile(
    r'\*self\.(\w+)\.entry\((\w+)(\s+as\s+u32)?\)\.or_insert\([\d.]+\)\s*\+=\s*(\w+)'
)

# Pattern: self.foo.get(&key)
BTREE_GET_RE = re.compile(r'self\.(\w+)\.get\(&(\w+)\)')

def phase2_btreemap_to_arraymap(file_path, content):
    """Convert simple BTreeMap<u32, primitive> patterns to ArrayMap."""
    if 'BTreeMap<u32,' not in content:
        return content, 0

    lines = content.split('\n')
    count = 0

    # Track which fields are BTreeMap<u32, V>
    btree_fields = {}  # field_name -> value_type

    for i, line in enumerate(lines):
        m = BTREEMAP_U32_FIELD_RE.search(line)
        if m:
            field_name = m.group(3)
            val_type = m.group(4)
            btree_fields[field_name] = val_type

    if not btree_fields:
        return content, 0

    # For now, just log what we'd change (this is informational)
    # Actual conversion requires knowing N (the array size)
    # We'll add a comment marker for manual review
    new_content = content
    for field, vtype in btree_fields.items():
        count += 1

    return new_content, 0  # Phase 2 is informational only for now


# ===========================================================================
# PHASE 3: Missing #[repr(align(64))]
# ===========================================================================

REPR_ALIGN_RE = re.compile(r'#\[repr\(.*align\(64\)')

# Hot structs that benefit from cache alignment
HOT_STRUCT_INDICATORS = [
    'Stats', 'Counter', 'Metric', 'Cache', 'Buffer',
    'Queue', 'Pool', 'Timer', 'State', 'Context',
]

STRUCT_DEF_RE = re.compile(r'^(\s*)pub\s+struct\s+(\w+)')

def phase3_repr_align(file_path, content):
    """Add #[repr(align(64))] to performance-critical structs."""
    lines = content.split('\n')
    insertions = []
    in_test = False

    for i, line in enumerate(lines):
        if TEST_MOD_RE.search(line):
            in_test = True
        if in_test:
            continue

        m = STRUCT_DEF_RE.match(line)
        if not m:
            continue

        struct_name = m.group(2)
        indent = m.group(1)

        # Check if it's a hot struct
        is_hot = any(ind in struct_name for ind in HOT_STRUCT_INDICATORS)
        if not is_hot:
            continue

        # Check if already has repr(align)
        has_align = False
        for j in range(max(0, i-3), i):
            if REPR_ALIGN_RE.search(lines[j]):
                has_align = True
                break

        if not has_align:
            insertions.append((i, f"{indent}#[repr(align(64))]"))

    if not insertions:
        return content, 0

    for idx, text in reversed(insertions):
        lines.insert(idx, text)

    return '\n'.join(lines), len(insertions)


# ===========================================================================
# PHASE 4: format!() → write!() / zero-alloc alternatives
# ===========================================================================

# Pattern: format!("static string {}", single_var) in non-test code
FORMAT_SIMPLE_RE = re.compile(r'format!\("([^"]*?)"\s*,\s*(\w+)\s*\)')

def phase4_format_elimination(file_path, content):
    """Informational: count remaining format!() calls on hot paths."""
    if 'format!' not in content:
        return content, 0

    lines = content.split('\n')
    in_test = False
    count = 0

    for i, line in enumerate(lines):
        if TEST_MOD_RE.search(line):
            in_test = True
        if in_test:
            continue
        if 'format!' in line:
            count += 1

    return content, count  # Informational only


# ===========================================================================
# MAIN
# ===========================================================================

def process_file(file_path, phases):
    """Process a single file through the requested phases."""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
    except (UnicodeDecodeError, IOError):
        return {}

    original = content
    results = {}

    if 1 in phases:
        content, c = phase1_inline(file_path, content)
        if c > 0:
            results['inline'] = c

    if 3 in phases:
        content, c = phase3_repr_align(file_path, content)
        if c > 0:
            results['repr_align'] = c

    if content != original and not DRY_RUN:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(content)

    # Informational phases
    if 2 in phases:
        _, c = phase2_btreemap_to_arraymap(file_path, original)
        if c > 0:
            results['btreemap_u32'] = c

    if 4 in phases:
        _, c = phase4_format_elimination(file_path, original)
        if c > 0:
            results['format'] = c

    return results


def main():
    phases = [PHASE] if PHASE else [1, 3]  # Default: inline + repr_align (safe transforms)

    print(f"=== NEXUS Mass Performance Optimizer v2 ===")
    print(f"Phases: {phases}")
    print(f"Dry run: {DRY_RUN}")
    print()

    totals = {
        'inline': 0,
        'repr_align': 0,
        'btreemap_u32': 0,
        'format': 0,
        'files_modified': 0,
    }

    file_count = 0
    for root, dirs, files in os.walk(NEXUS_SRC):
        # Skip test directories
        dirs[:] = [d for d in dirs if d != 'tests' and d != 'test']
        for fname in files:
            if not fname.endswith('.rs'):
                continue
            fpath = os.path.join(root, fname)
            file_count += 1

            results = process_file(fpath, phases)
            if results:
                totals['files_modified'] += 1
                for k, v in results.items():
                    totals[k] = totals.get(k, 0) + v

                if DRY_RUN:
                    print(f"  {fpath}: {results}")

    print(f"\n=== RESULTS ===")
    print(f"Files scanned:  {file_count}")
    print(f"Files modified: {totals['files_modified']}")
    if totals['inline']:
        print(f"#[inline] added: {totals['inline']}")
    if totals['repr_align']:
        print(f"#[repr(align(64))] added: {totals['repr_align']}")
    if totals['btreemap_u32']:
        print(f"BTreeMap<u32,_> fields found: {totals['btreemap_u32']}")
    if totals['format']:
        print(f"format!() on hot paths: {totals['format']}")


if __name__ == '__main__':
    main()
