#!/usr/bin/env python3
"""
╔══════════════════════════════════════════════════════════════════════════════╗
║  HELIX OS — NEXUS "First Contact" Stability Harness                        ║
║                                                                            ║
║  Automates:                                                                ║
║    1. cargo build  (-p helix-minimal-os, --target x86_64-unknown-none)     ║
║    2. QEMU launch  (-serial stdio, -d guest_errors,int, -no-reboot)       ║
║    3. Log capture   (stdout + stderr → timestamped file)                   ║
║    4. Telemetry     (filters [NEXUS], [CORE], panics, faults)             ║
║                                                                            ║
║  Usage:                                                                    ║
║    python3 scripts/nexus/first_contact.py                                  ║
║    python3 scripts/nexus/first_contact.py --cycles 5 --timeout 30          ║
║    python3 scripts/nexus/first_contact.py --features nexus-lite            ║
║    python3 scripts/nexus/first_contact.py --skip-build                     ║
╚══════════════════════════════════════════════════════════════════════════════╝
"""

from __future__ import annotations

import argparse
import os
import re
import signal
import subprocess
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import TextIO


# ─────────────────────────────────────────────────────────────────────────────
# Constants
# ─────────────────────────────────────────────────────────────────────────────

HELIX_ROOT = Path(__file__).resolve().parent.parent.parent
BUILD_DIR = HELIX_ROOT / "build"
LOG_DIR = BUILD_DIR / "logs" / "nexus"
OUTPUT_DIR = BUILD_DIR / "output"
KERNEL_BIN = OUTPUT_DIR / "helix-kernel"

TARGET = "x86_64-unknown-none"
PACKAGE = "helix-minimal-os"

# ANSI colours (terminal only)
C_RESET = "\033[0m"
C_BOLD = "\033[1m"
C_DIM = "\033[2m"
C_RED = "\033[31m"
C_GREEN = "\033[32m"
C_YELLOW = "\033[33m"
C_BLUE = "\033[34m"
C_CYAN = "\033[36m"
C_MAGENTA = "\033[35m"

# Regex filters for telemetry extraction
TELEMETRY_PATTERNS: list[tuple[str, str, re.Pattern[str]]] = [
    ("NEXUS",  C_CYAN,    re.compile(r"^\s*\[NEXUS\]",    re.IGNORECASE)),
    ("CORE",   C_BLUE,    re.compile(r"^\s*\[CORE\]",     re.IGNORECASE)),
    ("HEAL",   C_GREEN,   re.compile(r"^\s*\[HEAL",       re.IGNORECASE)),
    ("PREDICT",C_MAGENTA, re.compile(r"^\s*\[PREDICT",    re.IGNORECASE)),
    ("PANIC",  C_RED,     re.compile(r"panic|PANIC|triple fault|TRIPLE FAULT|page fault|EXCEPTION", re.IGNORECASE)),
    ("BOOT",   C_YELLOW,  re.compile(r"^\s*\[BOOT\]|Booting|multiboot|Multiboot|_start", re.IGNORECASE)),
    ("SCHED",  C_GREEN,   re.compile(r"scheduler|SCHEDULER|Scheduler Adjust|timeslice", re.IGNORECASE)),
    ("MEM",    C_YELLOW,  re.compile(r"heap|allocat|page.table|memory.map|OOM|out.of.memory", re.IGNORECASE)),
    ("SERIAL", C_DIM,     re.compile(r"serial|Serial|COM1", re.IGNORECASE)),
]

# Critical failure signatures (abort the run immediately)
CRITICAL_PATTERNS = [
    re.compile(r"triple fault", re.IGNORECASE),
    re.compile(r"QEMU.*Shutting down", re.IGNORECASE),
]


# ─────────────────────────────────────────────────────────────────────────────
# Data structures
# ─────────────────────────────────────────────────────────────────────────────

