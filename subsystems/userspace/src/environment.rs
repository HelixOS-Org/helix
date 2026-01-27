//! # Environment Variables
//!
//! Environment variable management for userspace programs.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use spin::RwLock;

/// Environment variable
#[derive(Debug, Clone)]
pub struct EnvVar {
    /// Variable name
    pub name: String,
    /// Variable value
    pub value: String,
    /// Is exported to child processes
    pub exported: bool,
}

impl EnvVar {
    /// Create new environment variable
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            exported: true,
        }
    }
    
    /// Create unexported variable
    pub fn local(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            exported: false,
        }
    }
    
    /// Format as NAME=VALUE
    pub fn format(&self) -> String {
        alloc::format!("{}={}", self.name, self.value)
    }
}

/// Environment container
#[derive(Debug)]
pub struct Environment {
    /// Variables
    vars: RwLock<BTreeMap<String, EnvVar>>,
}

impl Environment {
    /// Create new empty environment
    pub fn new() -> Self {
        Self {
            vars: RwLock::new(BTreeMap::new()),
        }
    }
    
    /// Create with default variables
    pub fn with_defaults() -> Self {
        let env = Self::new();
        
        // Set defaults
        env.set("PATH", "/bin:/usr/bin:/sbin:/usr/sbin");
        env.set("HOME", "/root");
        env.set("USER", "root");
        env.set("SHELL", "/bin/hsh");
        env.set("TERM", "helix-term");
        env.set("LANG", "en_US.UTF-8");
        env.set("PWD", "/");
        
        env
    }
    
    /// Get variable value
    pub fn get(&self, name: &str) -> Option<String> {
        self.vars.read().get(name).map(|v| v.value.clone())
    }
    
    /// Set variable
    pub fn set(&self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into();
        let var = EnvVar::new(name.clone(), value);
        self.vars.write().insert(name, var);
    }
    
    /// Set local (unexported) variable
    pub fn set_local(&self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into();
        let var = EnvVar::local(name.clone(), value);
        self.vars.write().insert(name, var);
    }
    
    /// Remove variable
    pub fn unset(&self, name: &str) {
        self.vars.write().remove(name);
    }
    
    /// Check if variable exists
    pub fn contains(&self, name: &str) -> bool {
        self.vars.read().contains_key(name)
    }
    
    /// Get all variables
    pub fn all(&self) -> alloc::vec::Vec<(String, String)> {
        self.vars.read()
            .iter()
            .map(|(k, v)| (k.clone(), v.value.clone()))
            .collect()
    }
    
    /// Get exported variables only
    pub fn exported(&self) -> alloc::vec::Vec<(String, String)> {
        self.vars.read()
            .iter()
            .filter(|(_, v)| v.exported)
            .map(|(k, v)| (k.clone(), v.value.clone()))
            .collect()
    }
    
    /// Iterate over all variables
    pub fn iter(&self) -> alloc::vec::Vec<(String, String)> {
        self.all()
    }
    
    /// Expand variables in a string
    /// Replaces $VAR and ${VAR} with their values
    pub fn expand(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();
        
        while let Some(c) = chars.next() {
            if c == '$' {
                if chars.peek() == Some(&'{') {
                    // ${VAR} format
                    chars.next(); // consume '{'
                    let mut var_name = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch == '}' {
                            chars.next();
                            break;
                        }
                        var_name.push(chars.next().unwrap());
                    }
                    if let Some(value) = self.get(&var_name) {
                        result.push_str(&value);
                    }
                } else {
                    // $VAR format
                    let mut var_name = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch.is_alphanumeric() || ch == '_' {
                            var_name.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    if !var_name.is_empty() {
                        if let Some(value) = self.get(&var_name) {
                            result.push_str(&value);
                        }
                    } else {
                        result.push('$');
                    }
                }
            } else {
                result.push(c);
            }
        }
        
        result
    }
    
    /// Clone environment for child process
    pub fn clone_for_child(&self) -> Self {
        let child = Self::new();
        for (name, value) in self.exported() {
            child.set(name, value);
        }
        child
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl Clone for Environment {
    fn clone(&self) -> Self {
        let new_env = Self::new();
        for (name, value) in self.all() {
            new_env.set(name, value);
        }
        new_env
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_basic() {
        let env = Environment::new();
        env.set("FOO", "bar");
        assert_eq!(env.get("FOO"), Some("bar".to_string()));
    }

    #[test]
    fn test_env_expand() {
        let env = Environment::new();
        env.set("NAME", "Helix");
        env.set("VERSION", "0.1.0");
        
        let result = env.expand("Welcome to $NAME v${VERSION}!");
        assert_eq!(result, "Welcome to Helix v0.1.0!");
    }

    #[test]
    fn test_env_defaults() {
        let env = Environment::with_defaults();
        assert!(env.contains("PATH"));
        assert!(env.contains("HOME"));
    }
}
