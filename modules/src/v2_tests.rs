//! # Module System v2 Tests
//!
//! Unit tests for the unified module API.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::*;
    use crate::{ModuleError, ModuleFlags, ModuleVersion};
    
    // =========================================================================
    // Test Module Implementation
    // =========================================================================
    
    struct TestModule {
        initialized: bool,
        started: bool,
        events_received: usize,
    }
    
    impl TestModule {
        fn new() -> Self {
            Self {
                initialized: false,
                started: false,
                events_received: 0,
            }
        }
    }
    
    impl ModuleTrait for TestModule {
        fn info(&self) -> ModuleInfo {
            ModuleInfo::new("test-module")
                .version(1, 0, 0)
                .description("Test module for unit tests")
                .author("Test Author")
                .flags(ModuleFlags::empty())
                .provides(&["test.service"])
        }
        
        fn init(&mut self, _ctx: &Context) -> Result<(), ModuleError> {
            self.initialized = true;
            Ok(())
        }
        
        fn start(&mut self) -> Result<(), ModuleError> {
            if !self.initialized {
                return Err(ModuleError::WrongState {
                    current: crate::ModuleState::Loaded,
                    required: crate::ModuleState::Running,
                });
            }
            self.started = true;
            Ok(())
        }
        
        fn stop(&mut self) -> Result<(), ModuleError> {
            self.started = false;
            Ok(())
        }
        
        fn handle_event(&mut self, event: &Event) -> EventResponse {
            self.events_received += 1;
            match event {
                Event::Tick { .. } => EventResponse::Handled,
                Event::Shutdown => EventResponse::Handled,
                _ => EventResponse::Ignored,
            }
        }
        
        fn handle_request(&mut self, request: &Request) -> Result<Response, ModuleError> {
            match request.request_type.as_str() {
                "ping" => Ok(Response::ok(b"pong".to_vec())),
                _ => Ok(Response::err("Unknown request")),
            }
        }
        
        fn is_healthy(&self) -> bool {
            self.initialized
        }
    }
    
    // =========================================================================
    // ModuleInfo Tests
    // =========================================================================
    
    #[test]
    fn test_module_info_builder() {
        let info = ModuleInfo::new("my-module")
            .version(2, 1, 0)
            .description("A test module")
            .author("Test Author")
            .license("Apache-2.0")
            .flags(ModuleFlags::HOT_RELOADABLE)
            .provides(&["service.a", "service.b"]);
        
        assert_eq!(info.name, "my-module");
        assert_eq!(info.version.major, 2);
        assert_eq!(info.version.minor, 1);
        assert_eq!(info.version.patch, 0);
        assert_eq!(info.description, "A test module");
        assert_eq!(info.author, "Test Author");
        assert_eq!(info.license, "Apache-2.0");
        assert!(info.flags.contains(ModuleFlags::HOT_RELOADABLE));
        assert_eq!(info.provides.len(), 2);
    }
    
    #[test]
    fn test_module_info_defaults() {
        let info = ModuleInfo::new("simple");
        
        assert_eq!(info.name, "simple");
        assert_eq!(info.version.major, 0);
        assert_eq!(info.version.minor, 1);
        assert_eq!(info.version.patch, 0);
        assert_eq!(info.license, "MIT");
        assert!(info.flags.is_empty());
    }
    
    // =========================================================================
    // Response Tests
    // =========================================================================
    
    #[test]
    fn test_response_ok() {
        let resp = Response::ok(vec![1, 2, 3]);
        assert!(resp.success);
        assert_eq!(resp.payload, vec![1, 2, 3]);
        assert!(resp.error.is_none());
    }
    
    #[test]
    fn test_response_err() {
        let resp = Response::err("Something failed");
        assert!(!resp.success);
        assert!(resp.payload.is_empty());
        assert_eq!(resp.error, Some(String::from("Something failed")));
    }
    
    #[test]
    fn test_response_ok_empty() {
        let resp = Response::ok_empty();
        assert!(resp.success);
        assert!(resp.payload.is_empty());
    }
    
    // =========================================================================
    // Event Tests
    // =========================================================================
    
    #[test]
    fn test_event_tick() {
        let event = Event::Tick { timestamp_ns: 1000000 };
        match event {
            Event::Tick { timestamp_ns } => assert_eq!(timestamp_ns, 1000000),
            _ => panic!("Wrong event type"),
        }
    }
    
    #[test]
    fn test_event_memory_pressure() {
        let event = Event::MemoryPressure { level: MemoryPressureLevel::Low };
        match event {
            Event::MemoryPressure { level } => assert_eq!(level, MemoryPressureLevel::Low),
            _ => panic!("Wrong event type"),
        }
    }
    
    // =========================================================================
    // Module Lifecycle Tests
    // =========================================================================
    
    #[test]
    fn test_module_lifecycle() {
        let mut module = TestModule::new();
        
        // Check initial state
        assert!(!module.initialized);
        assert!(!module.started);
        
        // Get info before init
        let info = module.info();
        assert_eq!(info.name, "test-module");
        
        // Create mock context
        let config_fn = |_: &str| -> Option<&str> { None };
        let request_fn = |_: &str, _: Request| -> Result<Response, ModuleError> {
            Ok(Response::ok_empty())
        };
        let ctx = Context::new(crate::ModuleId::new(), &config_fn, &request_fn);
        
        // Initialize
        module.init(&ctx).expect("Init should succeed");
        assert!(module.initialized);
        assert!(!module.started);
        assert!(module.is_healthy());
        
        // Start
        module.start().expect("Start should succeed");
        assert!(module.started);
        
        // Handle events
        let tick = Event::Tick { timestamp_ns: 0 };
        let response = module.handle_event(&tick);
        assert!(matches!(response, EventResponse::Handled));
        assert_eq!(module.events_received, 1);
        
        // Handle request
        let request = Request {
            source: "caller",
            request_type: String::from("ping"),
            payload: Vec::new(),
        };
        let response = module.handle_request(&request).expect("Request should succeed");
        assert!(response.success);
        assert_eq!(response.payload, b"pong");
        
        // Stop
        module.stop().expect("Stop should succeed");
        assert!(!module.started);
    }
    
    #[test]
    fn test_module_start_without_init_fails() {
        let mut module = TestModule::new();
        
        // Starting without init should fail
        let result = module.start();
        assert!(result.is_err());
    }
}
