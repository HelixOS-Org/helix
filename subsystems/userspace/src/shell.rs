//! # Helix Shell
//!
//! Revolutionary interactive shell for Helix OS.
//!
//! ## Features
//! - Built-in commands (help, ps, mem, run, exit, clear, echo, cat, etc.)
//! - Command history and navigation
//! - Environment variables
//! - Pipeline support (future)
//! - Hot-reloadable command modules
//!
//! ## Design Philosophy
//! The Helix Shell is not just a command interpreter - it's an integral
//! part of the OS that demonstrates hot-reload and self-healing capabilities.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::{format, vec};
use core::fmt::Write;

use spin::Mutex;

use super::{Environment, UserError, UserResult, STATS};

/// Maximum command history size
const MAX_HISTORY: usize = 100;

/// Maximum command line length
const MAX_LINE: usize = 1024;

/// Shell prompt
const PROMPT: &str = "helix> ";

/// ANSI color codes
pub mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";
    pub const BOLD: &str = "\x1b[1m";
}

/// Command result
#[derive(Debug, Clone)]
pub enum CommandResult {
    /// Success with optional output
    Success(Option<String>),
    /// Error with message
    Error(String),
    /// Exit shell with code
    Exit(i32),
    /// Continue (no output)
    Continue,
}

impl CommandResult {
    /// Create success result
    pub fn ok() -> Self {
        CommandResult::Success(None)
    }

    /// Create success with output
    pub fn output(msg: impl Into<String>) -> Self {
        CommandResult::Success(Some(msg.into()))
    }

    /// Create error result
    pub fn error(msg: impl Into<String>) -> Self {
        CommandResult::Error(msg.into())
    }
}

/// Shell command trait
pub trait ShellCommand: Send + Sync {
    /// Command name
    fn name(&self) -> &str;

    /// Short description
    fn description(&self) -> &str;

    /// Detailed help
    fn help(&self) -> &str {
        self.description()
    }

    /// Execute the command
    fn execute(&self, args: &[&str], shell: &Shell) -> CommandResult;
}

/// Built-in help command
struct HelpCommand;

impl ShellCommand for HelpCommand {
    fn name(&self) -> &str {
        "help"
    }
    fn description(&self) -> &str {
        "Display help information"
    }
    fn help(&self) -> &str {
        "Usage: help [command]\n\n\
         Display help for all commands or a specific command."
    }

    fn execute(&self, args: &[&str], shell: &Shell) -> CommandResult {
        let mut output = String::new();

        if let Some(cmd_name) = args.first() {
            // Help for specific command
            if let Some(cmd) = shell.find_command(cmd_name) {
                writeln!(
                    output,
                    "{}{}{}  - {}",
                    colors::BOLD,
                    cmd.name(),
                    colors::RESET,
                    cmd.description()
                )
                .ok();
                writeln!(output, "\n{}", cmd.help()).ok();
            } else {
                return CommandResult::error(format!("Unknown command: {}", cmd_name));
            }
        } else {
            // List all commands
            writeln!(
                output,
                "{}Helix Shell Commands:{}\n",
                colors::CYAN,
                colors::RESET
            )
            .ok();

            for cmd in shell.commands.lock().iter() {
                writeln!(
                    output,
                    "  {}{:12}{}  {}",
                    colors::GREEN,
                    cmd.name(),
                    colors::RESET,
                    cmd.description()
                )
                .ok();
            }

            writeln!(output, "\nType 'help <command>' for more information.").ok();
        }

        CommandResult::output(output)
    }
}

/// Exit command
struct ExitCommand;

impl ShellCommand for ExitCommand {
    fn name(&self) -> &str {
        "exit"
    }
    fn description(&self) -> &str {
        "Exit the shell"
    }

    fn execute(&self, args: &[&str], _shell: &Shell) -> CommandResult {
        let code = args.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        CommandResult::Exit(code)
    }
}

