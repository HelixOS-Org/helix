//! # Declarative Macros
//!
//! This module provides macros for declaring and registering subsystems
//! in a clean, declarative way.
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use helix_init::prelude::*;
//!
//! // Define a subsystem using the macro
//! define_subsystem! {
//!     name: "memory_manager",
//!     phase: Early,
//!     priority: 100,
//!     essential: true,
//!     provides: [MEMORY, HEAP],
//!     requires: [CONSOLE],
//!     dependencies: [
//!         required: "firmware",
//!         optional: "debug_console",
//!     ],
//!
//!     struct MemoryManager {
//!         heap_start: usize,
//!         heap_size: usize,
//!     }
//!
//!     impl {
//!         fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
//!             // Initialize memory management
//!             Ok(())
//!         }
//!     }
//! }
//!
//! // Or use attribute-style registration
//! #[subsystem(phase = "Core", priority = 50)]
//! struct Scheduler {
//!     // ...
//! }
//! ```

/// Define a subsystem with full configuration
///
/// # Syntax
///
/// ```rust,ignore
/// define_subsystem! {
///     name: "subsystem_name",
///     phase: Phase,
///     priority: i32,
///     essential: bool,
///     provides: [CAPS...],
///     requires: [CAPS...],
///     dependencies: [
///         required: "dep1",
///         optional: "dep2",
///     ],
///
///     struct Name { fields... }
///
///     impl {
///         fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> { ... }
///     }
/// }
/// ```
#[macro_export]
macro_rules! define_subsystem {
    (
        name: $name:literal,
        phase: $phase:ident,
        $(priority: $priority:expr,)?
        $(essential: $essential:expr,)?
        $(provides: [$($provides:ident),* $(,)?],)?
        $(requires: [$($requires:ident),* $(,)?],)?
        $(dependencies: [
            $(required: $req_dep:literal),* $(,)?
            $(optional: $opt_dep:literal),* $(,)?
        ],)?

        $(#[$struct_attr:meta])*
        $vis:vis struct $struct_name:ident {
            $($field_vis:vis $field_name:ident : $field_type:ty),* $(,)?
        }

        impl {
            $($impl_item:tt)*
        }
    ) => {
        // Define the struct
        $(#[$struct_attr])*
        $vis struct $struct_name {
            info: $crate::subsystem::SubsystemInfo,
            $($field_vis $field_name : $field_type),*
        }

        // Static dependencies
        static _DEPS: &[$crate::subsystem::Dependency] = &[
            $($($crate::subsystem::Dependency::required($req_dep),)*)?
            $($($crate::subsystem::Dependency::optional($opt_dep),)*)?
        ];

        // Static info
        static _INFO: $crate::subsystem::SubsystemInfo = {
            let mut info = $crate::subsystem::SubsystemInfo::new(
                $name,
                $crate::phase::InitPhase::$phase
            );

            $(info = info.with_priority($priority);)?
            $(if $essential { info = info.essential(); })?

            info = info.with_dependencies(_DEPS);

            $(
                info = info.provides(
                    $crate::phase::PhaseCapabilities::empty()
                    $(| $crate::phase::PhaseCapabilities::$provides)*
                );
            )?

            $(
                info = info.requires(
                    $crate::phase::PhaseCapabilities::empty()
                    $(| $crate::phase::PhaseCapabilities::$requires)*
                );
            )?

            info
        };

        impl $struct_name {
            /// Create new instance
            pub fn new($($field_name : $field_type),*) -> Self {
                Self {
                    info: _INFO.clone(),
                    $($field_name),*
                }
            }

            $($impl_item)*
        }

        impl $crate::subsystem::Subsystem for $struct_name {
            fn info(&self) -> &$crate::subsystem::SubsystemInfo {
                &self.info
            }

            // The init method should be provided in impl block
        }
    };
}

/// Declare dependencies for a subsystem
///
/// # Example
///
/// ```rust,ignore
/// dependencies! {
///     required: "memory", "cpu",
///     optional: "debug",
///     weak: "tracing",
/// }
/// ```
#[macro_export]
macro_rules! dependencies {
    (
        $(required: $($req:literal),* $(,)?)?
        $(optional: $($opt:literal),* $(,)?)?
        $(weak: $($weak:literal),* $(,)?)?
        $(conflict: $($conflict:literal),* $(,)?)?
    ) => {
        &[
            $($($crate::subsystem::Dependency::required($req),)*)?
            $($($crate::subsystem::Dependency::optional($opt),)*)?
            $($($crate::subsystem::Dependency::weak($weak),)*)?
            $($($crate::subsystem::Dependency::conflict($conflict),)*)?
        ]
    };
}

/// Define subsystem info at compile time
///
/// # Example
///
/// ```rust,ignore
/// static MY_INFO: SubsystemInfo = subsystem_info! {
///     name: "my_subsystem",
///     phase: Core,
///     priority: 50,
/// };
/// ```
#[macro_export]
macro_rules! subsystem_info {
    (
        name: $name:literal,
        phase: $phase:ident
        $(, priority: $priority:expr)?
        $(, essential: $essential:expr)?
        $(, timeout: $timeout:expr)?
        $(,)?
    ) => {{
        let mut info = $crate::subsystem::SubsystemInfo::new(
            $name,
            $crate::phase::InitPhase::$phase
        );

        $(info = info.with_priority($priority);)?
        $(if $essential { info = info.essential(); })?
        $(info = info.with_timeout($timeout);)?

        info
    }};
}

/// Register a subsystem for static initialization
///
/// This macro creates a static constructor that registers the subsystem
/// with the global registry at startup.
///
/// # Example
///
/// ```rust,ignore
/// register_subsystem!(MemoryManager::new);
/// ```
#[macro_export]
macro_rules! register_subsystem {
    ($factory:expr, $info:expr) => {
        #[cfg(not(test))]
        #[used]
        #[cfg_attr(target_os = "linux", link_section = ".init_array")]
        #[cfg_attr(target_os = "macos", link_section = "__DATA,__mod_init_func")]
        #[cfg_attr(windows, link_section = ".CRT$XCU")]
        static _REGISTER: extern "C" fn() = {
            extern "C" fn _register() {
                unsafe {
                    $crate::registry::register_static(|| ::alloc::boxed::Box::new($factory), $info);
                }
            }
            _register
        };
    };
}

/// Create a rollback action from a closure
///
/// # Example
///
/// ```rust,ignore
/// let rollback = rollback_action!("Cleanup memory", || {
///     // cleanup code
///     Ok(())
/// });
/// ctx.add_rollback(rollback);
/// ```
#[macro_export]
macro_rules! rollback_action {
    ($desc:literal, $action:expr) => {
        $crate::error::FnRollback::new($action, $desc)
    };
    ($desc:literal,critical, $action:expr) => {
        $crate::error::FnRollback::new($action, $desc).critical()
    };
}

/// Assert we're in the correct phase
///
/// # Example
///
/// ```rust,ignore
/// require_phase!(ctx, Core);
/// ```
#[macro_export]
macro_rules! require_phase {
    ($ctx:expr, $phase:ident) => {
        $ctx.require_phase($crate::phase::InitPhase::$phase)?
    };
}

/// Assert a capability is available
///
/// # Example
///
/// ```rust,ignore
/// require_capability!(ctx, HEAP);
/// ```
#[macro_export]
macro_rules! require_capability {
    ($ctx:expr, $cap:ident) => {
        $ctx.require_capability($crate::phase::PhaseCapabilities::$cap)?
    };
}

/// Log at different levels
#[macro_export]
macro_rules! init_trace {
    ($ctx:expr, $($arg:tt)*) => {
        $ctx.trace(::alloc::format!($($arg)*))
    };
}

#[macro_export]
macro_rules! init_debug {
    ($ctx:expr, $($arg:tt)*) => {
        $ctx.debug(::alloc::format!($($arg)*))
    };
}

#[macro_export]
macro_rules! init_info {
    ($ctx:expr, $($arg:tt)*) => {
        $ctx.info(::alloc::format!($($arg)*))
    };
}

#[macro_export]
macro_rules! init_warn {
    ($ctx:expr, $($arg:tt)*) => {
        $ctx.warn(::alloc::format!($($arg)*))
    };
}

#[macro_export]
macro_rules! init_error {
    ($ctx:expr, $($arg:tt)*) => {
        $ctx.error(::alloc::format!($($arg)*))
    };
}

/// Implement Subsystem trait with boilerplate
#[macro_export]
macro_rules! impl_subsystem {
    (
        for $type:ty;
        info = $info:expr;

        $(validate($v_ctx:ident) $validate_body:block)?

        init($i_self:ident, $i_ctx:ident) $init_body:block

        $(shutdown($s_self:ident, $s_ctx:ident) $shutdown_body:block)?
    ) => {
        impl $crate::subsystem::Subsystem for $type {
            fn info(&self) -> &$crate::subsystem::SubsystemInfo {
                $info
            }

            $(
            fn validate(&self, $v_ctx: &$crate::context::InitContext) -> $crate::error::InitResult<()> {
                $validate_body
            }
            )?

            fn init(&mut $i_self, $i_ctx: &mut $crate::context::InitContext) -> $crate::error::InitResult<()> {
                $init_body
            }

            $(
            fn shutdown(&mut $s_self, $s_ctx: &mut $crate::context::InitContext) -> $crate::error::InitResult<()> {
                $shutdown_body
            }
            )?
        }
    };
}

/// Quick subsystem definition for simple cases
#[macro_export]
macro_rules! simple_subsystem {
    ($name:ident,phase: $phase:ident,init: | $ctx:ident | $body:block) => {
        pub struct $name {
            info: $crate::subsystem::SubsystemInfo,
        }

        impl $name {
            pub fn new() -> Self {
                Self {
                    info: $crate::subsystem::SubsystemInfo::new(
                        stringify!($name),
                        $crate::phase::InitPhase::$phase,
                    ),
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $crate::subsystem::Subsystem for $name {
            fn info(&self) -> &$crate::subsystem::SubsystemInfo {
                &self.info
            }

            fn init(
                &mut self,
                $ctx: &mut $crate::context::InitContext,
            ) -> $crate::error::InitResult<()> {
                $body
            }
        }
    };
}

/// Define multiple subsystems in a module
#[macro_export]
macro_rules! subsystem_module {
    (
        module $mod_name:ident {
            $($subsystem:tt)*
        }
    ) => {
        pub mod $mod_name {
            use super::*;
            use $crate::prelude::*;

            $($subsystem)*
        }
    };
}

/// Create config value
#[macro_export]
macro_rules! config_value {
    (bool: $val:expr) => {
        $crate::context::ConfigValue::Bool($val)
    };
    (int: $val:expr) => {
        $crate::context::ConfigValue::Int($val)
    };
    (uint: $val:expr) => {
        $crate::context::ConfigValue::Uint($val)
    };
    (string: $val:expr) => {
        $crate::context::ConfigValue::String(::alloc::string::String::from($val))
    };
}

/// Build config from key-value pairs
#[macro_export]
macro_rules! build_config {
    ($($key:literal => $value:expr),* $(,)?) => {{
        let mut config = $crate::context::MemoryConfig::new();
        $(
            config.set($key, $value);
        )*
        config
    }};
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn test_dependencies_macro() {
        let deps: &[Dependency] = dependencies! {
            required: "a", "b",
            optional: "c",
        };

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0].kind, DependencyKind::Required);
        assert_eq!(deps[2].kind, DependencyKind::Optional);
    }

    #[test]
    fn test_subsystem_info_macro() {
        let info = subsystem_info! {
            name: "test",
            phase: Core,
            priority: 50,
        };

        assert_eq!(info.name, "test");
        assert_eq!(info.phase, InitPhase::Core);
        assert_eq!(info.priority, 50);
    }

    #[test]
    fn test_simple_subsystem_macro() {
        simple_subsystem! {
            TestSub,
            phase: Boot,
            init: |_ctx| {
                Ok(())
            }
        }

        let sub = TestSub::new();
        assert_eq!(sub.info().phase, InitPhase::Boot);
    }

    #[test]
    fn test_config_value_macro() {
        let bool_val = config_value!(bool: true);
        assert_eq!(bool_val.as_bool(), Some(true));

        let int_val = config_value!(int: 42);
        assert_eq!(int_val.as_int(), Some(42));
    }

    #[test]
    fn test_build_config_macro() {
        let config = build_config! {
            "debug" => ConfigValue::Bool(true),
            "timeout" => ConfigValue::Int(1000),
        };

        assert_eq!(config.get_bool("debug", false), true);
        assert_eq!(config.get_int("timeout", 0), 1000);
    }
}
