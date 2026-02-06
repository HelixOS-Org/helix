//! Frame Context & Per-Frame Resources
//!
//! Manages per-frame state and resources:
//! - Frame-local allocations
//! - Command buffer recording
//! - Submission management
//! - Frame synchronization

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::resource::{BufferDesc, BufferHandle, TextureDesc, TextureHandle};
use crate::scheduler::{DescriptorAllocation, DescriptorHeapType, UploadAllocation};

/// Frame context containing all per-frame resources.
pub struct Frame {
    /// Frame index (within ring buffer).
    index: usize,
    /// Frame number (monotonically increasing).
    number: u64,
    /// Command lists for this frame.
    command_lists: Vec<CommandList>,
    /// Scratch buffer allocator.
    scratch_allocator: ScratchAllocator,
    /// Upload heap allocator.
    upload_allocator: UploadAllocator,
    /// Descriptor allocators.
    descriptors: DescriptorAllocators,
    /// Pending resource deletions.
    deletions: Vec<DeferredDeletion>,
    /// Frame fence value.
    fence_value: u64,
    /// Is frame in flight.
    in_flight: bool,
}

impl Frame {
    /// Create a new frame.
    pub fn new(index: usize, config: &FrameConfig) -> Self {
        Self {
            index,
            number: 0,
            command_lists: Vec::new(),
            scratch_allocator: ScratchAllocator::new(config.scratch_size),
            upload_allocator: UploadAllocator::new(config.upload_size),
            descriptors: DescriptorAllocators::new(config),
            deletions: Vec::new(),
            fence_value: 0,
            in_flight: false,
        }
    }

    /// Get frame index.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Get frame number.
    pub fn number(&self) -> u64 {
        self.number
    }

    /// Begin frame (wait for previous use to complete).
    pub fn begin(&mut self, frame_number: u64) {
        // Would wait for fence here
        self.in_flight = false;

        // Reset allocators
        self.scratch_allocator.reset();
        self.upload_allocator.reset();
        self.descriptors.reset();

        // Process deferred deletions
        self.process_deletions();

        // Reset command lists
        for cmd in &mut self.command_lists {
            cmd.reset();
        }

        self.number = frame_number;
    }

    /// End frame and submit.
    pub fn end(&mut self) {
        self.fence_value += 1;
        self.in_flight = true;
    }

    /// Get or create a command list.
    pub fn get_command_list(&mut self, list_type: CommandListType) -> &mut CommandList {
        // Find existing or create new
        for (i, cmd) in self.command_lists.iter().enumerate() {
            if cmd.list_type == list_type && !cmd.recording {
                return &mut self.command_lists[i];
            }
        }

        // Create new
        self.command_lists.push(CommandList::new(list_type));
        self.command_lists.last_mut().unwrap()
    }

    /// Allocate scratch memory.
    pub fn allocate_scratch(&mut self, size: usize, alignment: usize) -> Option<ScratchAllocation> {
        self.scratch_allocator.allocate(size, alignment)
    }

    /// Allocate upload memory.
    pub fn allocate_upload(&mut self, size: usize, alignment: usize) -> Option<UploadAllocation> {
        self.upload_allocator.allocate(size, alignment)
    }

    /// Allocate descriptors.
    pub fn allocate_descriptors(
        &mut self,
        count: u32,
        heap_type: DescriptorHeapType,
    ) -> Option<DescriptorAllocation> {
        self.descriptors.allocate(count, heap_type)
    }

    /// Defer resource deletion to end of frame.
    pub fn defer_deletion(&mut self, deletion: DeferredDeletion) {
        self.deletions.push(deletion);
    }

    fn process_deletions(&mut self) {
        for deletion in self.deletions.drain(..) {
            match deletion {
                DeferredDeletion::Buffer(handle) => {
                    // Would destroy buffer
                },
                DeferredDeletion::Texture(handle) => {
                    // Would destroy texture
                },
                DeferredDeletion::Pipeline(id) => {
                    // Would destroy pipeline
                },
                DeferredDeletion::Custom { data, destructor } => {
                    destructor(data);
                },
            }
        }
    }
}

