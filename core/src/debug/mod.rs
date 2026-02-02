//! # Debug Interface
//!
//! Kernel debugging facilities.

pub mod console;

use alloc::string::String;
use alloc::vec::Vec;

/// Debug command handler
pub trait DebugCommand: Send + Sync {
    /// Command name
    fn name(&self) -> &'static str;

    /// Command description
    fn description(&self) -> &'static str;

    /// Execute the command
    fn execute(&self, args: &[&str]) -> Result<String, String>;
}

/// Debug interface
pub struct DebugInterface {
    /// Registered commands
    commands: spin::RwLock<Vec<&'static dyn DebugCommand>>,
}

impl DebugInterface {
    /// Create a new debug interface
    pub const fn new() -> Self {
        Self {
            commands: spin::RwLock::new(Vec::new()),
        }
    }

    /// Register a debug command
    pub fn register_command(&self, command: &'static dyn DebugCommand) {
        self.commands.write().push(command);
    }

    /// Execute a command by name
    pub fn execute(&self, input: &str) -> Result<String, String> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Err("No command specified".into());
        }

        let name = parts[0];
        let args = &parts[1..];

        let commands = self.commands.read();
        for cmd in commands.iter() {
            if cmd.name() == name {
                return cmd.execute(args);
            }
        }

        Err(alloc::format!("Unknown command: {}", name))
    }

    /// List available commands
    pub fn list_commands(&self) -> Vec<(&'static str, &'static str)> {
        self.commands
            .read()
            .iter()
            .map(|c| (c.name(), c.description()))
            .collect()
    }
}

/// Global debug interface
static DEBUG: DebugInterface = DebugInterface::new();

/// Get the debug interface
pub fn debug_interface() -> &'static DebugInterface {
    &DEBUG
}
