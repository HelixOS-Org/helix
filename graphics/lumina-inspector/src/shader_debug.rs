//! # Shader Debugger
//!
//! Revolutionary shader debugging with:
//! - Variable inspection
//! - Breakpoints
//! - Step execution
//! - Waveform analysis

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::resource::ShaderStage;
use crate::{InspectorError, InspectorErrorKind, InspectorResult};

/// Shader debugger
pub struct ShaderDebugger {
    enabled: bool,
    sessions: BTreeMap<u64, DebugSession>,
    breakpoints: BTreeMap<u64, Vec<ShaderBreakpoint>>,
    next_session_id: u64,
}

impl ShaderDebugger {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            sessions: BTreeMap::new(),
            breakpoints: BTreeMap::new(),
            next_session_id: 1,
        }
    }

    /// Start a debug session
    pub fn start_session(
        &mut self,
        shader_id: u64,
        breakpoints: &[ShaderBreakpoint],
    ) -> InspectorResult<ShaderDebugSession> {
        if !self.enabled {
            return Err(InspectorError::new(
                InspectorErrorKind::ShaderDebugError,
                "Shader debugging is disabled",
            ));
        }

        let session_id = self.next_session_id;
        self.next_session_id += 1;

        let session = DebugSession {
            session_id,
            shader_id,
            state: DebugState::Running,
            current_location: None,
            variables: BTreeMap::new(),
            call_stack: Vec::new(),
            hit_breakpoints: Vec::new(),
        };

        self.sessions.insert(session_id, session);
        self.breakpoints.insert(shader_id, breakpoints.to_vec());

        Ok(ShaderDebugSession {
            session_id,
            shader_id,
        })
    }

    /// Stop a debug session
    pub fn stop_session(&mut self, session_id: u64) {
        if let Some(session) = self.sessions.remove(&session_id) {
            self.breakpoints.remove(&session.shader_id);
        }
    }

    /// Continue execution
    pub fn continue_execution(&mut self, session_id: u64) -> InspectorResult<()> {
        let session = self.sessions.get_mut(&session_id).ok_or_else(|| {
            InspectorError::new(InspectorErrorKind::ShaderDebugError, "Session not found")
        })?;

        session.state = DebugState::Running;
        Ok(())
    }

    /// Step to next instruction
    pub fn step(&mut self, session_id: u64) -> InspectorResult<StepResult> {
        let session = self.sessions.get_mut(&session_id).ok_or_else(|| {
            InspectorError::new(InspectorErrorKind::ShaderDebugError, "Session not found")
        })?;

        session.state = DebugState::Stepping;

        // Simulated step result
        Ok(StepResult {
            new_location: session.current_location.clone(),
            variables_changed: Vec::new(),
        })
    }

    /// Step over function call
    pub fn step_over(&mut self, session_id: u64) -> InspectorResult<StepResult> {
        self.step(session_id)
    }

    /// Step into function
    pub fn step_into(&mut self, session_id: u64) -> InspectorResult<StepResult> {
        self.step(session_id)
    }

    /// Step out of function
    pub fn step_out(&mut self, session_id: u64) -> InspectorResult<StepResult> {
        self.step(session_id)
    }

    /// Get variable value
    pub fn get_variable(&self, session_id: u64, name: &str) -> Option<ShaderVariable> {
        let session = self.sessions.get(&session_id)?;
        session.variables.get(name).cloned()
    }

    /// Get all variables
    pub fn get_variables(&self, session_id: u64) -> Option<&BTreeMap<String, ShaderVariable>> {
        self.sessions.get(&session_id).map(|s| &s.variables)
    }

    /// Get call stack
    pub fn get_call_stack(&self, session_id: u64) -> Option<&[StackFrame]> {
        self.sessions
            .get(&session_id)
            .map(|s| s.call_stack.as_slice())
    }

    /// Add breakpoint
    pub fn add_breakpoint(&mut self, shader_id: u64, breakpoint: ShaderBreakpoint) {
        self.breakpoints
            .entry(shader_id)
            .or_insert_with(Vec::new)
            .push(breakpoint);
    }

    /// Remove breakpoint
    pub fn remove_breakpoint(&mut self, shader_id: u64, breakpoint_id: u64) {
        if let Some(breakpoints) = self.breakpoints.get_mut(&shader_id) {
            breakpoints.retain(|bp| bp.id != breakpoint_id);
        }
    }

    /// Get breakpoints for shader
    pub fn get_breakpoints(&self, shader_id: u64) -> Option<&[ShaderBreakpoint]> {
        self.breakpoints.get(&shader_id).map(|v| v.as_slice())
    }
}

/// Debug session handle
#[derive(Debug, Clone)]
pub struct ShaderDebugSession {
    pub session_id: u64,
    pub shader_id: u64,
}

/// Internal debug session state
struct DebugSession {
    session_id: u64,
    shader_id: u64,
    state: DebugState,
    current_location: Option<SourceLocation>,
    variables: BTreeMap<String, ShaderVariable>,
    call_stack: Vec<StackFrame>,
    hit_breakpoints: Vec<u64>,
}

