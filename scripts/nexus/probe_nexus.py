#!/usr/bin/env python3
"""
╔══════════════════════════════════════════════════════════════════════════════╗
║                                                                            ║
║   HELIX OS — NEXUS PROBE                                                   ║
║   Continuous Autonomy Verification Harness                                 ║
║                                                                            ║
║   This is NOT a test runner. This is a probe.                              ║
║   It boots the kernel, injects chaos, and listens for evidence             ║
║   that the NEXUS subsystem is making autonomous decisions.                 ║
║                                                                            ║
║   Architecture:                                                            ║
║                                                                            ║
║     ┌──────────────────────────────────────────────────┐                   ║
║     │  QEMU  (helix-kernel)                            │                   ║
║     │  ┌──────────┐     ┌──────────────────────┐       │                   ║
║     │  │ -serial   │────▶ pty/pipe  ──▶ Reader  │       │                   ║
║     │  │ chardev   │     │  (kernel log parse)  │       │                   ║
║     │  └──────────┘     └──────────────────────┘       │                   ║
║     │  ┌──────────┐     ┌──────────────────────┐       │                   ║
║     │  │ -monitor  │◀───│ unix socket ◀─ Poker  │       │                   ║
║     │  │ HMP       │     │ (sendkey / nmi /     │       │                   ║
║     │  │           │     │  inject-nmi / IRQ)   │       │                   ║
║     │  └──────────┘     └──────────────────────┘       │                   ║
║     └──────────────────────────────────────────────────┘                   ║
║                      │                                                     ║
║                      ▼                                                     ║
║            ┌──────────────────┐                                            ║
║            │   Dashboard      │                                            ║
║            │   (live tally)   │                                            ║
║            └──────────────────┘                                            ║
║                                                                            ║
║   Usage:                                                                   ║
║     python3 scripts/nexus/probe_nexus.py                                   ║
║     python3 scripts/nexus/probe_nexus.py --poke-interval 3                 ║
║     python3 scripts/nexus/probe_nexus.py --skip-build --cycle-timeout 30   ║
║                                                                            ║
║   Stop:  Ctrl+C  (graceful shutdown, prints final report)                  ║
║                                                                            ║
╚══════════════════════════════════════════════════════════════════════════════╝
"""

from __future__ import annotations

import argparse
import fcntl
import json
import os
import random
import re
import select
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time
import traceback
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path


# =============================================================================
# PATHS
# =============================================================================

HELIX_ROOT = Path(__file__).resolve().parent.parent.parent
BUILD_DIR  = HELIX_ROOT / "build"
LOG_DIR    = BUILD_DIR / "logs" / "nexus"
OUTPUT_DIR = BUILD_DIR / "output"
KERNEL_BIN = OUTPUT_DIR / "helix-kernel"

TARGET  = "x86_64-unknown-none"
PACKAGE = "helix-minimal-os"


# =============================================================================
# ANSI PALETTE
# =============================================================================

RST     = "\033[0m"
BOLD    = "\033[1m"
DIM     = "\033[2m"
BLINK   = "\033[5m"
RED     = "\033[31m"
GREEN   = "\033[32m"
YELLOW  = "\033[33m"
BLUE    = "\033[34m"
MAGENTA = "\033[35m"
CYAN    = "\033[36m"
WHITE   = "\033[37m"
BG_RED  = "\033[41m"
BG_GRN  = "\033[42m"
BG_BLU  = "\033[44m"
BG_CYN  = "\033[46m"

# Cursor control for dashboard redraw
CLEAR_LINE = "\033[2K"
CURSOR_UP  = "\033[A"
SAVE_CUR   = "\033[s"
REST_CUR   = "\033[u"


# =============================================================================
# TELEMETRY — what counts as "Proof of Life"
# =============================================================================

# Each tuple: (tag, colour, regex, is_decision)
# is_decision = True means this line is evidence of NEXUS autonomy.