/// Frame configuration.
#[derive(Debug, Clone)]
pub struct FrameConfig {
    /// Scratch buffer size.
    pub scratch_size: usize,
    /// Upload buffer size.
    pub upload_size: usize,
    /// CBV/SRV/UAV descriptor count.
    pub cbv_srv_uav_count: u32,
    /// Sampler descriptor count.
    pub sampler_count: u32,
    /// RTV descriptor count.
    pub rtv_count: u32,
    /// DSV descriptor count.
    pub dsv_count: u32,
}

impl Default for FrameConfig {
    fn default() -> Self {
        Self {
            scratch_size: 64 * 1024 * 1024, // 64 MB
            upload_size: 32 * 1024 * 1024,  // 32 MB
            cbv_srv_uav_count: 65536,
            sampler_count: 2048,
            rtv_count: 256,
            dsv_count: 64,
        }
    }
}

/// Command list for recording GPU commands.
pub struct CommandList {
    /// List type.
    list_type: CommandListType,
    /// Is currently recording.
    recording: bool,
    /// Commands.
    commands: Vec<RecordedCommand>,
    /// Name.
    name: Option<String>,
}

impl CommandList {
    /// Create new command list.
    pub fn new(list_type: CommandListType) -> Self {
        Self {
            list_type,
            recording: false,
            commands: Vec::new(),
            name: None,
        }
    }

    /// Reset for reuse.
    pub fn reset(&mut self) {
        self.recording = false;
        self.commands.clear();
    }

    /// Begin recording.
    pub fn begin(&mut self) {
        self.recording = true;
        self.commands.clear();
    }

    /// End recording.
    pub fn end(&mut self) {
        self.recording = false;
    }

    /// Set debug name.
    pub fn set_name(&mut self, name: &str) {
        self.name = Some(String::from(name));
    }

    /// Record a barrier.
    pub fn barrier(&mut self, barriers: Vec<ResourceBarrier>) {
        self.commands.push(RecordedCommand::Barrier(barriers));
    }

    /// Record a draw call.
    pub fn draw(
        &mut self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        self.commands.push(RecordedCommand::Draw {
            vertex_count,
            instance_count,
            first_vertex,
            first_instance,
        });
    }

    /// Record an indexed draw call.
    pub fn draw_indexed(
        &mut self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) {
        self.commands.push(RecordedCommand::DrawIndexed {
            index_count,
            instance_count,
            first_index,
            vertex_offset,
            first_instance,
        });
    }

    /// Record an indirect draw.
    pub fn draw_indirect(&mut self, buffer: BufferHandle, offset: u64, count: u32) {
        self.commands.push(RecordedCommand::DrawIndirect {
            buffer,
            offset,
            count,
        });
    }

    /// Record a dispatch.
    pub fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        self.commands.push(RecordedCommand::Dispatch { x, y, z });
    }

    /// Record an indirect dispatch.
    pub fn dispatch_indirect(&mut self, buffer: BufferHandle, offset: u64) {
        self.commands
            .push(RecordedCommand::DispatchIndirect { buffer, offset });
    }

    /// Copy buffer to buffer.
    pub fn copy_buffer(&mut self, src: BufferHandle, dst: BufferHandle, size: u64) {
        self.commands.push(RecordedCommand::CopyBuffer {
            src,
            dst,
            src_offset: 0,
            dst_offset: 0,
            size,
        });
    }

    /// Copy buffer to texture.
    pub fn copy_buffer_to_texture(
        &mut self,
        src_buffer: BufferHandle,
        dst_texture: TextureHandle,
        region: BufferTextureCopy,
    ) {
        self.commands.push(RecordedCommand::CopyBufferToTexture {
            src_buffer,
            dst_texture,
            region,
        });
    }

    /// Get command count.
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }
}

/// Command list type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandListType {
    /// Graphics commands.
    Graphics,
    /// Compute commands.
    Compute,
    /// Copy/transfer commands.
    Copy,
}