/// Debug state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugState {
    Running,
    Paused,
    Stepping,
    Completed,
    Error,
}

/// Shader breakpoint
#[derive(Debug, Clone)]
pub struct ShaderBreakpoint {
    pub id: u64,
    pub location: BreakpointLocation,
    pub condition: Option<String>,
    pub hit_count: Option<u32>,
    pub enabled: bool,
}

/// Breakpoint location
#[derive(Debug, Clone)]
pub enum BreakpointLocation {
    Line { file: String, line: u32 },
    Function { name: String },
    Instruction { offset: u64 },
    Pixel { x: u32, y: u32 },
    Thread { invocation: [u32; 3] },
}

/// Source location
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: Option<String>,
    pub line: u32,
    pub column: u32,
    pub instruction: u64,
}

/// Step result
#[derive(Debug, Clone)]
pub struct StepResult {
    pub new_location: Option<SourceLocation>,
    pub variables_changed: Vec<String>,
}

/// Shader variable
#[derive(Debug, Clone)]
pub struct ShaderVariable {
    pub name: String,
    pub variable_type: ShaderType,
    pub value: ShaderValue,
    pub storage_class: StorageClass,
}

/// Shader type
#[derive(Debug, Clone)]
pub enum ShaderType {
    Void,
    Bool,
    Int,
    Uint,
    Float,
    Double,
    Vec2,
    Vec3,
    Vec4,
    IVec2,
    IVec3,
    IVec4,
    UVec2,
    UVec3,
    UVec4,
    Mat2,
    Mat3,
    Mat4,
    Sampler2D,
    SamplerCube,
    Image2D,
    Struct {
        name: String,
        fields: Vec<(String, ShaderType)>,
    },
    Array {
        element: Box<ShaderType>,
        size: Option<u32>,
    },
}

/// Shader value
#[derive(Debug, Clone)]
pub enum ShaderValue {
    Void,
    Bool(bool),
    Int(i32),
    Uint(u32),
    Float(f32),
    Double(f64),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    IVec2([i32; 2]),
    IVec3([i32; 3]),
    IVec4([i32; 4]),
    UVec2([u32; 2]),
    UVec3([u32; 3]),
    UVec4([u32; 4]),
    Mat2([[f32; 2]; 2]),
    Mat3([[f32; 3]; 3]),
    Mat4([[f32; 4]; 4]),
    Struct(Vec<(String, ShaderValue)>),
    Array(Vec<ShaderValue>),
    Handle(u64),
    Unknown,
}

/// Storage class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageClass {
    UniformConstant,
    Input,
    Uniform,
    Output,
    Workgroup,
    CrossWorkgroup,
    Private,
    Function,
    Generic,
    PushConstant,
    AtomicCounter,
    Image,
    StorageBuffer,
}

/// Stack frame
#[derive(Debug, Clone)]
pub struct StackFrame {
    pub function_name: String,
    pub location: SourceLocation,
    pub locals: Vec<String>,
}

/// Waveform debugger for SIMD analysis
pub struct WaveformDebugger {
    wave_size: u32,
    lanes: Vec<LaneState>,
    current_instruction: u64,
}

impl WaveformDebugger {
    pub fn new(wave_size: u32) -> Self {
        Self {
            wave_size,
            lanes: (0..wave_size)
                .map(|i| LaneState {
                    lane_id: i,
                    active: true,
                    converged: true,
                    registers: BTreeMap::new(),
                })
                .collect(),
            current_instruction: 0,
        }
    }

    /// Get active lane count
    pub fn active_lane_count(&self) -> u32 {
        self.lanes.iter().filter(|l| l.active).count() as u32
    }

    /// Check if wave is uniform
    pub fn is_uniform(&self, register: &str) -> bool {
        let values: Vec<_> = self
            .lanes
            .iter()
            .filter(|l| l.active)
            .filter_map(|l| l.registers.get(register))
            .collect();

        if values.is_empty() {
            return true;
        }

        let first = &values[0];
        values.iter().all(|v| v == first)
    }

    /// Get divergence mask
    pub fn divergence_mask(&self) -> u64 {
        let mut mask = 0u64;
        for (i, lane) in self.lanes.iter().enumerate() {
            if lane.active && !lane.converged {
                mask |= 1 << i;
            }
        }
        mask
    }

    /// Get lane value
    pub fn get_lane_value(&self, lane_id: u32, register: &str) -> Option<&ShaderValue> {
        self.lanes.get(lane_id as usize)?.registers.get(register)
    }

    /// Get all lane values for a register
    pub fn get_all_lane_values(&self, register: &str) -> Vec<Option<&ShaderValue>> {
        self.lanes
            .iter()
            .map(|l| l.registers.get(register))
            .collect()
    }
}

/// Lane state
#[derive(Debug, Clone)]
pub struct LaneState {
    pub lane_id: u32,
    pub active: bool,
    pub converged: bool,
    pub registers: BTreeMap<String, ShaderValue>,
}