SIGNAL_PATTERNS: list[tuple[str, str, re.Pattern[str], bool]] = [
    # ── Category A: Adaptation ────────────────────────────────────────────
    ("ADAPT",   CYAN,    re.compile(
        r"\[NEXUS\].*(?:adjust|adapt|rebalanc|optimi[sz]|quantum|heuristic"
        r"|schedul|timeslice|load\s*>|threshold|scaling)",
        re.IGNORECASE), True),

    # ── Category B: Healing ───────────────────────────────────────────────
    ("HEAL",    GREEN,   re.compile(
        r"\[NEXUS\].*(?:heal|recover|rollback|quarantine|reclaim|self.heal"
        r"|auto.recover|stalled|watchdog|restart)",
        re.IGNORECASE), True),

    # ── Category C: Metacognition ─────────────────────────────────────────
    ("META",    MAGENTA, re.compile(
        r"\[NEXUS\].*(?:deviation|drift|boot.time|introspect|reflect"
        r"|metacog|confidence|predict|forecast|anomal|canary)",
        re.IGNORECASE), True),

    # ── Generic NEXUS output (not necessarily a decision) ─────────────────
    ("NEXUS",   CYAN,    re.compile(r"\[NEXUS\]",   re.IGNORECASE), False),
    ("nexus",   CYAN,    re.compile(r"\[nexus-lite\]", re.IGNORECASE), False),
    ("CORE",    BLUE,    re.compile(r"\[CORE\]",    re.IGNORECASE), False),
    ("BOOT",    YELLOW,  re.compile(r"\[BOOT\]",    re.IGNORECASE), False),
    ("SCHED",   GREEN,   re.compile(r"\[SCHED\]",   re.IGNORECASE), False),
    ("DEMO",    WHITE,   re.compile(r"\[DEMO\]",    re.IGNORECASE), False),

    # ── Crash signatures ──────────────────────────────────────────────────
    ("PANIC",   RED,     re.compile(
        r"panic|PANIC|triple.fault|page.fault|EXCEPTION|double.fault"
        r"|general.protection|stack.fault|invalid.opcode",
        re.IGNORECASE), False),
    ("CRASH",   RED,     re.compile(
        r"Shutting.down|QEMU.*terminated|KVM.*error|CPU.halted",
        re.IGNORECASE), False),

    # ── Memory subsystem ──────────────────────────────────────────────────
    ("MEM",     YELLOW,  re.compile(
        r"heap|alloc|page.table|memory.map|OOM|frame",
        re.IGNORECASE), False),
]

# Patterns that mean "QEMU just died, restart immediately"
DEATH_SIGNATURES: list[re.Pattern[str]] = [
    re.compile(r"triple.fault",       re.IGNORECASE),
    re.compile(r"Shutting.down",      re.IGNORECASE),
    re.compile(r"KVM internal error", re.IGNORECASE),
    re.compile(r"CPU halted",         re.IGNORECASE),
]


# =============================================================================
# STIMULI — what we inject to provoke NEXUS
# =============================================================================

# QEMU Human Monitor Protocol commands.
# We send these over a UNIX socket connected to `-monitor unix:...`
STIMULI_POOL: list[tuple[str, str]] = [
    # ── Keystrokes: simulate user input, generate IRQ1 (keyboard) ─────
    ("sendkey a",               "keyboard IRQ — key 'a'"),
    ("sendkey b",               "keyboard IRQ — key 'b'"),
    ("sendkey ret",             "keyboard IRQ — Enter"),
    ("sendkey spc",             "keyboard IRQ — Space"),
    ("sendkey 1",               "keyboard IRQ — key '1'"),
    ("sendkey esc",             "keyboard IRQ — Escape"),
    ("sendkey tab",             "keyboard IRQ — Tab"),

    # ── NMI: non-maskable interrupt, forces exception handling path ───
    ("nmi 0",                   "NMI → CPU 0"),

    # ── Info queries: harmless but exercise the monitor path ──────────
    ("info registers",          "register dump"),
    ("info cpus",               "CPU state query"),
    ("info mtree",              "memory topology dump"),
]


# =============================================================================
# DATA MODEL
# =============================================================================