@dataclass
class CycleResult:
    """Telemetry collected from one QEMU boot cycle."""

    cycle: int
    start_time: float = 0.0
    end_time: float = 0.0
    exit_code: int | None = None
    log_path: Path = Path()

    # Counters
    total_lines: int = 0
    nexus_lines: int = 0
    core_lines: int = 0
    panic_lines: int = 0
    heal_events: int = 0
    predict_events: int = 0
    sched_events: int = 0
    mem_events: int = 0

    # Critical failure
    crashed: bool = False
    crash_signature: str = ""

    # Raw captured lines of interest
    highlights: list[str] = field(default_factory=list)

    @property
    def duration_ms(self) -> float:
        return (self.end_time - self.start_time) * 1000.0

    @property
    def boot_ok(self) -> bool:
        return not self.crashed and self.exit_code is not None


# ─────────────────────────────────────────────────────────────────────────────
# Build
# ─────────────────────────────────────────────────────────────────────────────

def build_kernel(features: list[str], release: bool = True) -> bool:
    """Run cargo build for the minimal profile."""

    banner("BUILD")

    cmd: list[str] = [
        "cargo", "build",
        "-p", PACKAGE,
        "--target", TARGET,
    ]
    if release:
        cmd.append("--release")
    if features:
        cmd.extend(["--features", ",".join(features)])

    info(f"Command: {' '.join(cmd)}")
    info(f"Working directory: {HELIX_ROOT}")

    result = subprocess.run(
        cmd,
        cwd=HELIX_ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )

    # Dump compiler output
    if result.stdout:
        for line in result.stdout.splitlines():
            if "error" in line.lower():
                err(line)
            elif "warning" in line.lower():
                warn(line)
            else:
                dim(line)

    if result.returncode != 0:
        err(f"Build failed (exit {result.returncode})")
        return False

    # Copy binary to output dir
    profile_dir = "release" if release else "debug"
    built_bin = HELIX_ROOT / "target" / TARGET / profile_dir / PACKAGE
    if built_bin.exists():
        OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
        subprocess.run(["cp", str(built_bin), str(KERNEL_BIN)], check=True)
        size_kb = built_bin.stat().st_size / 1024
        ok(f"Kernel binary: {KERNEL_BIN}  ({size_kb:.1f} KB)")
    else:
        err(f"Expected binary not found: {built_bin}")
        return False

    return True


# ─────────────────────────────────────────────────────────────────────────────
# QEMU
# ─────────────────────────────────────────────────────────────────────────────

def build_qemu_cmd(memory: str = "256M", cpus: int = 1) -> list[str]:
    """Construct the QEMU command line."""

    cmd = [
        "qemu-system-x86_64",
        "-machine", "q35",
        "-m", memory,
        "-smp", str(cpus),
        "-display", "none",

        # ── Telemetry channels ──
        "-serial", "stdio",

        # ── Debug logging ──
        "-d", "guest_errors,int",

        # ── Stability: no auto-reboot on triple fault ──
        "-no-reboot",
        "-no-shutdown",

        # ── Boot ──
        "-kernel", str(KERNEL_BIN),

        # ── Devices ──
        "-device", "virtio-serial-pci",
        "-debugcon", f"file:{LOG_DIR / 'debug_port.log'}",
        "-global", "isa-debugcon.iobase=0x402",
    ]

    # KVM acceleration (optional, best-effort)
    if Path("/dev/kvm").exists():
        cmd.extend(["-enable-kvm"])

    return cmd