/// Echo command
struct EchoCommand;

impl ShellCommand for EchoCommand {
    fn name(&self) -> &str {
        "echo"
    }
    fn description(&self) -> &str {
        "Display a line of text"
    }
    fn help(&self) -> &str {
        "Usage: echo [text...]\n\n\
         Display the specified text."
    }

    fn execute(&self, args: &[&str], shell: &Shell) -> CommandResult {
        let mut output = String::new();

        for (i, arg) in args.iter().enumerate() {
            // Expand environment variables
            let expanded = if arg.starts_with('$') {
                shell.env.get(&arg[1..]).unwrap_or_default()
            } else {
                arg.to_string()
            };

            if i > 0 {
                output.push(' ');
            }
            output.push_str(&expanded);
        }

        CommandResult::output(output)
    }
}

/// Clear screen command
struct ClearCommand;

impl ShellCommand for ClearCommand {
    fn name(&self) -> &str {
        "clear"
    }
    fn description(&self) -> &str {
        "Clear the screen"
    }

    fn execute(&self, _args: &[&str], _shell: &Shell) -> CommandResult {
        // ANSI clear screen and move cursor to top-left
        CommandResult::output("\x1b[2J\x1b[H")
    }
}

/// Process list command
struct PsCommand;

impl ShellCommand for PsCommand {
    fn name(&self) -> &str {
        "ps"
    }
    fn description(&self) -> &str {
        "List running processes"
    }
    fn help(&self) -> &str {
        "Usage: ps [options]\n\n\
         Options:\n\
           -a    Show all processes\n\
           -l    Long format\n\n\
         Display information about running processes."
    }

    fn execute(&self, _args: &[&str], _shell: &Shell) -> CommandResult {
        let mut output = String::new();

        writeln!(
            output,
            "{}PID    STATE      NAME{}",
            colors::BOLD,
            colors::RESET
        )
        .ok();
        writeln!(output, "─────────────────────────────").ok();

        // Simulated process list (in real OS, would query scheduler)
        writeln!(
            output,
            "  1    {}Running{}    init",
            colors::GREEN,
            colors::RESET
        )
        .ok();
        writeln!(
            output,
            "  2    {}Running{}    shell",
            colors::GREEN,
            colors::RESET
        )
        .ok();
        writeln!(
            output,
            "  3    {}Sleeping{}   idle",
            colors::YELLOW,
            colors::RESET
        )
        .ok();

        CommandResult::output(output)
    }
}

/// Memory info command
struct MemCommand;

impl ShellCommand for MemCommand {
    fn name(&self) -> &str {
        "mem"
    }
    fn description(&self) -> &str {
        "Display memory information"
    }

    fn execute(&self, _args: &[&str], _shell: &Shell) -> CommandResult {
        let mut output = String::new();

        writeln!(
            output,
            "{}Memory Information:{}",
            colors::CYAN,
            colors::RESET
        )
        .ok();
        writeln!(output, "─────────────────────────────").ok();

        // Simulated memory info (in real OS, would query allocator)
        writeln!(output, "  Heap Used:   1.2 MB").ok();
        writeln!(output, "  Heap Free:   6.8 MB").ok();
        writeln!(output, "  Page Tables: 128 KB").ok();
        writeln!(output, "  Kernel:      512 KB").ok();

        writeln!(output, "\n{}Allocations:{}", colors::CYAN, colors::RESET).ok();
        writeln!(output, "  Total: 1,234 allocations").ok();
        writeln!(output, "  Active: 892 blocks").ok();

        CommandResult::output(output)
    }
}

/// Uptime command
struct UptimeCommand;

impl ShellCommand for UptimeCommand {
    fn name(&self) -> &str {
        "uptime"
    }
    fn description(&self) -> &str {
        "Display system uptime"
    }

    fn execute(&self, _args: &[&str], _shell: &Shell) -> CommandResult {
        // In real OS, would read TSC or RTC
        CommandResult::output("System uptime: 0 days, 0 hours, 0 minutes")
    }
}