/// Pixel debugging for fragment shaders
pub struct PixelDebugger {
    target_x: u32,
    target_y: u32,
    captured_invocations: Vec<PixelInvocation>,
    watch_expressions: Vec<String>,
}

impl PixelDebugger {
    pub fn new(x: u32, y: u32) -> Self {
        Self {
            target_x: x,
            target_y: y,
            captured_invocations: Vec::new(),
            watch_expressions: Vec::new(),
        }
    }

    /// Set target pixel
    pub fn set_target(&mut self, x: u32, y: u32) {
        self.target_x = x;
        self.target_y = y;
        self.captured_invocations.clear();
    }

    /// Add watch expression
    pub fn add_watch(&mut self, expression: String) {
        self.watch_expressions.push(expression);
    }

    /// Get captured invocations
    pub fn invocations(&self) -> &[PixelInvocation] {
        &self.captured_invocations
    }

    /// Record an invocation
    pub fn record_invocation(&mut self, invocation: PixelInvocation) {
        self.captured_invocations.push(invocation);
    }
}

/// Captured pixel invocation
#[derive(Debug, Clone)]
pub struct PixelInvocation {
    pub x: u32,
    pub y: u32,
    pub primitive_id: u32,
    pub inputs: BTreeMap<String, ShaderValue>,
    pub outputs: BTreeMap<String, ShaderValue>,
    pub instructions_executed: u64,
    pub discarded: bool,
}

/// Compute thread debugger
pub struct ComputeDebugger {
    workgroup_size: [u32; 3],
    target_invocation: [u32; 3],
    captured_data: Option<ComputeInvocation>,
}

impl ComputeDebugger {
    pub fn new(workgroup_size: [u32; 3]) -> Self {
        Self {
            workgroup_size,
            target_invocation: [0, 0, 0],
            captured_data: None,
        }
    }

    /// Set target invocation
    pub fn set_target(&mut self, global_id: [u32; 3]) {
        self.target_invocation = global_id;
        self.captured_data = None;
    }

    /// Get captured data
    pub fn captured(&self) -> Option<&ComputeInvocation> {
        self.captured_data.as_ref()
    }

    /// Record invocation
    pub fn record(&mut self, invocation: ComputeInvocation) {
        self.captured_data = Some(invocation);
    }
}

/// Captured compute invocation
#[derive(Debug, Clone)]
pub struct ComputeInvocation {
    pub global_id: [u32; 3],
    pub local_id: [u32; 3],
    pub workgroup_id: [u32; 3],
    pub variables: BTreeMap<String, ShaderValue>,
    pub shared_memory: Vec<u8>,
    pub barriers_hit: u32,
}

/// Printf-style shader debugging
pub struct ShaderPrintf {
    messages: Vec<PrintfMessage>,
    max_messages: usize,
}

impl ShaderPrintf {
    pub fn new(max_messages: usize) -> Self {
        Self {
            messages: Vec::new(),
            max_messages,
        }
    }

    /// Record a printf message
    pub fn record(&mut self, message: PrintfMessage) {
        if self.messages.len() >= self.max_messages {
            self.messages.remove(0);
        }
        self.messages.push(message);
    }

    /// Get all messages
    pub fn messages(&self) -> &[PrintfMessage] {
        &self.messages
    }

    /// Clear messages
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Filter by invocation
    pub fn filter_by_invocation(&self, global_id: [u32; 3]) -> Vec<&PrintfMessage> {
        self.messages
            .iter()
            .filter(|m| m.global_invocation_id == global_id)
            .collect()
    }
}

/// Printf message from shader
#[derive(Debug, Clone)]
pub struct PrintfMessage {
    pub format_string: String,
    pub arguments: Vec<ShaderValue>,
    pub global_invocation_id: [u32; 3],
    pub stage: ShaderStage,
    pub timestamp: u64,
}

impl PrintfMessage {
    /// Format the message
    pub fn format(&self) -> String {
        // Simple formatting - in reality would need full printf implementation
        let mut result = self.format_string.clone();

        for arg in &self.arguments {
            let value_str = match arg {
                ShaderValue::Int(v) => alloc::format!("{}", v),
                ShaderValue::Uint(v) => alloc::format!("{}", v),
                ShaderValue::Float(v) => alloc::format!("{:.6}", v),
                ShaderValue::Vec2(v) => alloc::format!("({:.4}, {:.4})", v[0], v[1]),
                ShaderValue::Vec3(v) => alloc::format!("({:.4}, {:.4}, {:.4})", v[0], v[1], v[2]),
                ShaderValue::Vec4(v) => {
                    alloc::format!("({:.4}, {:.4}, {:.4}, {:.4})", v[0], v[1], v[2], v[3])
                },
                _ => String::from("?"),
            };

            if let Some(pos) = result.find('%') {
                result.replace_range(pos..pos + 2, &value_str);
            }
        }

        result
    }
}