def run_qemu_cycle(
    cycle: int,
    timeout: float,
    memory: str,
    cpus: int,
) -> CycleResult:
    """Execute a single QEMU boot cycle, capture all output."""

    result = CycleResult(cycle=cycle)
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
    log_file = LOG_DIR / f"cycle_{cycle:03d}_{timestamp}.log"
    result.log_path = log_file

    qemu_cmd = build_qemu_cmd(memory=memory, cpus=cpus)

    banner(f"CYCLE {cycle}")
    info(f"Timeout : {timeout}s")
    info(f"Log     : {log_file}")
    info(f"Command : {' '.join(qemu_cmd)}")
    sep()

    result.start_time = time.monotonic()

    try:
        proc = subprocess.Popen(
            qemu_cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            cwd=HELIX_ROOT,
            preexec_fn=os.setsid,
        )
    except FileNotFoundError:
        err("qemu-system-x86_64 not found. Install QEMU first.")
        result.crashed = True
        result.crash_signature = "QEMU_NOT_FOUND"
        result.end_time = time.monotonic()
        return result

    lines_buf: list[str] = []

    try:
        deadline = time.monotonic() + timeout

        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                info(f"Timeout reached ({timeout}s). Stopping QEMU.")
                break

            # Non-blocking read via select-like approach
            try:
                # Use a short timeout poll
                import select
                rlist, _, _ = select.select([proc.stdout], [], [], 0.1)
                if not rlist:
                    # Check if process exited
                    if proc.poll() is not None:
                        # Drain remaining output
                        remaining_output = proc.stdout.read()
                        if remaining_output:
                            for line in remaining_output.splitlines():
                                lines_buf.append(line)
                                _classify_line(line, result)
                        break
                    continue
            except (ValueError, OSError):
                break

            line = proc.stdout.readline()
            if not line:
                if proc.poll() is not None:
                    break
                continue

            line = line.rstrip("\n")
            lines_buf.append(line)
            result.total_lines += 1

            _classify_line(line, result)

            # Check for critical crash
            for pat in CRITICAL_PATTERNS:
                if pat.search(line):
                    result.crashed = True
                    result.crash_signature = line.strip()[:120]

    except KeyboardInterrupt:
        warn("Interrupted by user (Ctrl+C)")
    finally:
        result.end_time = time.monotonic()
        _kill_process_group(proc)

    result.exit_code = proc.returncode

    # Write raw log
    with open(log_file, "w") as f:
        f.write(f"# Helix NEXUS First-Contact — Cycle {cycle}\n")
        f.write(f"# Timestamp: {timestamp}\n")
        f.write(f"# Duration:  {result.duration_ms:.1f} ms\n")
        f.write(f"# Exit code: {result.exit_code}\n")
        f.write(f"# Crashed:   {result.crashed}\n")
        f.write(f"# {'=' * 72}\n\n")
        for raw_line in lines_buf:
            f.write(raw_line + "\n")

    return result


def _classify_line(line: str, result: CycleResult) -> None:
    """Classify a single output line and update counters."""

    for tag, colour, pattern in TELEMETRY_PATTERNS:
        if pattern.search(line):
            # Print highlighted
            sys.stdout.write(f"  {colour}{C_BOLD}[{tag:>7}]{C_RESET} {colour}{line}{C_RESET}\n")
            sys.stdout.flush()

            # Update counters
            if tag == "NEXUS":
                result.nexus_lines += 1
            elif tag == "CORE":
                result.core_lines += 1
            elif tag == "PANIC":
                result.panic_lines += 1
            elif tag == "HEAL":
                result.heal_events += 1
            elif tag == "PREDICT":
                result.predict_events += 1
            elif tag == "SCHED":
                result.sched_events += 1
            elif tag == "MEM":
                result.mem_events += 1

            result.highlights.append(f"[{tag}] {line}")
            return  # First match wins

    # Non-matching lines: print dimmed
    sys.stdout.write(f"  {C_DIM}{line}{C_RESET}\n")
    sys.stdout.flush()


def _kill_process_group(proc: subprocess.Popen) -> None:
    """Kill QEMU and any child processes."""
    try:
        os.killpg(os.getpgid(proc.pid), signal.SIGTERM)
    except (ProcessLookupError, PermissionError):
        pass
    try:
        proc.wait(timeout=3)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
        except (ProcessLookupError, PermissionError):
            pass