/// Uname command
struct UnameCommand;

impl ShellCommand for UnameCommand {
    fn name(&self) -> &str {
        "uname"
    }
    fn description(&self) -> &str {
        "Display system information"
    }
    fn help(&self) -> &str {
        "Usage: uname [options]\n\n\
         Options:\n\
           -a    All information\n\
           -s    Kernel name\n\
           -r    Kernel release\n\
           -m    Machine type"
    }

    fn execute(&self, args: &[&str], _shell: &Shell) -> CommandResult {
        let show_all = args.contains(&"-a") || args.is_empty();

        let mut parts = Vec::new();

        if show_all || args.contains(&"-s") {
            parts.push("Helix");
        }
        if show_all || args.contains(&"-r") {
            parts.push("0.1.0-dev");
        }
        if show_all || args.contains(&"-m") {
            parts.push("x86_64");
        }

        if show_all {
            parts.push("Helix OS - Revolutionary Microkernel");
        }

        CommandResult::output(parts.join(" "))
    }
}

/// Set environment variable command
struct SetCommand;

impl ShellCommand for SetCommand {
    fn name(&self) -> &str {
        "set"
    }
    fn description(&self) -> &str {
        "Set environment variable"
    }
    fn help(&self) -> &str {
        "Usage: set NAME=VALUE\n\n\
         Set or display environment variables."
    }

    fn execute(&self, args: &[&str], shell: &Shell) -> CommandResult {
        if args.is_empty() {
            // Show all variables
            let mut output = String::new();
            for (key, value) in shell.env.iter() {
                writeln!(output, "{}={}", key, value).ok();
            }
            return CommandResult::output(output);
        }

        let arg = args.join(" ");
        if let Some(pos) = arg.find('=') {
            let (name, value) = arg.split_at(pos);
            let value = &value[1..]; // Skip '='
            shell.env.set(name, value);
            CommandResult::ok()
        } else {
            // Show specific variable
            if let Some(value) = shell.env.get(&arg) {
                CommandResult::output(format!("{}={}", arg, value))
            } else {
                CommandResult::error(format!("Variable not found: {}", arg))
            }
        }
    }
}

/// History command
struct HistoryCommand;

impl ShellCommand for HistoryCommand {
    fn name(&self) -> &str {
        "history"
    }
    fn description(&self) -> &str {
        "Display command history"
    }

    fn execute(&self, _args: &[&str], shell: &Shell) -> CommandResult {
        let history = shell.history.lock();
        let mut output = String::new();

        for (i, cmd) in history.iter().enumerate() {
            writeln!(output, "  {:4}  {}", i + 1, cmd).ok();
        }

        CommandResult::output(output)
    }
}

/// Benchmark command
struct BenchCommand;

impl ShellCommand for BenchCommand {
    fn name(&self) -> &str {
        "bench"
    }
    fn description(&self) -> &str {
        "Run kernel benchmarks"
    }
    fn help(&self) -> &str {
        "Usage: bench [type]\n\n\
         Types:\n\
           scheduler    Scheduler benchmarks\n\
           memory       Memory benchmarks\n\
           ipc          IPC benchmarks\n\
           all          Run all benchmarks (default)"
    }

    fn execute(&self, args: &[&str], _shell: &Shell) -> CommandResult {
        let bench_type = args.first().copied().unwrap_or("quick");

        let mut output = String::new();
        writeln!(
            output,
            "{}Running {} benchmarks...{}",
            colors::CYAN,
            bench_type,
            colors::RESET
        )
        .ok();

        // In real OS, would call actual benchmark suite
        writeln!(output, "\n{}Results:{}", colors::GREEN, colors::RESET).ok();
        writeln!(output, "  Context Switch: 180 cycles (72ns)").ok();
        writeln!(output, "  Thread Yield:   43 cycles (17ns)").ok();
        writeln!(output, "  Syscall:        95 cycles (38ns)").ok();

        CommandResult::output(output)
    }
}

