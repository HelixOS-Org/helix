# Helix Userspace Subsystem

The revolutionary userspace subsystem for Helix OS, providing:

## Features

### 🔄 ELF64 Loader
- Full ELF64 parsing and loading
- Dynamic relocation support
- Program header interpretation
- Segment permission mapping

### 🐚 Helix Shell
- Interactive shell with command history
- Built-in commands (help, ps, mem, run, exit, clear)
- Command parsing and execution
- Environment variable support

### ⚙️ Runtime
- Process spawning and management
- Userspace memory management
- Signal handling framework
- File descriptor table

### 📞 Syscall Interface
- POSIX-compatible syscall numbers
- Full argument validation
- Return value conventions
- Error code mapping

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Helix Shell                          │
│              Interactive User Interface                 │
├─────────────────────────────────────────────────────────┤
│                  Command Executor                        │
│           Parse → Resolve → Execute → Output            │
├─────────────────────────────────────────────────────────┤
│     ELF Loader     │     Runtime     │    Syscalls     │
│   Load Programs    │  Manage Procs   │  Kernel Bridge  │
├─────────────────────────────────────────────────────────┤
│                    Memory Manager                        │
│         Userspace Allocations & Page Tables             │
├─────────────────────────────────────────────────────────┤
│                    Helix Kernel                          │
└─────────────────────────────────────────────────────────┘
```

## Usage

```rust
use helix_userspace::{Shell, Runtime, ElfLoader};

// Initialize userspace runtime
let runtime = Runtime::init()?;

// Start interactive shell
let shell = Shell::new(&runtime);
shell.run()?;

// Or load an ELF program
let loader = ElfLoader::new(&runtime);
let process = loader.load("/bin/hello")?;
process.spawn()?;
```

## Syscall Table

| Number | Name    | Description              |
|--------|---------|--------------------------|
| 0      | read    | Read from file descriptor|
| 1      | write   | Write to file descriptor |
| 2      | open    | Open a file              |
| 3      | close   | Close file descriptor    |
| 57     | fork    | Create child process     |
| 59     | execve  | Execute program          |
| 60     | exit    | Terminate process        |
| 62     | kill    | Send signal              |

## Revolutionary Aspects

1. **Hot-Reloadable Shell**: Commands can be updated without restart
2. **Intent-Based Execution**: Commands express intent, DIS optimizes
3. **Self-Healing Runtime**: Crashed processes auto-restart
4. **Integrated Benchmarks**: Performance metrics built-in