/// Recorded command.
#[derive(Debug, Clone)]
pub enum RecordedCommand {
    Barrier(Vec<ResourceBarrier>),
    Draw {
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    },
    DrawIndexed {
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    },
    DrawIndirect {
        buffer: BufferHandle,
        offset: u64,
        count: u32,
    },
    Dispatch {
        x: u32,
        y: u32,
        z: u32,
    },
    DispatchIndirect {
        buffer: BufferHandle,
        offset: u64,
    },
    CopyBuffer {
        src: BufferHandle,
        dst: BufferHandle,
        src_offset: u64,
        dst_offset: u64,
        size: u64,
    },
    CopyBufferToTexture {
        src_buffer: BufferHandle,
        dst_texture: TextureHandle,
        region: BufferTextureCopy,
    },
    BeginRenderPass {
        name: String,
    },
    EndRenderPass,
    SetPipeline {
        pipeline_id: u64,
    },
    SetViewport {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        min_depth: f32,
        max_depth: f32,
    },
    SetScissor {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
    PushConstants {
        offset: u32,
        data: Vec<u8>,
    },
}

/// Resource barrier.
#[derive(Debug, Clone)]
pub struct ResourceBarrier {
    /// Resource type.
    pub resource: BarrierResource,
    /// State before.
    pub state_before: ResourceState,
    /// State after.
    pub state_after: ResourceState,
}

/// Barrier resource.
#[derive(Debug, Clone)]
pub enum BarrierResource {
    Buffer(BufferHandle),
    Texture(TextureHandle),
}

/// Resource state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceState {
    Common,
    VertexBuffer,
    IndexBuffer,
    ConstantBuffer,
    ShaderResource,
    UnorderedAccess,
    RenderTarget,
    DepthWrite,
    DepthRead,
    CopySrc,
    CopyDst,
    Present,
}

/// Buffer to texture copy region.
#[derive(Debug, Clone)]
pub struct BufferTextureCopy {
    /// Buffer offset.
    pub buffer_offset: u64,
    /// Buffer row pitch.
    pub buffer_row_pitch: u32,
    /// Buffer slice pitch.
    pub buffer_slice_pitch: u32,
    /// Texture offset.
    pub texture_offset: [u32; 3],
    /// Texture extent.
    pub texture_extent: [u32; 3],
    /// Mip level.
    pub mip_level: u32,
    /// Array layer.
    pub array_layer: u32,
}

/// Scratch memory allocator (per-frame).
struct ScratchAllocator {
    capacity: usize,
    offset: usize,
}

impl ScratchAllocator {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            offset: 0,
        }
    }

    fn reset(&mut self) {
        self.offset = 0;
    }

    fn allocate(&mut self, size: usize, alignment: usize) -> Option<ScratchAllocation> {
        let aligned = (self.offset + alignment - 1) & !(alignment - 1);
        if aligned + size <= self.capacity {
            let allocation = ScratchAllocation {
                offset: aligned,
                size,
            };
            self.offset = aligned + size;
            Some(allocation)
        } else {
            None
        }
    }
}

/// Scratch memory allocation.
#[derive(Debug, Clone)]
pub struct ScratchAllocation {
    /// Offset in scratch buffer.
    pub offset: usize,
    /// Size.
    pub size: usize,
}

/// Upload memory allocator (per-frame).
struct UploadAllocator {
    capacity: usize,
    offset: usize,
}

impl UploadAllocator {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            offset: 0,
        }
    }

    fn reset(&mut self) {
        self.offset = 0;
    }

    fn allocate(&mut self, size: usize, alignment: usize) -> Option<UploadAllocation> {
        let aligned = (self.offset + alignment - 1) & !(alignment - 1);
        if aligned + size <= self.capacity {
            let allocation = UploadAllocation {
                offset: aligned,
                size,
            };
            self.offset = aligned + size;
            Some(allocation)
        } else {
            None
        }
    }
}

/// Descriptor allocators (per-frame).
struct DescriptorAllocators {
    cbv_srv_uav: DescriptorRingAllocator,
    sampler: DescriptorRingAllocator,
    rtv: DescriptorRingAllocator,
    dsv: DescriptorRingAllocator,
}

impl DescriptorAllocators {
    fn new(config: &FrameConfig) -> Self {
        Self {
            cbv_srv_uav: DescriptorRingAllocator::new(config.cbv_srv_uav_count),
            sampler: DescriptorRingAllocator::new(config.sampler_count),
            rtv: DescriptorRingAllocator::new(config.rtv_count),
            dsv: DescriptorRingAllocator::new(config.dsv_count),
        }
    }

    fn reset(&mut self) {
        self.cbv_srv_uav.reset();
        self.sampler.reset();
        self.rtv.reset();
        self.dsv.reset();
    }