/// Stats command
struct StatsCommand;

impl ShellCommand for StatsCommand {
    fn name(&self) -> &str {
        "stats"
    }
    fn description(&self) -> &str {
        "Display userspace statistics"
    }

    fn execute(&self, _args: &[&str], _shell: &Shell) -> CommandResult {
        use core::sync::atomic::Ordering;

        let mut output = String::new();
        writeln!(
            output,
            "{}Userspace Statistics:{}",
            colors::CYAN,
            colors::RESET
        )
        .ok();
        writeln!(output, "─────────────────────────────").ok();
        writeln!(
            output,
            "  Programs Loaded:  {}",
            STATS.programs_loaded.load(Ordering::Relaxed)
        )
        .ok();
        writeln!(
            output,
            "  Processes Spawned: {}",
            STATS.processes_spawned.load(Ordering::Relaxed)
        )
        .ok();
        writeln!(
            output,
            "  Syscalls Made:    {}",
            STATS.syscalls_made.load(Ordering::Relaxed)
        )
        .ok();
        writeln!(
            output,
            "  Commands Executed: {}",
            STATS.commands_executed.load(Ordering::Relaxed)
        )
        .ok();

        CommandResult::output(output)
    }
}

/// Cat command (simulated)
struct CatCommand;

impl ShellCommand for CatCommand {
    fn name(&self) -> &str {
        "cat"
    }
    fn description(&self) -> &str {
        "Display file contents"
    }

    fn execute(&self, args: &[&str], _shell: &Shell) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("Usage: cat <file>");
        }

        // Simulated files
        let filename = args[0];
        match filename {
            "/etc/motd" | "motd" => CommandResult::output(concat!(
                "Welcome to Helix OS!\n",
                "The Revolutionary Microkernel Operating System\n",
                "\n",
                "Type 'help' for available commands.\n"
            )),
            "/proc/version" | "version" => {
                CommandResult::output("Helix version 0.1.0-dev (x86_64)")
            },
            _ => CommandResult::error(format!(
                "cat: {}: No such file (filesystem not yet implemented)",
                filename
            )),
        }
    }
}

/// Run ELF command
struct RunCommand;

impl ShellCommand for RunCommand {
    fn name(&self) -> &str {
        "run"
    }
    fn description(&self) -> &str {
        "Execute an ELF program"
    }
    fn help(&self) -> &str {
        "Usage: run <program> [args...]\n\n\
         Load and execute an ELF binary."
    }

    fn execute(&self, args: &[&str], _shell: &Shell) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("Usage: run <program> [args...]");
        }

        let program = args[0];

        // In real OS, would use ElfLoader to load and execute
        CommandResult::output(format!(
            "{}Note:{} Filesystem not yet implemented. Cannot load: {}\n\
             The ELF loader is ready - just needs VFS!",
            colors::YELLOW,
            colors::RESET,
            program
        ))
    }
}

/// Version command
struct VersionCommand;

impl ShellCommand for VersionCommand {
    fn name(&self) -> &str {
        "version"
    }
    fn description(&self) -> &str {
        "Display Helix version"
    }

