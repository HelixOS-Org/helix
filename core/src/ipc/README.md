# Helix IPC System

Inter-Process Communication infrastructure for the Helix kernel, enabling modules to communicate through events and messages.

## Components

### 1. Event Bus (`event_bus.rs`)

A publish-subscribe system for broadcasting events to multiple subscribers.

```rust
use helix_core::ipc::{
    EventBus, EventSubscription, EventTopic, Event, EventResponse,
    global_event_bus, subscribe, publish_event,
};

// Subscribe to tick events
let subscription = EventSubscription::new(
    "my_module",
    vec![EventTopic::Tick],
    Box::new(|event| {
        if let Event::Tick { timestamp_ns, tick_number } = event {
            log::info!("Tick {} at {}ns", tick_number, timestamp_ns);
        }
        EventResponse::Handled
    }),
);

let sub_id = subscribe(subscription);

// Publish an event
let result = publish_event(Event::Tick {
    timestamp_ns: 1_000_000,
    tick_number: 1,
});
log::info!("Delivered to {} handlers", result.handled);
```

#### Event Topics

| Topic | Description |
|-------|-------------|
| `Tick` | Timer tick events |
| `Shutdown` | System shutdown notification |
| `MemoryPressure` | Memory pressure alerts |
| `CpuHotplug` | CPU online/offline events |
| `Process` | Process lifecycle events |
| `Interrupt` | Hardware interrupt events |
| `Custom(name)` | User-defined events |
| `All` | Receive all events |

#### Subscription Priority

Subscribers can have priorities that determine delivery order:

```rust
use helix_core::ipc::SubscriptionPriority;

let sub = EventSubscription::new("critical", topics, handler)
    .with_priority(SubscriptionPriority::HIGHEST);  // Runs first
```

### 2. Message Router (`message_router.rs`)

Point-to-point messaging for direct module communication with request/response pattern.

```rust
use helix_core::ipc::{
    MessageRouter, Request, Response, ModuleAddress,
    global_router, send_request, send_to,
};

// Register a module with the router
global_router().register(
    1, // module ID
    "scheduler",
    Box::new(|request: &Request| {
        match request.request_type.as_str() {
            "get_stats" => {
                Response::success(b"running=5,waiting=10".to_vec())
            }
            "pause" => {
                Response::ok()
            }
            _ => Response::NotSupported,
        }
    }),
)?;

// Send a request
let request = Request::new(
    ModuleAddress::Name("client".into()),
    "get_stats",
);
let response = send_to("scheduler", request)?;

match response {
    Response::Success(data) => log::info!("Got: {:?}", data),
    Response::Error(msg) => log::error!("Error: {}", msg),
    _ => {}
}
```

#### Message Priority

Messages can have different priorities:

```rust
use helix_core::ipc::MessagePriority;

let request = Request::new(from, "shutdown")
    .with_priority(MessagePriority::Critical);
```

### 3. Channels (`channel.rs`)

Low-level bounded channels for data transfer.

```rust
use helix_core::ipc::{channel, default_channel, oneshot};

// Create a bounded channel
let (tx, rx) = channel::<u32>(16);

// Send data
tx.send(42)?;
tx.send(100)?;

// Receive data
while let Ok(value) = rx.try_recv() {
    log::info!("Received: {}", value);
}

// One-shot channel for single value
let (tx, rx) = oneshot::<String>();
tx.send("done".to_string())?;
let result = rx.try_recv()?;
```

## Integration with Module System

Modules can use IPC through the `ModuleTrait`:

```rust
use helix_modules::v2::{ModuleTrait, ModuleResult, Context};
use helix_core::ipc::{
    Event as IpcEvent, EventResponse as IpcEventResponse,
    Request as IpcRequest, Response as IpcResponse,
    global_event_bus, EventSubscription, EventTopic,
    global_router,
};

pub struct MyModule;

impl ModuleTrait for MyModule {
    fn init(&self, ctx: &Context) -> ModuleResult<()> {
        // Subscribe to events
        let sub = EventSubscription::new(
            "my_module",
            vec![EventTopic::Tick, EventTopic::Shutdown],
            Box::new(|event| {
                // Handle events
                IpcEventResponse::Handled
            }),
        );
        global_event_bus().subscribe(sub);
        
        // Register with message router
        global_router().register(
            ctx.module_id().as_u64(),
            "my_module",
            Box::new(|req| IpcResponse::ok()),
        ).ok();
        
        Ok(())
    }

    fn handle_event(&self, event: &crate::Event) -> EventResponse {
        // Module-level event handling
        EventResponse::Ignored
    }

    fn handle_request(&self, request: &crate::Request) -> crate::Response {
        // Handle IPC requests
        crate::Response::NotSupported
    }
}
```

## Error Handling

```rust
use helix_core::ipc::{IpcError, IpcResult};

fn example() -> IpcResult<()> {
    match send_to("unknown", request) {
        Ok(response) => Ok(()),
        Err(IpcError::ModuleNotFound) => {
            log::warn!("Module not registered");
            Ok(())
        }
        Err(IpcError::ChannelFull) => {
            // Retry later
            Err(IpcError::ChannelFull)
        }
        Err(e) => Err(e),
    }
}
```

## Thread Safety

All IPC components are thread-safe:

- `EventBus` uses `RwLock` for subscriptions
- `MessageRouter` uses `RwLock` for module registry
- `Channel` uses `Mutex` with atomic counters
- Handlers must be `Send + Sync`

## Performance Considerations

1. **Event Bus**: O(n) dispatch to all matching subscribers
2. **Message Router**: O(log n) lookup by ID, O(log n) by name
3. **Channels**: O(1) send/receive operations

For high-frequency events (like timer ticks), consider:
- Using priorities to ensure critical handlers run first
- Filtering at the topic level, not in handlers
- Using direct channels for high-throughput data paths