@dataclass
class ProbeState:
    """Global mutable state for the infinite probe loop."""

    # ── Lifetime counters ─────────────────────────────────────────────
    cycles_total: int     = 0
    crashes_total: int    = 0
    lines_total: int      = 0
    stimuli_sent: int     = 0

    # ── NEXUS-specific ────────────────────────────────────────────────
    decisions_detected: int = 0     # lines matching ADAPT/HEAL/META
    nexus_signals: int      = 0    # any [NEXUS] line
    boot_signals: int       = 0
    panic_signals: int      = 0
    mem_signals: int        = 0
    sched_signals: int      = 0

    # ── Current cycle ─────────────────────────────────────────────────
    current_cycle: int      = 0
    cycle_start: float      = 0.0
    cycle_lines: int        = 0
    cycle_decisions: int    = 0

    # ── Recent history (ring buffer of last N interesting lines) ──────
    last_thought: str       = "(none yet)"
    recent_signals: list[str] = field(default_factory=list)
    MAX_RECENT: int         = 50

    # ── Timing ────────────────────────────────────────────────────────
    probe_start: float      = 0.0
    last_poke_time: float   = 0.0

    # ── Log file for current cycle ────────────────────────────────────
    cycle_log: Path | None  = None
    cycle_log_fd: object    = None   # open file handle

    # ── Control ───────────────────────────────────────────────────────
    shutdown_requested: bool = False

    def add_signal(self, tag: str, line: str) -> None:
        entry = f"[{tag:>6}] {line.rstrip()}"
        self.recent_signals.append(entry)
        if len(self.recent_signals) > self.MAX_RECENT:
            self.recent_signals.pop(0)

    @property
    def uptime_s(self) -> float:
        return time.monotonic() - self.probe_start if self.probe_start else 0.0

    @property
    def cycle_duration_ms(self) -> float:
        return (time.monotonic() - self.cycle_start) * 1000.0 if self.cycle_start else 0.0


# =============================================================================
# BUILD
# =============================================================================

def build_kernel(features: list[str], release: bool = True) -> bool:
    """Compile helix-minimal-os for the bare-metal target."""

    hdr("BUILD")

    cmd: list[str] = [
        "cargo", "build",
        "-p", PACKAGE,
        "--target", TARGET,
    ]
    if release:
        cmd.append("--release")
    if features:
        cmd.extend(["--features", ",".join(features)])

    info(f"$ {' '.join(cmd)}")

    result = subprocess.run(
        cmd,
        cwd=HELIX_ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )

    for line in (result.stdout or "").splitlines():
        lo = line.lower()
        if "error" in lo:
            err(line)
        elif "warning" in lo:
            warn(line)
        else:
            dim(line)

    if result.returncode != 0:
        err(f"Build failed (exit {result.returncode})")
        return False

    profile_dir = "release" if release else "debug"
    src = HELIX_ROOT / "target" / TARGET / profile_dir / PACKAGE
    if src.exists():
        OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
        subprocess.run(["cp", str(src), str(KERNEL_BIN)], check=True)
        kb = src.stat().st_size / 1024
        ok(f"Kernel: {KERNEL_BIN}  ({kb:.1f} KB)")
        return True

    err(f"Binary not found: {src}")
    return False


# =============================================================================
# QEMU PROCESS MANAGEMENT
# =============================================================================

def launch_qemu(
    state: ProbeState,
    monitor_sock_path: str,
    memory: str = "256M",
    cpus: int = 1,
) -> subprocess.Popen | None:
    """
    Launch one QEMU instance.

    Serial output → stdout pipe  (we read it line-by-line)
    Monitor       → UNIX socket  (we poke it for stimulus injection)
    """

    LOG_DIR.mkdir(parents=True, exist_ok=True)

    # Debug log file for QEMU's own -d output
    debug_log = LOG_DIR / "debug_port.log"

    cmd: list[str] = [
        "qemu-system-x86_64",
        "-machine",  "q35",
        "-m",        memory,
        "-smp",      str(cpus),
        "-display",  "none",

        # Serial → our stdout pipe (kernel log output)
        "-serial",   "stdio",

        # Monitor → UNIX socket (stimulus injection channel)
        "-monitor",  f"unix:{monitor_sock_path},server,nowait",

        # QEMU guest error + interrupt tracing → stderr (merged with stdout)
        "-d",        "guest_errors,int",

        # Do NOT auto-reboot on triple fault — let us see the crash
        "-no-reboot",
        "-no-shutdown",

        # Boot the kernel ELF directly (multiboot2)
        "-kernel",   str(KERNEL_BIN),

        # Extra devices
        "-device",   "virtio-serial-pci",
        "-debugcon",  f"file:{debug_log}",
        "-global",   "isa-debugcon.iobase=0x402",
    ]

    # KVM (best-effort)
    if Path("/dev/kvm").exists():
        cmd.extend(["-enable-kvm"])

    try:
        proc = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            cwd=HELIX_ROOT,
            preexec_fn=os.setsid,     # own process group for clean kill
        )
    except FileNotFoundError:
        err("qemu-system-x86_64 not found.  Install QEMU.")
        return None

    # Make stdout non-blocking so our read loop doesn't hang
    fd = proc.stdout.fileno()
    fl = fcntl.fcntl(fd, fcntl.F_GETFL)
    fcntl.fcntl(fd, fcntl.F_SETFL, fl | os.O_NONBLOCK)

    return proc