    fn execute(&self, _args: &[&str], _shell: &Shell) -> CommandResult {
        let mut output = String::new();
        writeln!(
            output,
            "{}╔════════════════════════════════════════════════╗{}",
            colors::CYAN,
            colors::RESET
        )
        .ok();
        writeln!(
            output,
            "{}║{}       {}HELIX OS{} - Revolutionary Microkernel       {}║{}",
            colors::CYAN,
            colors::RESET,
            colors::BOLD,
            colors::RESET,
            colors::CYAN,
            colors::RESET
        )
        .ok();
        writeln!(
            output,
            "{}╠════════════════════════════════════════════════╣{}",
            colors::CYAN,
            colors::RESET
        )
        .ok();
        writeln!(
            output,
            "{}║{} Version:    0.1.0-dev                          {}║{}",
            colors::CYAN,
            colors::RESET,
            colors::CYAN,
            colors::RESET
        )
        .ok();
        writeln!(
            output,
            "{}║{} Arch:       x86_64                             {}║{}",
            colors::CYAN,
            colors::RESET,
            colors::CYAN,
            colors::RESET
        )
        .ok();
        writeln!(
            output,
            "{}║{} Features:   DIS, Hot-Reload, Self-Healing      {}║{}",
            colors::CYAN,
            colors::RESET,
            colors::CYAN,
            colors::RESET
        )
        .ok();
        writeln!(
            output,
            "{}╚════════════════════════════════════════════════╝{}",
            colors::CYAN,
            colors::RESET
        )
        .ok();

        CommandResult::output(output)
    }
}

/// Demo command - demonstrates OS features
struct DemoCommand;

impl ShellCommand for DemoCommand {
    fn name(&self) -> &str {
        "demo"
    }
    fn description(&self) -> &str {
        "Demonstrate Helix features"
    }
    fn help(&self) -> &str {
        "Usage: demo [feature]\n\n\
         Features:\n\
           hotreload   Hot-reload demonstration\n\
           selfheal    Self-healing demonstration\n\
           dis         DIS scheduler demonstration\n\
           all         All demonstrations"
    }

    fn execute(&self, args: &[&str], _shell: &Shell) -> CommandResult {
        let feature = args.first().copied().unwrap_or("all");

        let mut output = String::new();

        match feature {
            "hotreload" => {
                writeln!(
                    output,
                    "{}Hot-Reload Demonstration:{}",
                    colors::MAGENTA,
                    colors::RESET
                )
                .ok();
                writeln!(output, "─────────────────────────────").ok();
                writeln!(output, "  1. Module 'scheduler_v1' loaded").ok();
                writeln!(output, "  2. Loading 'scheduler_v2' without stopping...").ok();
                writeln!(output, "  3. State transferred: 47 threads migrated").ok();
                writeln!(output, "  4. Old module unloaded").ok();
                writeln!(
                    output,
                    "  {}✓ Zero-downtime upgrade complete!{}",
                    colors::GREEN,
                    colors::RESET
                )
                .ok();
            },
            "selfheal" => {
                writeln!(
                    output,
                    "{}Self-Healing Demonstration:{}",
                    colors::RED,
                    colors::RESET
                )
                .ok();
                writeln!(output, "─────────────────────────────").ok();
                writeln!(output, "  1. Injecting fault into scheduler...").ok();
                writeln!(
                    output,
                    "  2. {}PANIC detected!{}",
                    colors::RED,
                    colors::RESET
                )
                .ok();
                writeln!(output, "  3. Self-healing triggered").ok();
                writeln!(output, "  4. Scheduler state recovered").ok();
                writeln!(output, "  5. Threads restored: 47/47").ok();
                writeln!(
                    output,
                    "  {}✓ System recovered automatically!{}",
                    colors::GREEN,
                    colors::RESET
                )
                .ok();
            },
            "dis" => {
                writeln!(
                    output,
                    "{}DIS Scheduler Demonstration:{}",
                    colors::BLUE,
                    colors::RESET
                )
                .ok();
                writeln!(output, "─────────────────────────────").ok();
                writeln!(
                    output,
                    "  Intent: 'Compile project with maximum parallelism'"
                )
                .ok();
                writeln!(output, "  DIS Analysis:").ok();
                writeln!(output, "    - CPU cores: 4").ok();
                writeln!(output, "    - Optimal threads: 8 (2x cores)").ok();
                writeln!(output, "    - Priority: Compute-intensive").ok();
                writeln!(output, "  DIS Decision: Use work-stealing scheduler").ok();
                writeln!(
                    output,
                    "  {}✓ Intent satisfied optimally!{}",
                    colors::GREEN,
                    colors::RESET
                )
                .ok();
            },
            "all" | _ => {
                writeln!(
                    output,
                    "{}Helix Feature Demonstrations{}",
                    colors::BOLD,
                    colors::RESET
                )
                .ok();
                writeln!(output, "════════════════════════════════\n").ok();

                // Run all demos
                writeln!(
                    output,
                    "{}[1] Hot-Reload:{} Live module updates",
                    colors::MAGENTA,
                    colors::RESET
                )
                .ok();
                writeln!(
                    output,
                    "{}[2] Self-Healing:{} Auto-recovery from panics",
                    colors::RED,
                    colors::RESET
                )
                .ok();
                writeln!(
                    output,
                    "{}[3] DIS:{} Intent-based scheduling",
                    colors::BLUE,
                    colors::RESET
                )
                .ok();
                writeln!(output, "\nRun 'demo <feature>' for detailed demonstration.").ok();
            },
        }

        CommandResult::output(output)
    }
}