    fn allocate(
        &mut self,
        count: u32,
        heap_type: DescriptorHeapType,
    ) -> Option<DescriptorAllocation> {
        match heap_type {
            DescriptorHeapType::CbvSrvUav => self.cbv_srv_uav.allocate(count),
            DescriptorHeapType::Sampler => self.sampler.allocate(count),
            DescriptorHeapType::Rtv => self.rtv.allocate(count),
            DescriptorHeapType::Dsv => self.dsv.allocate(count),
        }
    }
}

/// Descriptor ring allocator.
struct DescriptorRingAllocator {
    capacity: u32,
    offset: u32,
}

impl DescriptorRingAllocator {
    fn new(capacity: u32) -> Self {
        Self {
            capacity,
            offset: 0,
        }
    }

    fn reset(&mut self) {
        self.offset = 0;
    }

    fn allocate(&mut self, count: u32) -> Option<DescriptorAllocation> {
        if self.offset + count <= self.capacity {
            let start = self.offset;
            self.offset += count;
            Some(DescriptorAllocation {
                start,
                count,
                heap_type: DescriptorHeapType::CbvSrvUav, // Will be overwritten
            })
        } else {
            None
        }
    }
}

/// Deferred resource deletion.
#[derive(Debug)]
pub enum DeferredDeletion {
    Buffer(BufferHandle),
    Texture(TextureHandle),
    Pipeline(u64),
    Custom { data: u64, destructor: fn(u64) },
}

/// Frame graph builder.
pub struct FrameGraphBuilder {
    /// Passes.
    passes: Vec<FrameGraphPass>,
    /// Resources.
    resources: Vec<FrameGraphResource>,
}

impl FrameGraphBuilder {
    /// Create new builder.
    pub fn new() -> Self {
        Self {
            passes: Vec::new(),
            resources: Vec::new(),
        }
    }

    /// Add a pass.
    pub fn add_pass(&mut self, name: &str, pass_type: PassType) -> PassHandle {
        let handle = PassHandle(self.passes.len() as u32);
        self.passes.push(FrameGraphPass {
            name: String::from(name),
            pass_type,
            reads: Vec::new(),
            writes: Vec::new(),
        });
        handle
    }

    /// Declare a resource.
    pub fn create_resource(&mut self, name: &str, desc: ResourceDesc) -> ResourceHandle {
        let handle = ResourceHandle(self.resources.len() as u32);
        self.resources.push(FrameGraphResource {
            name: String::from(name),
            desc,
            producer: None,
            consumers: Vec::new(),
        });
        handle
    }

    /// Mark pass reading resource.
    pub fn read(&mut self, pass: PassHandle, resource: ResourceHandle) {
        self.passes[pass.0 as usize].reads.push(resource);
        self.resources[resource.0 as usize].consumers.push(pass);
    }

    /// Mark pass writing resource.
    pub fn write(&mut self, pass: PassHandle, resource: ResourceHandle) {
        self.passes[pass.0 as usize].writes.push(resource);
        self.resources[resource.0 as usize].producer = Some(pass);
    }

    /// Build the frame graph.
    pub fn build(self) -> FrameGraph {
        // Topological sort and resource lifetime analysis
        FrameGraph {
            passes: self.passes,
            resources: self.resources,
            execution_order: Vec::new(),
        }
    }
}

/// Frame graph pass.
#[derive(Debug)]
struct FrameGraphPass {
    name: String,
    pass_type: PassType,
    reads: Vec<ResourceHandle>,
    writes: Vec<ResourceHandle>,
}

/// Frame graph resource.
#[derive(Debug)]
struct FrameGraphResource {
    name: String,
    desc: ResourceDesc,
    producer: Option<PassHandle>,
    consumers: Vec<PassHandle>,
}

/// Pass handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassHandle(u32);

/// Resource handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceHandle(u32);

/// Pass type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassType {
    Graphics,
    Compute,
    Copy,
    Present,
}

/// Resource description.
#[derive(Debug, Clone)]
pub enum ResourceDesc {
    Buffer(BufferDesc),
    Texture(TextureDesc),
    Imported,
}

/// Compiled frame graph.
pub struct FrameGraph {
    passes: Vec<FrameGraphPass>,
    resources: Vec<FrameGraphResource>,
    execution_order: Vec<u32>,
}

impl FrameGraph {
    /// Execute the frame graph.
    pub fn execute(&self, frame: &mut Frame) {
        for &pass_idx in &self.execution_order {
            let _pass = &self.passes[pass_idx as usize];
            // Would execute pass
        }
    }
}