# ─────────────────────────────────────────────────────────────────────────────
# Summary report
# ─────────────────────────────────────────────────────────────────────────────

def print_summary(results: list[CycleResult]) -> None:
    """Print a consolidated summary across all cycles."""

    banner("TELEMETRY SUMMARY")

    header = (
        f"{'Cycle':>5}  {'Duration':>10}  {'Exit':>4}  "
        f"{'NEXUS':>5}  {'CORE':>4}  {'PANIC':>5}  "
        f"{'HEAL':>4}  {'PRED':>4}  {'SCHED':>5}  {'MEM':>3}  "
        f"{'Status':>8}"
    )
    sys.stdout.write(f"\n{C_BOLD}{header}{C_RESET}\n")
    sys.stdout.write(f"{'─' * len(header)}\n")

    for r in results:
        status = f"{C_RED}CRASH{C_RESET}" if r.crashed else f"{C_GREEN}OK{C_RESET}"
        sys.stdout.write(
            f"{r.cycle:>5}  "
            f"{r.duration_ms:>8.1f}ms  "
            f"{str(r.exit_code or '?'):>4}  "
            f"{r.nexus_lines:>5}  "
            f"{r.core_lines:>4}  "
            f"{r.panic_lines:>5}  "
            f"{r.heal_events:>4}  "
            f"{r.predict_events:>4}  "
            f"{r.sched_events:>5}  "
            f"{r.mem_events:>3}  "
            f"{status:>8}\n"
        )

    # Aggregate
    total_nexus = sum(r.nexus_lines for r in results)
    total_panics = sum(r.panic_lines for r in results)
    total_heals = sum(r.heal_events for r in results)
    total_preds = sum(r.predict_events for r in results)
    crashes = sum(1 for r in results if r.crashed)

    sys.stdout.write(f"\n{'─' * len(header)}\n")
    sys.stdout.write(f"{C_BOLD}Cycles: {len(results)}   "
                     f"Crashes: {crashes}   "
                     f"NEXUS signals: {total_nexus}   "
                     f"Panics: {total_panics}   "
                     f"Heal events: {total_heals}   "
                     f"Predictions: {total_preds}{C_RESET}\n\n")

    # Highlight reel
    all_highlights: list[str] = []
    for r in results:
        all_highlights.extend(r.highlights)

    if all_highlights:
        banner("HIGHLIGHT REEL (filtered telemetry)")
        for h in all_highlights[:60]:
            sys.stdout.write(f"  {C_CYAN}▸{C_RESET} {h}\n")
        if len(all_highlights) > 60:
            sys.stdout.write(f"  {C_DIM}… and {len(all_highlights) - 60} more{C_RESET}\n")
    else:
        warn("No NEXUS/CORE telemetry lines captured. "
             "The kernel may not be reaching the sandbox code.")

    # Log paths
    sys.stdout.write(f"\n{C_BOLD}Raw logs:{C_RESET}\n")
    for r in results:
        sys.stdout.write(f"  {C_DIM}Cycle {r.cycle}: {r.log_path}{C_RESET}\n")
    sys.stdout.write("\n")


# ─────────────────────────────────────────────────────────────────────────────
# Pretty-print helpers
# ─────────────────────────────────────────────────────────────────────────────

def banner(text: str) -> None:
    sys.stdout.write(f"\n{C_BOLD}{C_CYAN}{'═' * 72}{C_RESET}\n")
    sys.stdout.write(f"{C_BOLD}{C_CYAN}  {text}{C_RESET}\n")
    sys.stdout.write(f"{C_BOLD}{C_CYAN}{'═' * 72}{C_RESET}\n\n")

def sep() -> None:
    sys.stdout.write(f"{C_DIM}{'─' * 72}{C_RESET}\n")

def info(msg: str) -> None:
    sys.stdout.write(f"  {C_BLUE}ℹ{C_RESET}  {msg}\n")

def ok(msg: str) -> None:
    sys.stdout.write(f"  {C_GREEN}✓{C_RESET}  {msg}\n")

