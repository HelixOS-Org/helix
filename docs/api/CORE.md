# Helix Core API Reference

<div align="center">

⚙️ **Complete Core Subsystem API Documentation**

*IPC, Orchestrator, Interrupts, Syscalls, and Hot-Reload*

</div>

---

## Table of Contents

1. [Overview](#1-overview)
2. [IPC System](#2-ipc-system)
3. [Orchestrator](#3-orchestrator)
4. [Interrupt Handling](#4-interrupt-handling)
5. [Syscall Gateway](#5-syscall-gateway)
6. [Hot-Reload System](#6-hot-reload-system)
7. [Debug Console](#7-debug-console)
8. [Self-Healing](#8-self-healing)
9. [Error Types](#9-error-types)

---

## 1. Overview

### 1.1 Core Crate Structure

```
helix-core/
├── src/
│   ├── lib.rs                 # Crate root
│   ├── selfheal.rs            # Self-healing mechanisms
│   ├── debug/
│   │   ├── mod.rs             # Debug module
│   │   └── console.rs         # Serial console
│   ├── hotreload/
│   │   ├── mod.rs             # Hot-reload engine
│   │   ├── crasher.rs         # Crash simulation (testing)
│   │   └── schedulers.rs      # Scheduler hot-reload
│   ├── interrupts/
│   │   ├── mod.rs             # Interrupt system
│   │   ├── exceptions.rs      # CPU exceptions
│   │   ├── handlers.rs        # Interrupt handlers
│   │   └── router.rs          # Interrupt routing
│   ├── ipc/
│   │   ├── mod.rs             # IPC system
│   │   ├── channel.rs         # Channels
│   │   ├── event_bus.rs       # Event bus
│   │   └── message_router.rs  # Message routing
│   ├── orchestrator/
│   │   ├── mod.rs             # Orchestrator
│   │   ├── capability_broker.rs
│   │   ├── lifecycle.rs       # Module lifecycle
│   │   ├── panic_handler.rs   # Panic handling
│   │   └── resource_broker.rs # Resource management
│   └── syscall/
│       ├── mod.rs             # Syscall gateway
│       ├── dispatcher.rs      # Syscall dispatch
│       └── gateway.rs         # Syscall entry
```

### 1.2 Dependencies

```toml
[dependencies]
helix-hal = { path = "../hal" }
helix-modules = { path = "../modules" }
spin = "0.9"
bitflags = "2.0"
```

---

## 2. IPC System

### 2.1 IPC Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         HELIX IPC ARCHITECTURE                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  IPC Methods:                                                               │
│  ═══════════                                                                │
│                                                                             │
│  ┌────────────────────┐  ┌────────────────────┐  ┌────────────────────┐    │
│  │     Channels       │  │     Event Bus      │  │   Message Router   │    │
│  │  (Point-to-Point)  │  │  (Pub/Sub)         │  │  (Request/Reply)   │    │
│  └────────────────────┘  └────────────────────┘  └────────────────────┘    │
│           │                       │                        │               │
│           │                       │                        │               │
│           └───────────────────────┴────────────────────────┘               │
│                                   │                                         │
│                                   ▼                                         │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                         IPC Core                                     │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │  │
│  │  │ Zero-Copy   │  │   Message   │  │  Capability │                  │  │
│  │  │  Transfer   │  │   Queuing   │  │   Checks    │                  │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                  │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Message Types

```rust
/// IPC message
#[derive(Debug, Clone)]
pub struct Message {
    /// Source module/process ID
    pub source: ProcessId,
    
    /// Destination module/process ID
    pub destination: ProcessId,
    
    /// Message type
    pub msg_type: MessageType,
    
    /// Message payload
    pub payload: MessagePayload,
    
    /// Timestamp
    pub timestamp: u64,
    
    /// Priority (0 = lowest, 255 = highest)
    pub priority: u8,
}

/// Message types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    /// Request expecting a reply
    Request,
    
    /// Reply to a request
    Reply,
    
    /// One-way notification
    Notification,
    
    /// Error message
    Error,
    
    /// System message (kernel internal)
    System,
    
    /// Event publication
    Event,
}

/// Message payload
#[derive(Debug, Clone)]
pub enum MessagePayload {
    /// Empty message
    Empty,
    
    /// Small inline data (up to 64 bytes)
    Inline(InlineData),
    
    /// Reference to shared memory
    SharedMemory {
        address: usize,
        size: usize,
        permissions: u32,
    },
    
    /// Capability transfer
    Capability(CapabilityId),
    
    /// File descriptor
    FileDescriptor(u32),
}

/// Inline data for small messages
#[derive(Debug, Clone)]
pub struct InlineData {
    data: [u8; 64],
    len: usize,
}

impl InlineData {
    pub fn new(data: &[u8]) -> Option<Self> {
        if data.len() > 64 {
            return None;
        }
        let mut inline = Self {
            data: [0u8; 64],
            len: data.len(),
        };
        inline.data[..data.len()].copy_from_slice(data);
        Some(inline)
    }
    
    pub fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len]
    }
}
```

### 2.3 Channel API

```rust
/// Bidirectional communication channel
pub struct Channel {
    /// Channel identifier
    id: ChannelId,
    
    /// Sender endpoint
    sender: Sender,
    
    /// Receiver endpoint
    receiver: Receiver,
    
    /// Channel capacity
    capacity: usize,
    
    /// Current message count
    count: AtomicUsize,
    
    /// Channel state
    state: AtomicU8,
}

/// Channel creation result
pub type ChannelResult<T> = Result<T, ChannelError>;

impl Channel {
    /// Create a new channel with given capacity
    pub fn new(capacity: usize) -> ChannelResult<Self> {
        if capacity == 0 {
            return Err(ChannelError::InvalidCapacity);
        }
        
        let id = ChannelId::generate();
        
        Ok(Self {
            id,
            sender: Sender::new(id),
            receiver: Receiver::new(id),
            capacity,
            count: AtomicUsize::new(0),
            state: AtomicU8::new(ChannelState::Open as u8),
        })
    }
    
    /// Get the sender endpoint
    pub fn sender(&self) -> Sender {
        self.sender.clone()
    }
    
    /// Get the receiver endpoint
    pub fn receiver(&self) -> Receiver {
        self.receiver.clone()
    }
    
    /// Check if channel is open
    pub fn is_open(&self) -> bool {
        self.state.load(Ordering::SeqCst) == ChannelState::Open as u8
    }
    
    /// Close the channel
    pub fn close(&self) {
        self.state.store(ChannelState::Closed as u8, Ordering::SeqCst);
    }
    
    /// Get current message count
    pub fn message_count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

/// Sender endpoint
#[derive(Clone)]
pub struct Sender {
    channel_id: ChannelId,
    // Internal state...
}

impl Sender {
    /// Send a message (blocking)
    pub fn send(&self, msg: Message) -> ChannelResult<()> {
        if !self.is_open() {
            return Err(ChannelError::Closed);
        }
        
        // Queue message
        self.queue_message(msg)?;
        
        Ok(())
    }
    
    /// Try to send without blocking
    pub fn try_send(&self, msg: Message) -> ChannelResult<()> {
        if !self.is_open() {
            return Err(ChannelError::Closed);
        }
        
        if self.is_full() {
            return Err(ChannelError::Full);
        }
        
        self.queue_message(msg)?;
        
        Ok(())
    }
    
    /// Send with timeout (in ticks)
    pub fn send_timeout(&self, msg: Message, timeout: u64) -> ChannelResult<()> {
        let deadline = current_tick() + timeout;
        
        loop {
            match self.try_send(msg.clone()) {
                Ok(()) => return Ok(()),
                Err(ChannelError::Full) => {
                    if current_tick() > deadline {
                        return Err(ChannelError::Timeout);
                    }
                    yield_now();
                }
                Err(e) => return Err(e),
            }
        }
    }
}

/// Receiver endpoint
#[derive(Clone)]
pub struct Receiver {
    channel_id: ChannelId,
    // Internal state...
}

impl Receiver {
    /// Receive a message (blocking)
    pub fn recv(&self) -> ChannelResult<Message> {
        loop {
            match self.try_recv() {
                Ok(msg) => return Ok(msg),
                Err(ChannelError::Empty) => {
                    if !self.is_open() {
                        return Err(ChannelError::Closed);
                    }
                    yield_now();
                }
                Err(e) => return Err(e),
            }
        }
    }
    
    /// Try to receive without blocking
    pub fn try_recv(&self) -> ChannelResult<Message> {
        if let Some(msg) = self.dequeue_message() {
            Ok(msg)
        } else if !self.is_open() {
            Err(ChannelError::Closed)
        } else {
            Err(ChannelError::Empty)
        }
    }
    
    /// Receive with timeout
    pub fn recv_timeout(&self, timeout: u64) -> ChannelResult<Message> {
        let deadline = current_tick() + timeout;
        
        loop {
            match self.try_recv() {
                Ok(msg) => return Ok(msg),
                Err(ChannelError::Empty) => {
                    if current_tick() > deadline {
                        return Err(ChannelError::Timeout);
                    }
                    yield_now();
                }
                Err(e) => return Err(e),
            }
        }
    }
    
    /// Check if messages are available
    pub fn is_ready(&self) -> bool {
        self.message_count() > 0
    }
}
```

### 2.4 Event Bus API

```rust
/// Event bus for publish/subscribe messaging
pub struct EventBus {
    /// Registered topics
    topics: Mutex<HashMap<TopicId, Topic>>,
    
    /// Subscriber registry
    subscribers: Mutex<HashMap<ProcessId, Vec<TopicId>>>,
    
    /// Event counter
    event_count: AtomicU64,
}

/// A topic for events
pub struct Topic {
    /// Topic identifier
    id: TopicId,
    
    /// Topic name
    name: &'static str,
    
    /// Subscribers
    subscribers: Vec<Subscriber>,
    
    /// Event history (ring buffer)
    history: RingBuffer<Event>,
}

/// Event structure
#[derive(Debug, Clone)]
pub struct Event {
    /// Event type
    pub event_type: EventType,
    
    /// Event data
    pub data: EventData,
    
    /// Source
    pub source: ProcessId,
    
    /// Timestamp
    pub timestamp: u64,
}

/// Event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventType {
    /// Module loaded
    ModuleLoaded,
    
    /// Module unloaded
    ModuleUnloaded,
    
    /// Process created
    ProcessCreated,
    
    /// Process exited
    ProcessExited,
    
    /// Interrupt received
    Interrupt,
    
    /// Timer tick
    TimerTick,
    
    /// Memory pressure
    MemoryPressure,
    
    /// Custom event
    Custom(u32),
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        Self {
            topics: Mutex::new(HashMap::new()),
            subscribers: Mutex::new(HashMap::new()),
            event_count: AtomicU64::new(0),
        }
    }
    
    /// Create a new topic
    pub fn create_topic(&self, name: &'static str) -> TopicId {
        let id = TopicId::generate();
        let topic = Topic {
            id,
            name,
            subscribers: Vec::new(),
            history: RingBuffer::new(64),
        };
        
        self.topics.lock().insert(id, topic);
        id
    }
    
    /// Subscribe to a topic
    pub fn subscribe(
        &self,
        topic: TopicId,
        callback: EventCallback,
    ) -> SubscriptionId {
        let subscriber = Subscriber {
            id: SubscriptionId::generate(),
            callback,
            filter: None,
        };
        
        if let Some(topic) = self.topics.lock().get_mut(&topic) {
            topic.subscribers.push(subscriber.clone());
        }
        
        subscriber.id
    }
    
    /// Subscribe with filter
    pub fn subscribe_filtered(
        &self,
        topic: TopicId,
        callback: EventCallback,
        filter: EventFilter,
    ) -> SubscriptionId {
        let subscriber = Subscriber {
            id: SubscriptionId::generate(),
            callback,
            filter: Some(filter),
        };
        
        if let Some(topic) = self.topics.lock().get_mut(&topic) {
            topic.subscribers.push(subscriber.clone());
        }
        
        subscriber.id
    }
    
    /// Unsubscribe from a topic
    pub fn unsubscribe(&self, subscription: SubscriptionId) {
        for topic in self.topics.lock().values_mut() {
            topic.subscribers.retain(|s| s.id != subscription);
        }
    }
    
    /// Publish an event to a topic
    pub fn publish(&self, topic: TopicId, event: Event) {
        self.event_count.fetch_add(1, Ordering::Relaxed);
        
        if let Some(topic) = self.topics.lock().get_mut(&topic) {
            // Add to history
            topic.history.push(event.clone());
            
            // Notify subscribers
            for subscriber in &topic.subscribers {
                // Apply filter if present
                if let Some(ref filter) = subscriber.filter {
                    if !filter.matches(&event) {
                        continue;
                    }
                }
                
                // Invoke callback
                (subscriber.callback)(event.clone());
            }
        }
    }
    
    /// Get event count
    pub fn event_count(&self) -> u64 {
        self.event_count.load(Ordering::Relaxed)
    }
}

/// Event callback function type
pub type EventCallback = fn(Event);

/// Event filter
pub struct EventFilter {
    pub event_types: Option<Vec<EventType>>,
    pub sources: Option<Vec<ProcessId>>,
}

impl EventFilter {
    pub fn matches(&self, event: &Event) -> bool {
        if let Some(ref types) = self.event_types {
            if !types.contains(&event.event_type) {
                return false;
            }
        }
        
        if let Some(ref sources) = self.sources {
            if !sources.contains(&event.source) {
                return false;
            }
        }
        
        true
    }
}
```

---

## 3. Orchestrator

### 3.1 Orchestrator Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            ORCHESTRATOR                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  The Orchestrator manages:                                                  │
│  ═════════════════════════                                                  │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │                                                                       │ │
│  │     ┌─────────────────┐    ┌─────────────────┐    ┌───────────────┐  │ │
│  │     │   Capability    │    │    Resource     │    │   Lifecycle   │  │ │
│  │     │     Broker      │    │     Broker      │    │    Manager    │  │ │
│  │     └────────┬────────┘    └────────┬────────┘    └───────┬───────┘  │ │
│  │              │                      │                      │          │ │
│  │              └──────────────────────┼──────────────────────┘          │ │
│  │                                     │                                 │ │
│  │                                     ▼                                 │ │
│  │              ┌──────────────────────────────────────┐                │ │
│  │              │           ORCHESTRATOR               │                │ │
│  │              │                                      │                │ │
│  │              │  • Module loading/unloading          │                │ │
│  │              │  • Capability granting/revocation    │                │ │
│  │              │  • Resource allocation               │                │ │
│  │              │  • Panic handling                    │                │ │
│  │              │  • System recovery                   │                │ │
│  │              └──────────────────────────────────────┘                │ │
│  │                                                                       │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Lifecycle Management

```rust
/// Module lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    /// Module is being loaded
    Loading,
    
    /// Module loaded, not yet initialized
    Loaded,
    
    /// Module is initializing
    Initializing,
    
    /// Module is ready and running
    Running,
    
    /// Module is paused
    Paused,
    
    /// Module is being stopped
    Stopping,
    
    /// Module is stopped
    Stopped,
    
    /// Module is being unloaded
    Unloading,
    
    /// Module encountered an error
    Error(LifecycleError),
}

/// Lifecycle manager
pub struct LifecycleManager {
    /// Module states
    states: Mutex<HashMap<ModuleId, LifecycleState>>,
    
    /// State transition callbacks
    callbacks: Mutex<Vec<StateCallback>>,
    
    /// Event bus for lifecycle events
    event_bus: &'static EventBus,
}

impl LifecycleManager {
    /// Create new lifecycle manager
    pub fn new(event_bus: &'static EventBus) -> Self {
        Self {
            states: Mutex::new(HashMap::new()),
            callbacks: Mutex::new(Vec::new()),
            event_bus,
        }
    }
    
    /// Get module state
    pub fn get_state(&self, module: ModuleId) -> Option<LifecycleState> {
        self.states.lock().get(&module).copied()
    }
    
    /// Transition module to new state
    pub fn transition(
        &self,
        module: ModuleId,
        new_state: LifecycleState,
    ) -> Result<(), LifecycleError> {
        let mut states = self.states.lock();
        let old_state = states.get(&module).copied();
        
        // Validate transition
        if !self.is_valid_transition(old_state, new_state) {
            return Err(LifecycleError::InvalidTransition);
        }
        
        // Update state
        states.insert(module, new_state);
        
        // Notify callbacks
        drop(states); // Release lock before callbacks
        self.notify_callbacks(module, old_state, new_state);
        
        // Publish event
        self.publish_lifecycle_event(module, old_state, new_state);
        
        Ok(())
    }
    
    /// Check if transition is valid
    fn is_valid_transition(
        &self,
        from: Option<LifecycleState>,
        to: LifecycleState,
    ) -> bool {
        use LifecycleState::*;
        
        match (from, to) {
            // Initial load
            (None, Loading) => true,
            
            // Normal progression
            (Some(Loading), Loaded) => true,
            (Some(Loaded), Initializing) => true,
            (Some(Initializing), Running) => true,
            (Some(Running), Paused) => true,
            (Some(Paused), Running) => true,
            (Some(Running), Stopping) => true,
            (Some(Paused), Stopping) => true,
            (Some(Stopping), Stopped) => true,
            (Some(Stopped), Unloading) => true,
            
            // Error transitions
            (Some(_), Error(_)) => true,
            (Some(Error(_)), Unloading) => true,
            
            // Invalid
            _ => false,
        }
    }
    
    /// Register state callback
    pub fn on_state_change(&self, callback: StateCallback) {
        self.callbacks.lock().push(callback);
    }
    
    /// Notify all callbacks
    fn notify_callbacks(
        &self,
        module: ModuleId,
        old: Option<LifecycleState>,
        new: LifecycleState,
    ) {
        for callback in self.callbacks.lock().iter() {
            callback(module, old, new);
        }
    }
    
    /// Load a module
    pub fn load_module(&self, module: ModuleId) -> Result<(), LifecycleError> {
        self.transition(module, LifecycleState::Loading)?;
        
        // Perform actual loading...
        // ...
        
        self.transition(module, LifecycleState::Loaded)?;
        self.transition(module, LifecycleState::Initializing)?;
        
        // Initialize...
        // ...
        
        self.transition(module, LifecycleState::Running)?;
        
        Ok(())
    }
    
    /// Unload a module
    pub fn unload_module(&self, module: ModuleId) -> Result<(), LifecycleError> {
        self.transition(module, LifecycleState::Stopping)?;
        
        // Stop module...
        // ...
        
        self.transition(module, LifecycleState::Stopped)?;
        self.transition(module, LifecycleState::Unloading)?;
        
        // Remove from states
        self.states.lock().remove(&module);
        
        Ok(())
    }
}

/// State change callback
pub type StateCallback = fn(ModuleId, Option<LifecycleState>, LifecycleState);
```

### 3.3 Capability Broker

```rust
/// Capability identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilityId(u64);

/// Capability types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// Memory access
    Memory {
        read: bool,
        write: bool,
        execute: bool,
    },
    
    /// I/O port access
    IoPort {
        start: u16,
        end: u16,
    },
    
    /// Interrupt handling
    Interrupt {
        vector: u8,
    },
    
    /// File system access
    FileSystem {
        path: &'static str,
        read: bool,
        write: bool,
    },
    
    /// Network access
    Network {
        port: u16,
    },
    
    /// Module management
    ModuleManagement,
    
    /// System configuration
    SystemConfig,
    
    /// Debug access
    Debug,
}

/// Capability broker manages capability grants
pub struct CapabilityBroker {
    /// Granted capabilities
    grants: Mutex<HashMap<ModuleId, Vec<CapabilityGrant>>>,
    
    /// Capability policies
    policies: Vec<CapabilityPolicy>,
}

/// A capability grant
#[derive(Debug, Clone)]
pub struct CapabilityGrant {
    /// Capability ID
    pub id: CapabilityId,
    
    /// The capability
    pub capability: Capability,
    
    /// Grantee module
    pub grantee: ModuleId,
    
    /// Grant time
    pub granted_at: u64,
    
    /// Expiration (0 = never)
    pub expires_at: u64,
    
    /// Revocable by system
    pub revocable: bool,
}

impl CapabilityBroker {
    /// Create new capability broker
    pub fn new() -> Self {
        Self {
            grants: Mutex::new(HashMap::new()),
            policies: Vec::new(),
        }
    }
    
    /// Grant a capability to a module
    pub fn grant(
        &self,
        module: ModuleId,
        capability: Capability,
        duration: Option<u64>,
    ) -> Result<CapabilityId, CapabilityError> {
        // Check policies
        for policy in &self.policies {
            if !policy.allows(&capability, module) {
                return Err(CapabilityError::PolicyDenied);
            }
        }
        
        let grant = CapabilityGrant {
            id: CapabilityId::generate(),
            capability,
            grantee: module,
            granted_at: current_tick(),
            expires_at: duration.map(|d| current_tick() + d).unwrap_or(0),
            revocable: true,
        };
        
        let id = grant.id;
        self.grants.lock().entry(module).or_default().push(grant);
        
        Ok(id)
    }
    
    /// Revoke a capability
    pub fn revoke(&self, id: CapabilityId) -> Result<(), CapabilityError> {
        for grants in self.grants.lock().values_mut() {
            if let Some(pos) = grants.iter().position(|g| g.id == id) {
                if !grants[pos].revocable {
                    return Err(CapabilityError::NotRevocable);
                }
                grants.remove(pos);
                return Ok(());
            }
        }
        
        Err(CapabilityError::NotFound)
    }
    
    /// Check if module has capability
    pub fn has_capability(
        &self,
        module: ModuleId,
        capability: &Capability,
    ) -> bool {
        let grants = self.grants.lock();
        
        if let Some(module_grants) = grants.get(&module) {
            for grant in module_grants {
                if self.capability_matches(&grant.capability, capability) {
                    // Check expiration
                    if grant.expires_at != 0 && current_tick() > grant.expires_at {
                        continue; // Expired
                    }
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Check if capabilities match
    fn capability_matches(&self, granted: &Capability, requested: &Capability) -> bool {
        use Capability::*;
        
        match (granted, requested) {
            (Memory { read: gr, write: gw, execute: ge },
             Memory { read: rr, write: rw, execute: re }) => {
                (!rr || *gr) && (!rw || *gw) && (!re || *ge)
            }
            (IoPort { start: gs, end: ge }, IoPort { start: rs, end: re }) => {
                *gs <= *rs && *ge >= *re
            }
            (Interrupt { vector: gv }, Interrupt { vector: rv }) => gv == rv,
            (ModuleManagement, ModuleManagement) => true,
            (SystemConfig, SystemConfig) => true,
            (Debug, Debug) => true,
            _ => false,
        }
    }
    
    /// List capabilities for a module
    pub fn list_capabilities(&self, module: ModuleId) -> Vec<CapabilityGrant> {
        self.grants
            .lock()
            .get(&module)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Add a policy
    pub fn add_policy(&mut self, policy: CapabilityPolicy) {
        self.policies.push(policy);
    }
}

/// Capability policy
pub struct CapabilityPolicy {
    pub name: &'static str,
    pub check: fn(&Capability, ModuleId) -> bool,
}

impl CapabilityPolicy {
    pub fn allows(&self, cap: &Capability, module: ModuleId) -> bool {
        (self.check)(cap, module)
    }
}
```

### 3.4 Resource Broker

```rust
/// Resource types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resource {
    /// Memory (in bytes)
    Memory(usize),
    
    /// CPU time quota (ticks per second)
    CpuTime(u32),
    
    /// I/O bandwidth (bytes per second)
    IoBandwidth(u64),
    
    /// Open file handles
    FileHandles(u32),
    
    /// Network connections
    NetworkConnections(u32),
    
    /// Custom resource
    Custom { name: &'static str, value: u64 },
}

/// Resource allocation
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    pub resource: Resource,
    pub allocated: u64,
    pub used: u64,
    pub holder: ModuleId,
}

/// Resource broker manages resource allocation
pub struct ResourceBroker {
    /// Resource pools
    pools: Mutex<HashMap<ResourceType, ResourcePool>>,
    
    /// Allocations per module
    allocations: Mutex<HashMap<ModuleId, Vec<ResourceAllocation>>>,
}

impl ResourceBroker {
    /// Create new resource broker
    pub fn new() -> Self {
        Self {
            pools: Mutex::new(HashMap::new()),
            allocations: Mutex::new(HashMap::new()),
        }
    }
    
    /// Initialize a resource pool
    pub fn init_pool(&self, resource_type: ResourceType, total: u64) {
        self.pools.lock().insert(
            resource_type,
            ResourcePool {
                total,
                available: total,
                allocations: Vec::new(),
            },
        );
    }
    
    /// Request resource allocation
    pub fn allocate(
        &self,
        module: ModuleId,
        resource: Resource,
        amount: u64,
    ) -> Result<AllocationId, ResourceError> {
        let resource_type = resource.resource_type();
        let mut pools = self.pools.lock();
        
        let pool = pools
            .get_mut(&resource_type)
            .ok_or(ResourceError::UnknownResource)?;
        
        if pool.available < amount {
            return Err(ResourceError::InsufficientResources);
        }
        
        pool.available -= amount;
        
        let allocation = ResourceAllocation {
            resource,
            allocated: amount,
            used: 0,
            holder: module,
        };
        
        let id = AllocationId::generate();
        pool.allocations.push((id, allocation.clone()));
        
        self.allocations
            .lock()
            .entry(module)
            .or_default()
            .push(allocation);
        
        Ok(id)
    }
    
    /// Release resource allocation
    pub fn deallocate(&self, id: AllocationId) -> Result<(), ResourceError> {
        let mut pools = self.pools.lock();
        
        for pool in pools.values_mut() {
            if let Some(pos) = pool.allocations.iter().position(|(aid, _)| *aid == id) {
                let (_, allocation) = pool.allocations.remove(pos);
                pool.available += allocation.allocated;
                return Ok(());
            }
        }
        
        Err(ResourceError::NotFound)
    }
    
    /// Get resource usage for a module
    pub fn get_usage(&self, module: ModuleId) -> Vec<ResourceAllocation> {
        self.allocations
            .lock()
            .get(&module)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Update usage counter
    pub fn update_usage(&self, id: AllocationId, used: u64) {
        let pools = self.pools.lock();
        
        for pool in pools.values() {
            if let Some((_, allocation)) = pool.allocations.iter().find(|(aid, _)| *aid == id) {
                // Note: This is simplified - would need mutable access
                // allocation.used = used;
            }
        }
    }
}

/// Resource pool
struct ResourcePool {
    total: u64,
    available: u64,
    allocations: Vec<(AllocationId, ResourceAllocation)>,
}
```

---

## 4. Interrupt Handling

### 4.1 Interrupt Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         INTERRUPT ARCHITECTURE                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  CPU Exception Flow:                                                        │
│  ══════════════════                                                         │
│                                                                             │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────────────────────┐  │
│  │ Hardware│───▶│  IDT    │───▶│ Handler │───▶│ Exception Dispatcher   │  │
│  │ Exception│   │ Lookup  │    │ Stub    │    │ (exception_handler)    │  │
│  └─────────┘    └─────────┘    └─────────┘    └───────────┬─────────────┘  │
│                                                           │                 │
│                                              ┌────────────┴────────────┐    │
│                                              ▼                         ▼    │
│                                    ┌─────────────────┐     ┌───────────────┐│
│                                    │ Recover/Handle │     │ Panic + Dump  ││
│                                    └─────────────────┘     └───────────────┘│
│                                                                             │
│  Hardware Interrupt Flow:                                                   │
│  ════════════════════════                                                   │
│                                                                             │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────────────────────┐  │
│  │ Device  │───▶│  APIC   │───▶│  IDT    │───▶│  Interrupt Router      │  │
│  │ IRQ     │    │         │    │ Vector  │    │                         │  │
│  └─────────┘    └─────────┘    └─────────┘    └───────────┬─────────────┘  │
│                                                           │                 │
│                                              ┌────────────┴────────────┐    │
│                                              ▼                         ▼    │
│                                    ┌─────────────────┐     ┌───────────────┐│
│                                    │ Module Handler  │     │ Default EOI   ││
│                                    └─────────────────┘     └───────────────┘│
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 4.2 Exception Handling

```rust
/// CPU exception types
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Exception {
    DivideError = 0,
    Debug = 1,
    NonMaskableInterrupt = 2,
    Breakpoint = 3,
    Overflow = 4,
    BoundRangeExceeded = 5,
    InvalidOpcode = 6,
    DeviceNotAvailable = 7,
    DoubleFault = 8,
    CoprocessorSegmentOverrun = 9,
    InvalidTss = 10,
    SegmentNotPresent = 11,
    StackSegmentFault = 12,
    GeneralProtectionFault = 13,
    PageFault = 14,
    // 15 reserved
    X87FloatingPoint = 16,
    AlignmentCheck = 17,
    MachineCheck = 18,
    SimdFloatingPoint = 19,
    VirtualizationException = 20,
    ControlProtection = 21,
    // 22-27 reserved
    HypervisorInjection = 28,
    VmmCommunication = 29,
    SecurityException = 30,
    // 31 reserved
}

/// Exception stack frame (pushed by CPU)
#[repr(C)]
pub struct ExceptionStackFrame {
    /// Instruction pointer
    pub rip: u64,
    /// Code segment
    pub cs: u64,
    /// CPU flags
    pub rflags: u64,
    /// Stack pointer
    pub rsp: u64,
    /// Stack segment
    pub ss: u64,
}

/// Page fault error code
bitflags! {
    pub struct PageFaultError: u64 {
        /// Page was present
        const PRESENT = 1 << 0;
        /// Write access caused fault
        const WRITE = 1 << 1;
        /// User mode access
        const USER = 1 << 2;
        /// Reserved bit was set
        const RESERVED = 1 << 3;
        /// Instruction fetch caused fault
        const INSTRUCTION_FETCH = 1 << 4;
        /// Protection key violation
        const PROTECTION_KEY = 1 << 5;
        /// Shadow stack access
        const SHADOW_STACK = 1 << 6;
    }
}

/// Handle exceptions
pub fn exception_handler(
    exception: Exception,
    frame: &ExceptionStackFrame,
    error_code: Option<u64>,
) {
    match exception {
        Exception::PageFault => {
            let cr2: u64;
            unsafe { core::arch::asm!("mov {}, cr2", out(reg) cr2) };
            
            let error = PageFaultError::from_bits_truncate(error_code.unwrap_or(0));
            
            serial_println!("[PAGE FAULT] at {:#x}", cr2);
            serial_println!("  Error: {:?}", error);
            serial_println!("  RIP: {:#x}", frame.rip);
            
            // Try to handle (e.g., demand paging)
            if !handle_page_fault(cr2, error) {
                panic!("Unhandled page fault at {:#x}", cr2);
            }
        }
        
        Exception::DoubleFault => {
            // Double fault - unrecoverable
            serial_println!("[DOUBLE FAULT] - FATAL");
            serial_println!("  RIP: {:#x}", frame.rip);
            serial_println!("  RSP: {:#x}", frame.rsp);
            
            // Halt
            loop {
                unsafe { core::arch::asm!("hlt") };
            }
        }
        
        Exception::GeneralProtectionFault => {
            serial_println!("[GPF] at {:#x}", frame.rip);
            serial_println!("  Error code: {:#x}", error_code.unwrap_or(0));
            
            panic!("General Protection Fault at {:#x}", frame.rip);
        }
        
        Exception::Breakpoint => {
            serial_println!("[BREAKPOINT] at {:#x}", frame.rip);
            // Continue execution
        }
        
        _ => {
            serial_println!("[EXCEPTION] {:?} at {:#x}", exception, frame.rip);
            if let Some(code) = error_code {
                serial_println!("  Error code: {:#x}", code);
            }
            
            panic!("Unhandled exception: {:?}", exception);
        }
    }
}
```

### 4.3 Interrupt Router

```rust
/// Interrupt handler function type
pub type InterruptHandler = fn(vector: u8, data: *mut ());

/// Interrupt router for dynamic handler registration
pub struct InterruptRouter {
    /// Registered handlers
    handlers: Mutex<[Option<HandlerEntry>; 256]>,
    
    /// Interrupt statistics
    stats: [AtomicU64; 256],
}

/// Handler entry
struct HandlerEntry {
    handler: InterruptHandler,
    data: *mut (),
    module: Option<ModuleId>,
}

impl InterruptRouter {
    /// Create new interrupt router
    pub const fn new() -> Self {
        const NONE: Option<HandlerEntry> = None;
        const ZERO: AtomicU64 = AtomicU64::new(0);
        
        Self {
            handlers: Mutex::new([NONE; 256]),
            stats: [ZERO; 256],
        }
    }
    
    /// Register an interrupt handler
    pub fn register(
        &self,
        vector: u8,
        handler: InterruptHandler,
        data: *mut (),
        module: Option<ModuleId>,
    ) -> Result<(), InterruptError> {
        let mut handlers = self.handlers.lock();
        
        if handlers[vector as usize].is_some() {
            return Err(InterruptError::AlreadyRegistered);
        }
        
        handlers[vector as usize] = Some(HandlerEntry {
            handler,
            data,
            module,
        });
        
        Ok(())
    }
    
    /// Unregister an interrupt handler
    pub fn unregister(&self, vector: u8) -> Result<(), InterruptError> {
        let mut handlers = self.handlers.lock();
        
        if handlers[vector as usize].is_none() {
            return Err(InterruptError::NotRegistered);
        }
        
        handlers[vector as usize] = None;
        
        Ok(())
    }
    
    /// Dispatch an interrupt
    pub fn dispatch(&self, vector: u8) {
        // Update statistics
        self.stats[vector as usize].fetch_add(1, Ordering::Relaxed);
        
        // Get handler
        let handlers = self.handlers.lock();
        
        if let Some(entry) = &handlers[vector as usize] {
            // Call handler
            (entry.handler)(vector, entry.data);
        } else {
            // Default handling - just acknowledge
            self.default_handler(vector);
        }
    }
    
    /// Default interrupt handler
    fn default_handler(&self, vector: u8) {
        // Send EOI for hardware interrupts
        if vector >= 32 {
            send_eoi();
        }
    }
    
    /// Get interrupt count for a vector
    pub fn get_count(&self, vector: u8) -> u64 {
        self.stats[vector as usize].load(Ordering::Relaxed)
    }
    
    /// Get total interrupt count
    pub fn total_interrupts(&self) -> u64 {
        self.stats.iter().map(|s| s.load(Ordering::Relaxed)).sum()
    }
}

/// Global interrupt router
pub static INTERRUPT_ROUTER: InterruptRouter = InterruptRouter::new();
```

---

## 5. Syscall Gateway

### 5.1 Syscall Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         SYSCALL ARCHITECTURE                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  User space:                                                                │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  syscall(num, arg1, arg2, arg3, arg4, arg5, arg6)                  │   │
│  │      │                                                              │   │
│  │      │  syscall instruction                                        │   │
│  │      ▼                                                              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Kernel space:                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                     │   │
│  │  ┌─────────────────────┐                                           │   │
│  │  │   Syscall Entry     │  ◀─── MSR_LSTAR points here              │   │
│  │  │   (assembly stub)   │                                           │   │
│  │  └──────────┬──────────┘                                           │   │
│  │             │                                                       │   │
│  │             │  Save registers, switch to kernel stack              │   │
│  │             ▼                                                       │   │
│  │  ┌─────────────────────┐                                           │   │
│  │  │   Syscall Gateway   │  Rust entry point                        │   │
│  │  └──────────┬──────────┘                                           │   │
│  │             │                                                       │   │
│  │             │  Validate arguments                                   │   │
│  │             ▼                                                       │   │
│  │  ┌─────────────────────┐                                           │   │
│  │  │  Syscall Dispatcher │  Route to handler                        │   │
│  │  └──────────┬──────────┘                                           │   │
│  │             │                                                       │   │
│  │             │  Execute handler                                      │   │
│  │             ▼                                                       │   │
│  │  ┌─────────────────────┐                                           │   │
│  │  │   Syscall Handler   │  Actual implementation                   │   │
│  │  └─────────────────────┘                                           │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Syscall Numbers

```rust
/// System call numbers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum SyscallNumber {
    // Process management (0-31)
    Exit = 0,
    Fork = 1,
    Execve = 2,
    Wait = 3,
    Getpid = 4,
    Getppid = 5,
    Clone = 6,
    Kill = 7,
    
    // Memory management (32-63)
    Brk = 32,
    Mmap = 33,
    Munmap = 34,
    Mprotect = 35,
    
    // File operations (64-127)
    Open = 64,
    Close = 65,
    Read = 66,
    Write = 67,
    Lseek = 68,
    Stat = 69,
    Fstat = 70,
    Mkdir = 71,
    Rmdir = 72,
    Unlink = 73,
    
    // Time (128-143)
    GetTimeOfDay = 128,
    Nanosleep = 129,
    ClockGetTime = 130,
    
    // Module management (144-159)
    ModuleLoad = 144,
    ModuleUnload = 145,
    ModuleInfo = 146,
    
    // IPC (160-191)
    MsgSend = 160,
    MsgRecv = 161,
    ChannelCreate = 162,
    ChannelDestroy = 163,
    
    // Debug/misc (192-255)
    Debug = 192,
    SysInfo = 193,
}

impl TryFrom<u64> for SyscallNumber {
    type Error = SyscallError;
    
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SyscallNumber::Exit),
            1 => Ok(SyscallNumber::Fork),
            // ... (all mappings)
            _ => Err(SyscallError::InvalidNumber),
        }
    }
}
```

### 5.3 Syscall Dispatcher

```rust
/// Syscall context (registers at time of syscall)
#[repr(C)]
pub struct SyscallContext {
    /// Syscall number (RAX)
    pub number: u64,
    /// Argument 1 (RDI)
    pub arg1: u64,
    /// Argument 2 (RSI)
    pub arg2: u64,
    /// Argument 3 (RDX)
    pub arg3: u64,
    /// Argument 4 (R10)
    pub arg4: u64,
    /// Argument 5 (R8)
    pub arg5: u64,
    /// Argument 6 (R9)
    pub arg6: u64,
    /// Return value (RAX on return)
    pub return_value: u64,
}

/// Syscall handler function type
pub type SyscallHandler = fn(&mut SyscallContext) -> i64;

/// Syscall dispatcher
pub struct SyscallDispatcher {
    /// Syscall handlers
    handlers: [Option<SyscallHandler>; 256],
    
    /// Syscall statistics
    stats: [AtomicU64; 256],
}

impl SyscallDispatcher {
    /// Create new dispatcher
    pub const fn new() -> Self {
        const NONE: Option<SyscallHandler> = None;
        const ZERO: AtomicU64 = AtomicU64::new(0);
        
        Self {
            handlers: [NONE; 256],
            stats: [ZERO; 256],
        }
    }
    
    /// Register a syscall handler
    pub fn register(&mut self, number: SyscallNumber, handler: SyscallHandler) {
        self.handlers[number as usize] = Some(handler);
    }
    
    /// Dispatch a syscall
    pub fn dispatch(&self, ctx: &mut SyscallContext) -> i64 {
        let number = ctx.number as usize;
        
        // Update statistics
        if number < 256 {
            self.stats[number].fetch_add(1, Ordering::Relaxed);
        }
        
        // Validate syscall number
        let syscall = match SyscallNumber::try_from(ctx.number) {
            Ok(s) => s,
            Err(_) => return -1, // ENOSYS
        };
        
        // Get handler
        match self.handlers[syscall as usize] {
            Some(handler) => handler(ctx),
            None => -1, // ENOSYS
        }
    }
    
    /// Initialize standard syscalls
    pub fn init_standard(&mut self) {
        // Process management
        self.register(SyscallNumber::Exit, sys_exit);
        self.register(SyscallNumber::Getpid, sys_getpid);
        
        // Memory
        self.register(SyscallNumber::Brk, sys_brk);
        self.register(SyscallNumber::Mmap, sys_mmap);
        self.register(SyscallNumber::Munmap, sys_munmap);
        
        // File operations
        self.register(SyscallNumber::Open, sys_open);
        self.register(SyscallNumber::Close, sys_close);
        self.register(SyscallNumber::Read, sys_read);
        self.register(SyscallNumber::Write, sys_write);
        
        // IPC
        self.register(SyscallNumber::MsgSend, sys_msg_send);
        self.register(SyscallNumber::MsgRecv, sys_msg_recv);
        
        // Debug
        self.register(SyscallNumber::Debug, sys_debug);
    }
}

// Example syscall implementations

fn sys_exit(ctx: &mut SyscallContext) -> i64 {
    let exit_code = ctx.arg1 as i32;
    
    serial_println!("[SYSCALL] exit({})", exit_code);
    
    // Terminate current process
    // process_exit(exit_code);
    
    0 // Never returns
}

fn sys_getpid(ctx: &mut SyscallContext) -> i64 {
    // Return current PID
    current_process_id() as i64
}

fn sys_write(ctx: &mut SyscallContext) -> i64 {
    let fd = ctx.arg1 as i32;
    let buf = ctx.arg2 as *const u8;
    let count = ctx.arg3 as usize;
    
    // Validate buffer pointer
    if !is_user_ptr_valid(buf, count) {
        return -14; // EFAULT
    }
    
    // Read from user memory
    let data = unsafe { core::slice::from_raw_parts(buf, count) };
    
    match fd {
        1 | 2 => {
            // stdout/stderr - write to serial
            for &byte in data {
                serial_write_byte(byte);
            }
            count as i64
        }
        _ => -9, // EBADF
    }
}

fn sys_debug(ctx: &mut SyscallContext) -> i64 {
    let code = ctx.arg1;
    let arg = ctx.arg2;
    
    match code {
        0 => {
            // Print debug info
            serial_println!("[DEBUG] arg = {:#x}", arg);
            0
        }
        1 => {
            // Trigger breakpoint
            unsafe { core::arch::asm!("int3") };
            0
        }
        _ => -22, // EINVAL
    }
}
```

---

## 6. Hot-Reload System

### 6.1 Hot-Reload Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         HOT-RELOAD SYSTEM                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Hot-Reload Process:                                                        │
│  ══════════════════                                                         │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                     │   │
│  │    1. PREPARE              2. SWAP               3. CLEANUP        │   │
│  │    ─────────               ────────              ──────────        │   │
│  │                                                                     │   │
│  │  ┌──────────────┐      ┌──────────────┐      ┌──────────────┐     │   │
│  │  │ Load new     │      │ Atomic swap  │      │ Unload old   │     │   │
│  │  │ module code  │ ───▶ │ function     │ ───▶ │ module code  │     │   │
│  │  │              │      │ pointers     │      │              │     │   │
│  │  └──────────────┘      └──────────────┘      └──────────────┘     │   │
│  │         │                    │                      │              │   │
│  │         │                    │                      │              │   │
│  │         ▼                    ▼                      ▼              │   │
│  │  ┌──────────────┐      ┌──────────────┐      ┌──────────────┐     │   │
│  │  │ Validate     │      │ Update state │      │ Free old     │     │   │
│  │  │ compatibility│      │ references   │      │ memory       │     │   │
│  │  └──────────────┘      └──────────────┘      └──────────────┘     │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Requirements:                                                              │
│  ─────────────                                                              │
│  • No change to public interface (ABI stability)                           │
│  • State must be serializable/transferable                                 │
│  • No long-running operations during swap                                  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 6.2 Hot-Reload Engine

```rust
/// Hot-reload manager
pub struct HotReloadManager {
    /// Active modules
    modules: Mutex<HashMap<ModuleId, LoadedModule>>,
    
    /// Pending reloads
    pending: Mutex<Vec<PendingReload>>,
    
    /// Reload history
    history: Mutex<Vec<ReloadEvent>>,
    
    /// Is hot-reload enabled
    enabled: AtomicBool,
}

/// A loaded module
struct LoadedModule {
    id: ModuleId,
    name: &'static str,
    version: Version,
    code_start: usize,
    code_size: usize,
    entry_points: HashMap<&'static str, usize>,
    state: Option<ModuleState>,
}

/// Pending reload operation
struct PendingReload {
    module_id: ModuleId,
    new_code: Vec<u8>,
    priority: u8,
    requested_at: u64,
}

/// Reload event
#[derive(Debug, Clone)]
pub struct ReloadEvent {
    pub module_id: ModuleId,
    pub old_version: Version,
    pub new_version: Version,
    pub timestamp: u64,
    pub duration_ns: u64,
    pub success: bool,
    pub error: Option<String>,
}

impl HotReloadManager {
    /// Create new hot-reload manager
    pub fn new() -> Self {
        Self {
            modules: Mutex::new(HashMap::new()),
            pending: Mutex::new(Vec::new()),
            history: Mutex::new(Vec::new()),
            enabled: AtomicBool::new(true),
        }
    }
    
    /// Enable/disable hot-reload
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }
    
    /// Check if hot-reload is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }
    
    /// Queue a module for reload
    pub fn queue_reload(
        &self,
        module_id: ModuleId,
        new_code: Vec<u8>,
        priority: u8,
    ) -> Result<(), HotReloadError> {
        if !self.is_enabled() {
            return Err(HotReloadError::Disabled);
        }
        
        // Validate new code
        self.validate_code(&new_code)?;
        
        // Add to pending queue
        self.pending.lock().push(PendingReload {
            module_id,
            new_code,
            priority,
            requested_at: current_tick(),
        });
        
        Ok(())
    }
    
    /// Execute pending reloads
    pub fn process_pending(&self) -> Vec<ReloadEvent> {
        let mut events = Vec::new();
        let mut pending = self.pending.lock();
        
        // Sort by priority
        pending.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        // Process each
        while let Some(reload) = pending.pop() {
            let event = self.execute_reload(reload);
            events.push(event.clone());
            self.history.lock().push(event);
        }
        
        events
    }
    
    /// Execute a single reload
    fn execute_reload(&self, reload: PendingReload) -> ReloadEvent {
        let start_time = current_tick();
        let mut modules = self.modules.lock();
        
        let old_version = modules
            .get(&reload.module_id)
            .map(|m| m.version.clone())
            .unwrap_or_default();
        
        // Perform reload
        match self.do_reload(&mut modules, reload.module_id, reload.new_code) {
            Ok(new_version) => {
                let duration = (current_tick() - start_time) * 1_000_000; // Approximate ns
                
                ReloadEvent {
                    module_id: reload.module_id,
                    old_version,
                    new_version,
                    timestamp: start_time,
                    duration_ns: duration,
                    success: true,
                    error: None,
                }
            }
            Err(e) => {
                ReloadEvent {
                    module_id: reload.module_id,
                    old_version: old_version.clone(),
                    new_version: old_version,
                    timestamp: start_time,
                    duration_ns: 0,
                    success: false,
                    error: Some(format!("{:?}", e)),
                }
            }
        }
    }
    
    /// Perform the actual reload
    fn do_reload(
        &self,
        modules: &mut HashMap<ModuleId, LoadedModule>,
        id: ModuleId,
        new_code: Vec<u8>,
    ) -> Result<Version, HotReloadError> {
        // Get existing module
        let existing = modules.get(&id).ok_or(HotReloadError::ModuleNotFound)?;
        
        // Save state
        let state = existing.state.clone();
        
        // Parse new code (simplified)
        let new_module = self.parse_module_code(&new_code)?;
        
        // Verify ABI compatibility
        self.verify_abi_compat(existing, &new_module)?;
        
        // Allocate new code space
        let new_code_addr = self.allocate_code_space(new_code.len())?;
        
        // Copy new code
        unsafe {
            core::ptr::copy_nonoverlapping(
                new_code.as_ptr(),
                new_code_addr as *mut u8,
                new_code.len(),
            );
        }
        
        // Update module entry
        let new_version = new_module.version.clone();
        let mut loaded = new_module;
        loaded.code_start = new_code_addr;
        loaded.state = state; // Preserve state
        
        // Atomic swap (replace entry)
        modules.insert(id, loaded);
        
        // Free old code (deferred to avoid issues)
        // self.free_code_space(existing.code_start, existing.code_size);
        
        Ok(new_version)
    }
    
    /// Validate code before loading
    fn validate_code(&self, code: &[u8]) -> Result<(), HotReloadError> {
        if code.is_empty() {
            return Err(HotReloadError::InvalidCode);
        }
        
        // Check magic number, format, etc.
        // ...
        
        Ok(())
    }
    
    /// Get reload history
    pub fn get_history(&self) -> Vec<ReloadEvent> {
        self.history.lock().clone()
    }
}
```

### 6.3 Scheduler Hot-Reload Example

```rust
/// Hot-reloadable scheduler interface
pub trait HotReloadableScheduler: Send + Sync {
    /// Get scheduler name
    fn name(&self) -> &'static str;
    
    /// Get scheduler version
    fn version(&self) -> Version;
    
    /// Add a task to the scheduler
    fn add_task(&mut self, task: Task);
    
    /// Remove a task
    fn remove_task(&mut self, id: TaskId) -> Option<Task>;
    
    /// Pick next task to run
    fn next(&mut self) -> Option<Task>;
    
    /// Serialize state for hot-reload
    fn serialize_state(&self) -> Vec<u8>;
    
    /// Deserialize state after hot-reload
    fn deserialize_state(&mut self, state: &[u8]) -> Result<(), SchedulerError>;
}

/// Scheduler hot-reload wrapper
pub struct SchedulerWrapper {
    /// Current scheduler instance
    inner: Mutex<Box<dyn HotReloadableScheduler>>,
    
    /// Reload count
    reload_count: AtomicU64,
}

impl SchedulerWrapper {
    /// Create with initial scheduler
    pub fn new(scheduler: Box<dyn HotReloadableScheduler>) -> Self {
        Self {
            inner: Mutex::new(scheduler),
            reload_count: AtomicU64::new(0),
        }
    }
    
    /// Hot-reload to a new scheduler
    pub fn hot_reload(
        &self,
        new_scheduler: Box<dyn HotReloadableScheduler>,
    ) -> Result<(), SchedulerError> {
        let mut inner = self.inner.lock();
        
        // Serialize current state
        let state = inner.serialize_state();
        
        // Replace scheduler
        let mut new = new_scheduler;
        
        // Restore state
        new.deserialize_state(&state)?;
        
        // Swap
        *inner = new;
        
        self.reload_count.fetch_add(1, Ordering::Relaxed);
        
        serial_println!(
            "[HOT-RELOAD] Scheduler {} -> {} (reload #{})",
            inner.name(),
            new.name(),
            self.reload_count.load(Ordering::Relaxed),
        );
        
        Ok(())
    }
    
    /// Get current scheduler
    pub fn current(&self) -> impl core::ops::Deref<Target = Box<dyn HotReloadableScheduler>> + '_ {
        self.inner.lock()
    }
}
```

---

## 7. Debug Console

### 7.1 Serial Console

```rust
/// Serial port for debug output
pub struct SerialConsole {
    /// Port base address
    port: u16,
    
    /// Is initialized
    initialized: AtomicBool,
}

impl SerialConsole {
    /// Standard COM1 port
    pub const COM1: u16 = 0x3F8;
    
    /// Create new serial console
    pub const fn new(port: u16) -> Self {
        Self {
            port,
            initialized: AtomicBool::new(false),
        }
    }
    
    /// Initialize the serial port
    pub fn init(&self) {
        unsafe {
            // Disable interrupts
            outb(self.port + 1, 0x00);
            
            // Enable DLAB
            outb(self.port + 3, 0x80);
            
            // Set baud rate to 115200
            outb(self.port + 0, 0x01); // Low byte
            outb(self.port + 1, 0x00); // High byte
            
            // 8 bits, no parity, one stop bit
            outb(self.port + 3, 0x03);
            
            // Enable FIFO
            outb(self.port + 2, 0xC7);
            
            // Enable IRQs, RTS/DSR set
            outb(self.port + 4, 0x0B);
        }
        
        self.initialized.store(true, Ordering::SeqCst);
    }
    
    /// Check if transmit buffer is empty
    fn is_transmit_empty(&self) -> bool {
        unsafe { inb(self.port + 5) & 0x20 != 0 }
    }
    
    /// Write a byte
    pub fn write_byte(&self, byte: u8) {
        while !self.is_transmit_empty() {
            core::hint::spin_loop();
        }
        
        unsafe {
            outb(self.port, byte);
        }
    }
    
    /// Write a string
    pub fn write_str(&self, s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
    }
}

impl core::fmt::Write for SerialConsole {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        SerialConsole::write_str(self, s);
        Ok(())
    }
}

/// Global serial console
pub static SERIAL: SerialConsole = SerialConsole::new(SerialConsole::COM1);

/// Print macro
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::debug::console::_print(format_args!($($arg)*))
    };
}

/// Println macro
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    
    let mut console = SERIAL;
    console.write_fmt(args).unwrap();
}
```

---

## 8. Self-Healing

### 8.1 Self-Healing System

```rust
/// Self-healing manager
pub struct SelfHealingManager {
    /// Recovery handlers
    handlers: Mutex<Vec<RecoveryHandler>>,
    
    /// Failure history
    failures: Mutex<Vec<Failure>>,
    
    /// Recovery statistics
    stats: RecoveryStats,
}

/// Recovery handler
pub struct RecoveryHandler {
    pub name: &'static str,
    pub failure_type: FailureType,
    pub recover: fn(&Failure) -> RecoveryResult,
}

/// Failure types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureType {
    /// Module crashed
    ModuleCrash,
    
    /// Memory corruption detected
    MemoryCorruption,
    
    /// Deadlock detected
    Deadlock,
    
    /// Resource exhaustion
    ResourceExhaustion,
    
    /// Hardware error
    HardwareError,
    
    /// Watchdog timeout
    WatchdogTimeout,
}

/// Recovery result
pub enum RecoveryResult {
    /// Successfully recovered
    Recovered,
    
    /// Partially recovered
    PartialRecovery(String),
    
    /// Recovery failed
    Failed(String),
}

/// Failure record
#[derive(Debug, Clone)]
pub struct Failure {
    pub failure_type: FailureType,
    pub module: Option<ModuleId>,
    pub description: String,
    pub timestamp: u64,
    pub context: FailureContext,
}

/// Failure context
#[derive(Debug, Clone)]
pub struct FailureContext {
    pub rip: u64,
    pub rsp: u64,
    pub registers: Option<Registers>,
    pub stack_trace: Vec<u64>,
}

impl SelfHealingManager {
    /// Create new self-healing manager
    pub fn new() -> Self {
        Self {
            handlers: Mutex::new(Vec::new()),
            failures: Mutex::new(Vec::new()),
            stats: RecoveryStats::default(),
        }
    }
    
    /// Register a recovery handler
    pub fn register_handler(&self, handler: RecoveryHandler) {
        self.handlers.lock().push(handler);
    }
    
    /// Report a failure
    pub fn report_failure(&self, failure: Failure) -> RecoveryResult {
        // Log failure
        serial_println!("[SELF-HEAL] Failure detected: {:?}", failure.failure_type);
        serial_println!("  Description: {}", failure.description);
        
        // Record failure
        self.failures.lock().push(failure.clone());
        
        // Find handler
        let handlers = self.handlers.lock();
        
        for handler in handlers.iter() {
            if handler.failure_type == failure.failure_type {
                serial_println!("[SELF-HEAL] Attempting recovery: {}", handler.name);
                
                let result = (handler.recover)(&failure);
                
                match &result {
                    RecoveryResult::Recovered => {
                        serial_println!("[SELF-HEAL] Recovery successful");
                        self.stats.successful.fetch_add(1, Ordering::Relaxed);
                    }
                    RecoveryResult::PartialRecovery(msg) => {
                        serial_println!("[SELF-HEAL] Partial recovery: {}", msg);
                        self.stats.partial.fetch_add(1, Ordering::Relaxed);
                    }
                    RecoveryResult::Failed(msg) => {
                        serial_println!("[SELF-HEAL] Recovery failed: {}", msg);
                        self.stats.failed.fetch_add(1, Ordering::Relaxed);
                    }
                }
                
                return result;
            }
        }
        
        serial_println!("[SELF-HEAL] No handler found for {:?}", failure.failure_type);
        RecoveryResult::Failed("No handler".to_string())
    }
    
    /// Get failure history
    pub fn get_failures(&self) -> Vec<Failure> {
        self.failures.lock().clone()
    }
    
    /// Get recovery statistics
    pub fn get_stats(&self) -> RecoveryStats {
        self.stats.clone()
    }
}

/// Recovery statistics
#[derive(Debug, Clone, Default)]
pub struct RecoveryStats {
    pub successful: AtomicU64,
    pub partial: AtomicU64,
    pub failed: AtomicU64,
}

// Example recovery handlers

fn recover_module_crash(failure: &Failure) -> RecoveryResult {
    if let Some(module_id) = failure.module {
        serial_println!("[RECOVER] Restarting module {:?}", module_id);
        
        // Unload crashed module
        // reload_module(module_id);
        
        RecoveryResult::Recovered
    } else {
        RecoveryResult::Failed("Unknown module".to_string())
    }
}

fn recover_resource_exhaustion(failure: &Failure) -> RecoveryResult {
    serial_println!("[RECOVER] Attempting to free resources");
    
    // Try to free caches, compact memory, etc.
    // free_caches();
    // compact_memory();
    
    RecoveryResult::PartialRecovery("Freed some resources".to_string())
}
```

---

## 9. Error Types

### 9.1 Core Error Types

```rust
/// Channel error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelError {
    /// Channel is closed
    Closed,
    /// Channel is full
    Full,
    /// Channel is empty
    Empty,
    /// Operation timed out
    Timeout,
    /// Invalid capacity
    InvalidCapacity,
}

/// Capability error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityError {
    /// Capability not found
    NotFound,
    /// Policy denied
    PolicyDenied,
    /// Cannot revoke
    NotRevocable,
    /// Already expired
    Expired,
}

/// Resource error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceError {
    /// Resource not found
    NotFound,
    /// Unknown resource type
    UnknownResource,
    /// Not enough resources
    InsufficientResources,
    /// Resource already allocated
    AlreadyAllocated,
}

/// Lifecycle error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleError {
    /// Invalid state transition
    InvalidTransition,
    /// Module not found
    ModuleNotFound,
    /// Already in desired state
    AlreadyInState,
}

/// Interrupt error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptError {
    /// Already registered
    AlreadyRegistered,
    /// Not registered
    NotRegistered,
    /// Invalid vector
    InvalidVector,
}

/// Syscall error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallError {
    /// Invalid syscall number
    InvalidNumber,
    /// Permission denied
    PermissionDenied,
    /// Invalid argument
    InvalidArgument,
    /// No such process
    NoSuchProcess,
}

/// Hot-reload error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotReloadError {
    /// Hot-reload is disabled
    Disabled,
    /// Module not found
    ModuleNotFound,
    /// Invalid code
    InvalidCode,
    /// ABI mismatch
    AbiMismatch,
    /// State transfer failed
    StateTransferFailed,
    /// Memory allocation failed
    AllocationFailed,
}
```

---

## Summary

The Helix Core provides:

1. **IPC System**: Channels, event bus, message routing
2. **Orchestrator**: Lifecycle, capabilities, resources
3. **Interrupts**: Exception handling, interrupt routing
4. **Syscalls**: Gateway, dispatcher, handlers
5. **Hot-Reload**: Live module updates
6. **Debug**: Serial console, diagnostics
7. **Self-Healing**: Failure detection and recovery

For implementation details, see [core/src/](../../core/src/).

---

<div align="center">

⚙️ *The core that powers Helix* ⚙️

</div>