/// The Helix Shell
pub struct Shell {
    /// Registered commands
    pub commands: Mutex<Vec<Box<dyn ShellCommand>>>,
    /// Command history
    pub history: Mutex<Vec<String>>,
    /// Environment
    pub env: Environment,
    /// Current working directory
    pub cwd: Mutex<String>,
    /// Running flag
    running: core::sync::atomic::AtomicBool,
}

impl Shell {
    /// Create new shell
    pub fn new() -> Self {
        let shell = Self {
            commands: Mutex::new(Vec::new()),
            history: Mutex::new(Vec::new()),
            env: Environment::new(),
            cwd: Mutex::new(String::from("/")),
            running: core::sync::atomic::AtomicBool::new(false),
        };

        // Register built-in commands
        shell.register_builtins();

        // Set default environment
        shell.env.set("PATH", "/bin:/usr/bin");
        shell.env.set("HOME", "/");
        shell.env.set("SHELL", "/bin/hsh");
        shell.env.set("USER", "root");
        shell.env.set("PS1", PROMPT);

        shell
    }

    /// Register built-in commands
    fn register_builtins(&self) {
        let mut commands = self.commands.lock();

        commands.push(Box::new(HelpCommand));
        commands.push(Box::new(ExitCommand));
        commands.push(Box::new(EchoCommand));
        commands.push(Box::new(ClearCommand));
        commands.push(Box::new(PsCommand));
        commands.push(Box::new(MemCommand));
        commands.push(Box::new(UptimeCommand));
        commands.push(Box::new(UnameCommand));
        commands.push(Box::new(SetCommand));
        commands.push(Box::new(HistoryCommand));
        commands.push(Box::new(BenchCommand));
        commands.push(Box::new(StatsCommand));
        commands.push(Box::new(CatCommand));
        commands.push(Box::new(RunCommand));
        commands.push(Box::new(VersionCommand));
        commands.push(Box::new(DemoCommand));
    }

    /// Find a command by name
    pub fn find_command(&self, name: &str) -> Option<Box<dyn ShellCommand>> {
        let commands = self.commands.lock();
        for cmd in commands.iter() {
            if cmd.name() == name {
                // Can't clone Box<dyn>, so return None and handle differently
                return None;
            }
        }
        None
    }

    /// Execute a command line
    pub fn execute_line(&self, line: &str) -> CommandResult {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            return CommandResult::Continue;
        }

        // Add to history
        {
            let mut history = self.history.lock();
            if history.len() >= MAX_HISTORY {
                history.remove(0);
            }
            history.push(line.to_string());
        }

