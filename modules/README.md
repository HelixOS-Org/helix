# Module System v2

The Helix Module System v2 provides a unified, consistent API for creating kernel modules.

## Overview

The v2 API simplifies module development by:
- Using static metadata instead of heap-allocated strings
- Providing a consistent lifecycle: `info` → `init` → `start` → `stop`
- Supporting events and IPC requests out of the box
- Maintaining backward compatibility with v1 modules

## Quick Start

```rust
use helix_modules::v2::{ModuleTrait, ModuleInfo, Context, Event, EventResponse};
use helix_modules::{ModuleError, ModuleFlags};

pub struct MyModule {
    active: bool,
}

impl ModuleTrait for MyModule {
    fn info(&self) -> ModuleInfo {
        ModuleInfo::new("my-module")
            .version(1, 0, 0)
            .description("My custom module")
            .author("Your Name")
            .provides(&["my.service"])
    }

    fn init(&mut self, ctx: &Context) -> Result<(), ModuleError> {
        // Read configuration
        let timeout = ctx.config_usize("timeout").unwrap_or(1000);
        Ok(())
    }

    fn start(&mut self) -> Result<(), ModuleError> {
        self.active = true;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), ModuleError> {
        self.active = false;
        Ok(())
    }

    fn handle_event(&mut self, event: &Event) -> EventResponse {
        match event {
            Event::Tick { .. } => {
                // Handle timer tick
                EventResponse::Handled
            }
            _ => EventResponse::Ignored,
        }
    }
}
```

## Module Lifecycle

```
┌─────────────────────────────────────────────────────────────────────┐
│                         MODULE LIFECYCLE                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐     │
│   │  Created │───▶│   Init   │───▶│  Start   │───▶│ Running  │     │
│   └──────────┘    └──────────┘    └──────────┘    └──────────┘     │
│        │                                               │            │
│        │                                               ▼            │
│        │                              ┌──────────┐◀─── Stop         │
│        │                              │ Stopped  │                  │
│        │                              └──────────┘                  │
│        │                                   │                        │
│        ▼                                   ▼                        │
│   ┌──────────────────────────────────────────────────────────┐     │
│   │                      Cleanup/Drop                         │     │
│   └──────────────────────────────────────────────────────────┘     │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## API Reference

### `ModuleInfo`

Static metadata about a module.

```rust
ModuleInfo::new("module-name")
    .version(major, minor, patch)  // Semantic version
    .description("What this module does")
    .author("Author Name")
    .license("MIT")
    .flags(ModuleFlags::ESSENTIAL | ModuleFlags::HOT_RELOADABLE)
    .dependencies(&["dep1", "dep2"])
    .provides(&["capability1", "capability2"])
```

### `Context`

Provided to modules during initialization.

```rust
fn init(&mut self, ctx: &Context) -> Result<(), ModuleError> {
    // Get module's assigned ID
    let id = ctx.id;
    
    // Read configuration
    let value = ctx.config("key");              // Option<&str>
    let value = ctx.config_or("key", "default"); // &str
    let value = ctx.config_usize("count");       // Option<usize>
    
    // Request service from another module
    let response = ctx.request("other-module", Request { ... })?;
    
    Ok(())
}
```

### Events

System events that modules can handle:

| Event | Description |
|-------|-------------|
| `Tick { timestamp_ns }` | Timer tick (periodic) |
| `Shutdown` | System is shutting down |
| `MemoryPressure { level }` | Memory is running low |
| `CpuHotplug { cpu_id, online }` | CPU added/removed |
| `Custom { name, data }` | User-defined event |

### Requests & Responses

IPC between modules:

```rust
// Sending a request
let request = Request {
    source: "my-module",
    request_type: String::from("get_status"),
    payload: vec![],
};
let response = ctx.request("target-module", request)?;

// Handling a request
fn handle_request(&mut self, request: &Request) -> Result<Response, ModuleError> {
    match request.request_type.as_str() {
        "get_status" => Ok(Response::ok(b"OK".to_vec())),
        _ => Ok(Response::err("Unknown request")),
    }
}
```

## Flags

| Flag | Description |
|------|-------------|
| `ESSENTIAL` | Cannot be unloaded |
| `HOT_RELOADABLE` | Supports hot-reload |
| `USERSPACE` | Runs in user space |
| `DRIVER` | Hardware driver |
| `FILESYSTEM` | Provides filesystem |
| `SCHEDULER` | Provides scheduling |
| `ALLOCATOR` | Provides memory allocation |
| `SECURITY` | Security module |

## Best Practices

1. **Keep `info()` simple** - Return static data, don't allocate
2. **Validate in `init()`** - Check configuration and dependencies
3. **Handle events efficiently** - Return `Ignored` for irrelevant events
4. **Clean up in `stop()`** - Release all resources
5. **Implement `is_healthy()`** - For monitoring and self-healing

## Migration from v1

If using the old `Module` trait, use `ModuleAdapter`:

```rust
use helix_modules::v2::ModuleAdapter;

let v2_module = MyV2Module::new();
let adapted: Box<dyn Module> = Box::new(ModuleAdapter::new(v2_module));
```

## Example: Scheduler Module

See [modules_impl/schedulers/round_robin/](../modules_impl/schedulers/round_robin/) for a complete example of a scheduler module using the v2 API.