def warn(msg: str) -> None:
    sys.stdout.write(f"  {C_YELLOW}⚠{C_RESET}  {C_YELLOW}{msg}{C_RESET}\n")

def err(msg: str) -> None:
    sys.stdout.write(f"  {C_RED}✗{C_RESET}  {C_RED}{msg}{C_RESET}\n")

def dim(msg: str) -> None:
    sys.stdout.write(f"  {C_DIM}{msg}{C_RESET}\n")


# ─────────────────────────────────────────────────────────────────────────────
# CLI
# ─────────────────────────────────────────────────────────────────────────────

def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(
        description="NEXUS First-Contact stability harness for Helix OS",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s                         # Single cycle, 15s timeout
  %(prog)s --cycles 5 --timeout 30 # 5 cycles, 30s each
  %(prog)s --features nexus-lite   # Enable nexus-lite feature
  %(prog)s --skip-build            # Skip cargo build, use existing binary
  %(prog)s --release               # Release build (default)
  %(prog)s --debug-build           # Debug build (unoptimised)
""",
    )
    p.add_argument("--cycles", type=int, default=1,
                   help="Number of QEMU boot cycles (default: 1)")
    p.add_argument("--timeout", type=float, default=15.0,
                   help="Max seconds per cycle before SIGTERM (default: 15)")
    p.add_argument("--features", type=str, default="",
                   help="Comma-separated cargo features (e.g. nexus-lite)")
    p.add_argument("--skip-build", action="store_true",
                   help="Skip the cargo build step")
    p.add_argument("--debug-build", action="store_true",
                   help="Build with dev profile instead of release")
    p.add_argument("--memory", type=str, default="256M",
                   help="QEMU RAM (default: 256M)")
    p.add_argument("--cpus", type=int, default=1,
                   help="QEMU vCPU count (default: 1)")
    return p.parse_args()


# ─────────────────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────────────────

def main() -> int:
    args = parse_args()

    banner("HELIX OS — NEXUS \"First Contact\" Stability Harness")
    info(f"Date    : {datetime.now(timezone.utc).isoformat()}")
    info(f"Root    : {HELIX_ROOT}")
    info(f"Cycles  : {args.cycles}")
    info(f"Timeout : {args.timeout}s per cycle")
    info(f"Memory  : {args.memory}")
    info(f"CPUs    : {args.cpus}")

    features = [f.strip() for f in args.features.split(",") if f.strip()]
    if features:
        info(f"Features: {', '.join(features)}")

    # Ensure log directory
    LOG_DIR.mkdir(parents=True, exist_ok=True)

    # ── Phase 1: Build ──
    if not args.skip_build:
        release = not args.debug_build
        if not build_kernel(features=features, release=release):
            err("Cannot proceed without a kernel binary.")
            return 1
    else:
        if not KERNEL_BIN.exists():
            err(f"Kernel binary not found: {KERNEL_BIN}")
            err("Run without --skip-build first.")
            return 1
        info(f"Skipping build. Using existing: {KERNEL_BIN}")

    # ── Phase 2: QEMU cycles ──
    results: list[CycleResult] = []

    for i in range(1, args.cycles + 1):
        r = run_qemu_cycle(
            cycle=i,
            timeout=args.timeout,
            memory=args.memory,
            cpus=args.cpus,
        )
        results.append(r)

        if r.crashed:
            warn(f"Cycle {i} crashed: {r.crash_signature}")
        else:
            ok(f"Cycle {i} completed ({r.duration_ms:.1f} ms, "
               f"exit={r.exit_code}, "
               f"nexus={r.nexus_lines}, panic={r.panic_lines})")

    # ── Phase 3: Summary ──
    print_summary(results)

    # Exit code: non-zero if any cycle crashed
    if any(r.crashed for r in results):
        err("One or more cycles crashed. Review logs above.")
        return 1

    ok("All cycles completed without critical failures.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