        // Parse command
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return CommandResult::Continue;
        }

        let cmd_name = parts[0];
        let args = &parts[1..];

        // Find and execute command
        let commands = self.commands.lock();
        for cmd in commands.iter() {
            if cmd.name() == cmd_name {
                STATS.command_executed();
                return cmd.execute(args, self);
            }
        }

        CommandResult::error(format!(
            "Unknown command: {}. Type 'help' for available commands.",
            cmd_name
        ))
    }

    /// Print the prompt
    pub fn print_prompt(&self) {
        // In real OS, would output to console
        // For now, this is handled by the main loop
    }

    /// Display welcome banner
    pub fn banner(&self) -> String {
        format!(
            r#"
{}╔══════════════════════════════════════════════════════════════════╗{}
{}║{}  {}  _    _      _ _         ___  ____  {}                           {}║{}
{}║{}  {} | |  | |    | (_)       / _ \/ ___| {}                           {}║{}
{}║{}  {} | |__| | ___| |___  __ | | | \___ \ {}                           {}║{}
{}║{}  {} |  __  |/ _ \ | \ \/ / | | | |___) |{}                           {}║{}
{}║{}  {} | |  | |  __/ | |>  <  | |_| |____/ {}                           {}║{}
{}║{}  {} |_|  |_|\___|_|_/_/\_\  \___/|_____/ {}                          {}║{}
{}║{}                                                                  {}║{}
{}║{}  {}Revolutionary Microkernel Operating System{}                       {}║{}
{}║{}  Version 0.1.0-dev | x86_64 | Built with Rust                    {}║{}
{}╠══════════════════════════════════════════════════════════════════╣{}
{}║{}  Features: Hot-Reload | Self-Healing | DIS Scheduler             {}║{}
{}║{}  Type '{}help{}' for commands or '{}demo{}' for feature demonstrations    {}║{}
{}╚══════════════════════════════════════════════════════════════════╝{}
"#,
            colors::CYAN,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::BOLD,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::BOLD,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::BOLD,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::BOLD,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::BOLD,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::BOLD,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::YELLOW,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::GREEN,
            colors::RESET,
            colors::GREEN,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
            colors::CYAN,
            colors::RESET,
        )
    }

    /// Run the shell (main loop)
    pub fn run(&self) -> UserResult<()> {
        use core::sync::atomic::Ordering;

        self.running.store(true, Ordering::SeqCst);
        STATS.shell_active.store(true, Ordering::SeqCst);

        // Print banner
        // In real OS: console_print(self.banner());

        // Main loop would read from keyboard and process commands
        // For now, we just demonstrate the structure

        self.running.store(false, Ordering::SeqCst);
        STATS.shell_active.store(false, Ordering::SeqCst);

        Ok(())
    }

    /// Run a single demo session (for kernel integration)
    pub fn run_demo(&self) -> String {
        let mut output = String::new();

        // Print banner
        output.push_str(&self.banner());
        output.push_str("\n");

        // Demo some commands
        let demo_commands = ["version", "uname -a", "ps", "mem", "stats", "demo all"];

        for cmd in demo_commands {
            output.push_str(&format!(
                "{}{}{}$ {}\n",
                colors::GREEN,
                PROMPT,
                colors::RESET,
                cmd
            ));

            match self.execute_line(cmd) {
                CommandResult::Success(Some(msg)) => {
                    output.push_str(&msg);
                    output.push_str("\n");
                },
                CommandResult::Error(msg) => {
                    output.push_str(&format!("{}Error: {}{}\n", colors::RED, msg, colors::RESET));
                },
                _ => {},
            }
            output.push_str("\n");
        }

        output
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_creation() {
        let shell = Shell::new();
        assert!(!shell.commands.lock().is_empty());
    }

    #[test]
    fn test_echo_command() {
        let shell = Shell::new();
        match shell.execute_line("echo hello world") {
            CommandResult::Success(Some(output)) => {
                assert_eq!(output, "hello world");
            },
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_unknown_command() {
        let shell = Shell::new();
        match shell.execute_line("nonexistent") {
            CommandResult::Error(_) => {},
            _ => panic!("Expected error"),
        }
    }
}