def kill_qemu(proc: subprocess.Popen | None) -> None:
    """Hard-kill QEMU and its process group."""
    if proc is None:
        return
    try:
        os.killpg(os.getpgid(proc.pid), signal.SIGTERM)
    except (ProcessLookupError, PermissionError, OSError):
        pass
    try:
        proc.wait(timeout=3)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
        except (ProcessLookupError, PermissionError, OSError):
            pass
        try:
            proc.wait(timeout=2)
        except subprocess.TimeoutExpired:
            pass


# =============================================================================
# MONITOR SOCKET — THE POKER
# =============================================================================

class MonitorSocket:
    """
    Manages a connection to the QEMU Human Monitor Protocol (HMP)
    over a UNIX domain socket.

    Used to inject stimuli: sendkey, nmi, info queries.
    """

    def __init__(self, path: str):
        self._path = path
        self._sock: socket.socket | None = None
        self._connected = False

    def connect(self, retries: int = 10, delay: float = 0.3) -> bool:
        """Try to connect to the monitor socket (QEMU creates it on startup)."""
        for attempt in range(retries):
            try:
                s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                s.settimeout(2.0)
                s.connect(self._path)
                # Drain the QEMU greeting banner
                try:
                    s.recv(4096)
                except socket.timeout:
                    pass
                self._sock = s
                self._connected = True
                return True
            except (ConnectionRefusedError, FileNotFoundError, OSError):
                time.sleep(delay)
        return False

    def send(self, command: str) -> str | None:
        """Send an HMP command and return the response (best-effort)."""
        if not self._connected or self._sock is None:
            return None
        try:
            self._sock.sendall((command + "\n").encode())
            time.sleep(0.05)
            try:
                return self._sock.recv(8192).decode(errors="replace")
            except socket.timeout:
                return "(timeout)"
        except (BrokenPipeError, ConnectionResetError, OSError):
            self._connected = False
            return None

    def close(self) -> None:
        if self._sock:
            try:
                self._sock.close()
            except OSError:
                pass
            self._sock = None
            self._connected = False

    @property
    def alive(self) -> bool:
        return self._connected


# =============================================================================
# LINE CLASSIFIER
# =============================================================================

def classify_line(line: str, state: ProbeState) -> None:
    """
    Match a single serial output line against all signal patterns.
    Update counters, record decisions, write to cycle log.
    """

    state.lines_total  += 1
    state.cycle_lines  += 1
    stripped = line.rstrip()

    # Write raw line to cycle log
    if state.cycle_log_fd:
        state.cycle_log_fd.write(stripped + "\n")
        state.cycle_log_fd.flush()

    matched = False

    for tag, colour, pattern, is_decision in SIGNAL_PATTERNS:
        if pattern.search(stripped):
            matched = True

            # ── Counter updates ───────────────────────────────────────
            if is_decision:
                state.decisions_detected += 1
                state.cycle_decisions    += 1
                state.last_thought = stripped
            if tag in ("NEXUS", "nexus"):
                state.nexus_signals += 1
            elif tag == "BOOT":
                state.boot_signals += 1
            elif tag in ("PANIC", "CRASH"):
                state.panic_signals += 1
            elif tag == "MEM":
                state.mem_signals += 1
            elif tag == "SCHED":
                state.sched_signals += 1

            state.add_signal(tag, stripped)

            # ── Coloured console output ───────────────────────────────
            tag_str = f"{colour}{BOLD}[{tag:>6}]{RST}"
            sys.stdout.write(f"    {tag_str} {colour}{stripped}{RST}\n")
            sys.stdout.flush()
            return

    # Unclassified line — dim output
    if stripped:
        sys.stdout.write(f"    {DIM}{stripped}{RST}\n")
        sys.stdout.flush()


def is_death(line: str) -> bool:
    """Return True if this line signals QEMU / kernel death."""
    for pat in DEATH_SIGNATURES:
        if pat.search(line):
            return True
    return False


# =============================================================================
# DASHBOARD — live status display
# =============================================================================

DASHBOARD_LINES = 16  # how many lines the dashboard occupies

def draw_dashboard(state: ProbeState, *, first: bool = False) -> None:
    """
    Print / overwrite a compact status panel above the log stream.
    """

    uptime = state.uptime_s
    hours   = int(uptime // 3600)
    minutes = int((uptime % 3600) // 60)
    seconds = int(uptime % 60)

    # Decision rate
    rate = (state.decisions_detected / uptime * 60) if uptime > 1 else 0.0

    # Status colour
    if state.decisions_detected > 0:
        status_colour = GREEN
        status_word   = "CONTACT"
        status_icon   = "◉"
    elif state.nexus_signals > 0:
        status_colour = YELLOW
        status_word   = "SIGNAL"
        status_icon   = "◎"
    else:
        status_colour = RED
        status_word   = "SILENT"
        status_icon   = "○"

    thought_display = state.last_thought[:72] if len(state.last_thought) > 72 else state.last_thought

    lines = [
        "",
        f"  {BOLD}{CYAN}╔══════════════════════════════════════════════════════════════════════╗{RST}",
        f"  {BOLD}{CYAN}║{RST}  {BOLD}NEXUS PROBE — LIVE DASHBOARD{RST}"
        f"                    {DIM}Uptime: {hours:02d}:{minutes:02d}:{seconds:02d}{RST}  {BOLD}{CYAN}║{RST}",
        f"  {BOLD}{CYAN}╠══════════════════════════════════════════════════════════════════════╣{RST}",
        f"  {BOLD}{CYAN}║{RST}                                                                    {BOLD}{CYAN}║{RST}",
        f"  {BOLD}{CYAN}║{RST}  Cycles Run .......... {BOLD}{state.cycles_total:<6}{RST}"
        f"   Crashes ........... {RED}{BOLD}{state.crashes_total:<6}{RST}         {BOLD}{CYAN}║{RST}",
        f"  {BOLD}{CYAN}║{RST}  Total Lines ......... {state.lines_total:<6}"
        f"   Stimuli Injected .. {YELLOW}{state.stimuli_sent:<6}{RST}         {BOLD}{CYAN}║{RST}",
        f"  {BOLD}{CYAN}║{RST}                                                                    {BOLD}{CYAN}║{RST}",
        f"  {BOLD}{CYAN}║{RST}  {status_colour}{BOLD}{status_icon} NEXUS DECISIONS DETECTED: "
        f"{state.decisions_detected:<6}{RST}"
        f"  {DIM}({rate:.1f}/min){RST}                 {BOLD}{CYAN}║{RST}",
        f"  {BOLD}{CYAN}║{RST}    NEXUS signals ..... {state.nexus_signals:<6}"
        f"   Panic signals ..... {state.panic_signals:<6}         {BOLD}{CYAN}║{RST}",
        f"  {BOLD}{CYAN}║{RST}    Scheduler signals . {state.sched_signals:<6}"
        f"   Memory signals .... {state.mem_signals:<6}         {BOLD}{CYAN}║{RST}",
        f"  {BOLD}{CYAN}║{RST}                                                                    {BOLD}{CYAN}║{RST}",
        f"  {BOLD}{CYAN}║{RST}  {BOLD}Status:{RST}  {status_colour}{BOLD}{status_word}{RST}"
        f"                                                   {BOLD}{CYAN}║{RST}",
        f"  {BOLD}{CYAN}║{RST}  {BOLD}Last Thought:{RST}  {MAGENTA}{thought_display}{RST}",
        f"  {BOLD}{CYAN}║{RST}                                                                    {BOLD}{CYAN}║{RST}",
        f"  {BOLD}{CYAN}╚══════════════════════════════════════════════════════════════════════╝{RST}",
    ]

    # Move cursor up to overwrite previous dashboard (unless first draw)
    if not first:
        sys.stdout.write(f"\033[{DASHBOARD_LINES}A")

    for l in lines:
        sys.stdout.write(CLEAR_LINE + l + "\n")
    sys.stdout.flush()


# =============================================================================
# THE CORE LOOP
# =============================================================================

def run_probe(
    state: ProbeState,
    *,
    cycle_timeout: float,
    poke_interval: float,
    poke_burst: int,
    memory: str,
    cpus: int,
    dashboard_interval: float,
) -> None:
    """
    Infinite loop:
      1. Launch QEMU
      2. Read serial output, classify every line
      3. Periodically poke via monitor socket
      4. On crash or timeout → log, increment counters, restart
      5. Redraw dashboard periodically

    Exits only on Ctrl+C (sets state.shutdown_requested).
    """

    # Temporary directory for the monitor socket
    sock_dir  = tempfile.mkdtemp(prefix="helix_probe_")
    sock_path = os.path.join(sock_dir, "monitor.sock")

    first_dashboard = True

    while not state.shutdown_requested:
        # ── Start a new cycle ─────────────────────────────────────────
        state.cycles_total += 1
        state.current_cycle = state.cycles_total
        state.cycle_start   = time.monotonic()
        state.cycle_lines   = 0
        state.cycle_decisions = 0

        ts = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
        log_path = LOG_DIR / f"probe_cycle_{state.cycles_total:05d}_{ts}.log"
        state.cycle_log = log_path

        try:
            state.cycle_log_fd = open(log_path, "w")
            state.cycle_log_fd.write(f"# Probe Cycle {state.cycles_total}\n")
            state.cycle_log_fd.write(f"# Started: {ts}\n")
            state.cycle_log_fd.write(f"# {'=' * 60}\n\n")
        except OSError:
            state.cycle_log_fd = None

        # Remove stale socket
        try:
            os.unlink(sock_path)
        except FileNotFoundError:
            pass

        hdr(f"CYCLE {state.cycles_total}")
        info(f"Log: {log_path}")

        # ── Launch QEMU ───────────────────────────────────────────────
        proc = launch_qemu(state, sock_path, memory=memory, cpus=cpus)
        if proc is None:
            err("Failed to launch QEMU. Waiting 3s before retry.")
            time.sleep(3)
            continue

        # ── Connect monitor socket ────────────────────────────────────
        monitor = MonitorSocket(sock_path)
        time.sleep(0.5)  # let QEMU initialise
        if monitor.connect():
            ok("Monitor socket connected — stimulus injection armed.")
        else:
            warn("Monitor socket failed to connect. Stimuli disabled for this cycle.")

        # ── Read / Poke / Dashboard loop ──────────────────────────────
        deadline           = time.monotonic() + cycle_timeout
        last_poke          = time.monotonic()
        last_dashboard     = time.monotonic()
        death_detected     = False
        remaining_buf      = ""

        try:
            while not state.shutdown_requested:
                now = time.monotonic()

                # ── Timeout check ─────────────────────────────────────
                if now >= deadline:
                    info(f"Cycle timeout ({cycle_timeout}s). Recycling.")
                    break

                # ── Process exit check ────────────────────────────────
                if proc.poll() is not None:
                    # Drain any remaining output
                    try:
                        leftover = proc.stdout.read()
                        if leftover:
                            for ln in leftover.splitlines():
                                classify_line(ln, state)
                                if is_death(ln):
                                    death_detected = True
                    except (OSError, ValueError):
                        pass
                    info(f"QEMU exited (code={proc.returncode}).")
                    if proc.returncode != 0:
                        death_detected = True
                    break

                # ── Read serial output (non-blocking) ─────────────────
                try:
                    rlist, _, _ = select.select([proc.stdout], [], [], 0.05)
                except (ValueError, OSError):
                    break

                if rlist:
                    try:
                        chunk = proc.stdout.read(65536)
                    except (OSError, ValueError):
                        chunk = None

                    if chunk:
                        remaining_buf += chunk
                        while "\n" in remaining_buf:
                            line, remaining_buf = remaining_buf.split("\n", 1)
                            classify_line(line, state)
                            if is_death(line):
                                death_detected = True

                if death_detected:
                    break

                # ── Stimulus injection ────────────────────────────────
                if monitor.alive and (now - last_poke) >= poke_interval:
                    last_poke = now
                    # Send a burst of random stimuli
                    for _ in range(poke_burst):
                        cmd_str, desc = random.choice(STIMULI_POOL)
                        resp = monitor.send(cmd_str)
                        state.stimuli_sent += 1
                        if state.cycle_log_fd:
                            state.cycle_log_fd.write(
                                f"# [POKE] {cmd_str}  ({desc})\n"
                            )

                # ── Dashboard refresh ─────────────────────────────────
                if (now - last_dashboard) >= dashboard_interval:
                    last_dashboard = now
                    draw_dashboard(state, first=first_dashboard)
                    first_dashboard = False

        except KeyboardInterrupt:
            state.shutdown_requested = True

        # ── Cycle cleanup ─────────────────────────────────────────────
        monitor.close()
        kill_qemu(proc)

        if death_detected:
            state.crashes_total += 1
            warn(f"Cycle {state.current_cycle} crashed.  "
                 f"Total crashes: {state.crashes_total}")
        else:
            ok(f"Cycle {state.current_cycle} completed.  "
               f"Lines={state.cycle_lines}  "
               f"Decisions={state.cycle_decisions}")

        # Close cycle log
        if state.cycle_log_fd:
            dur = state.cycle_duration_ms
            state.cycle_log_fd.write(f"\n# Duration: {dur:.1f} ms\n")
            state.cycle_log_fd.write(f"# Crashed: {death_detected}\n")
            state.cycle_log_fd.close()
            state.cycle_log_fd = None

        # Brief pause between cycles (let OS reclaim resources)
        if not state.shutdown_requested:
            time.sleep(1.0)

    # Clean up socket dir
    try:
        os.unlink(sock_path)
        os.rmdir(sock_dir)
    except OSError:
        pass


# =============================================================================
# FINAL REPORT
# =============================================================================

def print_final_report(state: ProbeState) -> None:
    """Print the comprehensive end-of-session report."""

    hdr("FINAL PROBE REPORT")

    uptime = state.uptime_s
    h = int(uptime // 3600)
    m = int((uptime % 3600) // 60)
    s = int(uptime % 60)

    rate = (state.decisions_detected / uptime * 60) if uptime > 1 else 0.0

    print(f"""
  {BOLD}Session Duration:{RST}       {h:02d}:{m:02d}:{s:02d}
  {BOLD}Cycles Completed:{RST}       {state.cycles_total}
  {BOLD}Total Crashes:{RST}          {RED}{state.crashes_total}{RST}
  {BOLD}Total Lines Parsed:{RST}     {state.lines_total}
  {BOLD}Stimuli Injected:{RST}       {state.stimuli_sent}

  {BOLD}{CYAN}═══════════════════════════════════════════{RST}
  {BOLD}{CYAN}  NEXUS AUTONOMY METRICS{RST}
  {BOLD}{CYAN}═══════════════════════════════════════════{RST}

  {BOLD}Decisions Detected:{RST}     {GREEN if state.decisions_detected > 0 else RED}{BOLD}{state.decisions_detected}{RST}
  {BOLD}Decision Rate:{RST}          {rate:.2f} / minute
  {BOLD}NEXUS Signals:{RST}          {state.nexus_signals}
  {BOLD}Panic Signals:{RST}          {state.panic_signals}
  {BOLD}Scheduler Signals:{RST}      {state.sched_signals}
  {BOLD}Memory Signals:{RST}         {state.mem_signals}
""")

    # Verdict
    if state.decisions_detected > 0:
        print(f"""
  {BG_GRN}{BOLD} VERDICT: CONTACT ESTABLISHED {RST}

  The probe detected {state.decisions_detected} autonomous decision(s) by the
  NEXUS subsystem.  The kernel is not merely executing static code —
  it is making runtime choices based on observed state.
""")
    elif state.nexus_signals > 0:
        print(f"""
  {BG_BLU}{BOLD} VERDICT: SIGNAL DETECTED — NO DECISIONS YET {RST}

  The NEXUS subsystem is emitting output ({state.nexus_signals} signals),
  but no adaptive/healing/metacognitive decisions were captured.
  The subsystem is running but may not yet be wired to a decision path.
""")
    else:
        print(f"""
  {BG_RED}{BOLD} VERDICT: SILENCE — NO NEXUS ACTIVITY {RST}

  No NEXUS-tagged output was detected in {state.cycles_total} cycle(s).
  The subsystem may not be compiled in, may not be reached during boot,
  or may be gated behind a feature flag that is not active.
""")

    # Recent signal history
    if state.recent_signals:
        hdr("RECENT SIGNAL HISTORY (last 50)")
        for entry in state.recent_signals:
            print(f"    {CYAN}▸{RST} {entry}")
        print()

    # Log directory
    print(f"  {BOLD}Log directory:{RST}  {LOG_DIR}")
    print()


# =============================================================================
# PRETTY PRINT HELPERS
# =============================================================================

def hdr(text: str) -> None:
    sys.stdout.write(f"\n{BOLD}{CYAN}{'═' * 72}{RST}\n")
    sys.stdout.write(f"{BOLD}{CYAN}  {text}{RST}\n")
    sys.stdout.write(f"{BOLD}{CYAN}{'═' * 72}{RST}\n\n")
    sys.stdout.flush()

def info(msg: str)  -> None:
    sys.stdout.write(f"  {BLUE}ℹ{RST}  {msg}\n"); sys.stdout.flush()

def ok(msg: str)    -> None:
    sys.stdout.write(f"  {GREEN}✓{RST}  {msg}\n"); sys.stdout.flush()

def warn(msg: str)  -> None:
    sys.stdout.write(f"  {YELLOW}⚠{RST}  {YELLOW}{msg}{RST}\n"); sys.stdout.flush()

def err(msg: str)   -> None:
    sys.stdout.write(f"  {RED}✗{RST}  {RED}{msg}{RST}\n"); sys.stdout.flush()

def dim(msg: str)   -> None:
    sys.stdout.write(f"  {DIM}{msg}{RST}\n"); sys.stdout.flush()


# =============================================================================
# CLI
# =============================================================================

def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(
        description="NEXUS Probe — Continuous Autonomy Verification for Helix OS",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=f"""
The probe runs indefinitely.  Press Ctrl+C to stop and print the final report.

Examples:
  %(prog)s                                     # defaults: 20s cycles, poke every 3s
  %(prog)s --cycle-timeout 60 --poke-interval 5 # longer cycles, slower pokes
  %(prog)s --features nexus-lite               # enable the nexus-lite sandbox
  %(prog)s --skip-build                        # reuse existing kernel binary
  %(prog)s --poke-burst 5                      # inject 5 stimuli per interval
  %(prog)s --no-poke                           # disable stimulus injection entirely

Logs are written to: {LOG_DIR}
""",
    )

    p.add_argument("--cycle-timeout", type=float, default=20.0,
                   help="Seconds per boot cycle before forced restart (default: 20)")
    p.add_argument("--poke-interval", type=float, default=3.0,
                   help="Seconds between stimulus injections (default: 3)")
    p.add_argument("--poke-burst", type=int, default=2,
                   help="Number of stimuli per injection interval (default: 2)")
    p.add_argument("--no-poke", action="store_true",
                   help="Disable stimulus injection entirely")
    p.add_argument("--features", type=str, default="",
                   help="Comma-separated cargo features (e.g. nexus-lite)")
    p.add_argument("--skip-build", action="store_true",
                   help="Skip cargo build, use existing binary")
    p.add_argument("--debug-build", action="store_true",
                   help="Use dev profile instead of release")
    p.add_argument("--memory", type=str, default="256M",
                   help="QEMU RAM (default: 256M)")
    p.add_argument("--cpus", type=int, default=1,
                   help="QEMU vCPU count (default: 1)")
    p.add_argument("--dashboard-interval", type=float, default=2.0,
                   help="Seconds between dashboard redraws (default: 2)")

    return p.parse_args()


# =============================================================================
# MAIN
# =============================================================================

def main() -> int:
    args = parse_args()

    state = ProbeState()
    state.probe_start = time.monotonic()

    # ── Signal handler for graceful Ctrl+C ────────────────────────────
    def handle_sigint(sig, frame):
        state.shutdown_requested = True
        # Don't raise — let the main loop exit cleanly

    signal.signal(signal.SIGINT, handle_sigint)

    # ── Banner ────────────────────────────────────────────────────────
    hdr("HELIX OS — NEXUS PROBE")
    print(f"""
    {BOLD}Continuous Autonomy Verification Harness{RST}
    {DIM}Searching for evidence of machine cognition.{RST}

    {BOLD}Configuration:{RST}
      Cycle timeout ........ {args.cycle_timeout}s
      Poke interval ........ {"DISABLED" if args.no_poke else f"{args.poke_interval}s  (burst={args.poke_burst})"}
      Memory ............... {args.memory}
      CPUs ................. {args.cpus}
      Dashboard refresh .... {args.dashboard_interval}s
""")

    features = [f.strip() for f in args.features.split(",") if f.strip()]
    if features:
        info(f"Features: {', '.join(features)}")

    LOG_DIR.mkdir(parents=True, exist_ok=True)

    # ── Build ─────────────────────────────────────────────────────────
    if not args.skip_build:
        if not build_kernel(features=features, release=not args.debug_build):
            err("Cannot proceed without a kernel binary.")
            return 1
    else:
        if not KERNEL_BIN.exists():
            err(f"Kernel binary not found: {KERNEL_BIN}")
            return 1
        info(f"Using existing binary: {KERNEL_BIN}")

    # ── Probe loop ────────────────────────────────────────────────────
    try:
        run_probe(
            state,
            cycle_timeout=args.cycle_timeout,
            poke_interval=args.poke_interval if not args.no_poke else 999999,
            poke_burst=args.poke_burst if not args.no_poke else 0,
            memory=args.memory,
            cpus=args.cpus,
            dashboard_interval=args.dashboard_interval,
        )
    except Exception:
        err("Unhandled exception in probe loop:")
        traceback.print_exc()

    # ── Final report ──────────────────────────────────────────────────
    print_final_report(state)

    return 0


if __name__ == "__main__":
    sys.exit(main())
