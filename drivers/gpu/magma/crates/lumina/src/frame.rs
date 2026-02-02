//! Frame management and render operations
//!
//! This module provides the `Frame` type which represents a single frame
//! being rendered. It provides a fluent API for recording render operations.

use alloc::vec::Vec;

use crate::buffer::{GpuBuffer, GpuSlice, GpuSliceMut};
use crate::color::{ClearValue, Color};
use crate::graph::{
    AccessFlags, Attachment, ImageLayout, LoadOp, PipelineStages, RenderGraph, RenderNode,
    ResourceState, StoreOp,
};
use crate::mesh::GpuMesh;
use crate::pipeline::{BlendMode, CullMode, DepthTest};
use crate::texture::GpuTexture;
use crate::types::GpuData;

/// Represents a single frame being rendered
///
/// A `Frame` is obtained from `Lumina::begin_frame()` and provides
/// methods for recording render and compute operations.
/// When dropped, the frame is automatically presented.
pub struct Frame<'a> {
    /// Index of this frame in the swapchain
    pub(crate) frame_index: u32,
    /// Time since application start (seconds)
    pub(crate) time: f32,
    /// Delta time since last frame (seconds)
    pub(crate) delta_time: f32,
    /// The render graph being built
    pub(crate) graph: &'a mut RenderGraph,
    /// Width of the swapchain image
    pub(crate) width: u32,
    /// Height of the swapchain image
    pub(crate) height: u32,
}

impl<'a> Frame<'a> {
    /// Returns the frame index
    #[inline]
    pub fn index(&self) -> u32 {
        self.frame_index
    }

    /// Returns the time since application start in seconds
    #[inline]
    pub fn time(&self) -> f32 {
        self.time
    }

    /// Returns the delta time since last frame in seconds
    #[inline]
    pub fn delta_time(&self) -> f32 {
        self.delta_time
    }

    /// Returns the swapchain width
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the swapchain height
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns the aspect ratio
    #[inline]
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    /// Begins a render operation to the swapchain
    ///
    /// This is the primary way to draw to the screen.
    ///
    /// # Example
    ///
    /// ```rust
    /// frame
    ///     .render()
    ///     .clear(Color::BLACK)
    ///     .draw(&mesh)
    ///     .with(vertex_shader, fragment_shader)
    ///     .submit();
    /// ```
    pub fn render(&mut self) -> RenderBuilder<'_, 'a> {
        RenderBuilder::new(self)
    }

    /// Begins a render operation to a custom render target
    ///
    /// Use this for offscreen rendering (G-buffers, shadow maps, etc.)
    pub fn render_to<'b, T>(&'b mut self, target: &'b T) -> RenderToBuilder<'b, 'a, T> {
        RenderToBuilder::new(self, target)
    }

    /// Begins a compute operation
    ///
    /// # Example
    ///
    /// ```rust
    /// frame
    ///     .compute()
    ///     .dispatch(compute_kernel)
    ///     .args(&mut buffer, delta_time)
    ///     .groups(buffer.len() / 256, 1, 1);
    /// ```
    pub fn compute(&mut self) -> ComputeBuilder<'_, 'a> {
        ComputeBuilder::new(self)
    }

    /// Allocates a temporary buffer for this frame only
    ///
    /// The buffer will be recycled when the frame ends.
    pub fn allocate_temp<T: GpuData>(&mut self, count: usize) -> GpuBuffer<T> {
        // TODO: Use a ring buffer allocator
        GpuBuffer::new(count, crate::buffer::BufferUsage::Storage)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RENDER BUILDER
// ═══════════════════════════════════════════════════════════════════════════

/// Builder for render operations to the swapchain
pub struct RenderBuilder<'b, 'a> {
    frame: &'b mut Frame<'a>,
    clear_color: Option<Color>,
    clear_depth: Option<f32>,
    depth_test: Option<DepthTest>,
    cull_mode: Option<CullMode>,
    blend_mode: Option<BlendMode>,
}

impl<'b, 'a> RenderBuilder<'b, 'a> {
    pub(crate) fn new(frame: &'b mut Frame<'a>) -> Self {
        Self {
            frame,
            clear_color: None,
            clear_depth: None,
            depth_test: None,
            cull_mode: None,
            blend_mode: None,
        }
    }

    /// Clears the color attachment
    pub fn clear(mut self, color: Color) -> Self {
        self.clear_color = Some(color);
        self
    }

    /// Clears the depth attachment
    pub fn clear_depth(mut self, depth: f32) -> Self {
        self.clear_depth = Some(depth);
        self
    }

    /// Sets the depth test mode
    pub fn depth_test(mut self, test: DepthTest) -> Self {
        self.depth_test = Some(test);
        self
    }

    /// Sets the cull mode
    pub fn cull(mut self, mode: CullMode) -> Self {
        self.cull_mode = Some(mode);
        self
    }

    /// Sets the blend mode
    pub fn blend(mut self, mode: BlendMode) -> Self {
        self.blend_mode = Some(mode);
        self
    }

    /// Draws a mesh
    pub fn draw(self, mesh: &GpuMesh) -> DrawBuilder<'b, 'a> {
        DrawBuilder {
            render: self,
            mesh: Some(mesh),
            instances: 1,
        }
    }

    /// Draws a buffer of points
    pub fn draw_points<T: GpuData>(self, buffer: &GpuBuffer<T>) -> DrawPointsBuilder<'b, 'a, T> {
        DrawPointsBuilder {
            render: self,
            buffer,
        }
    }

    /// Draws a fullscreen quad (for post-processing)
    pub fn draw_fullscreen(self) -> DrawFullscreenBuilder<'b, 'a> {
        DrawFullscreenBuilder { render: self }
    }

    /// Submits the render operation
    pub fn submit(self) {
        // Begin render pass
        let load_op = if self.clear_color.is_some() {
            LoadOp::Clear
        } else {
            LoadOp::Load
        };

        self.frame.graph.add_node(RenderNode::BeginRenderPass {
            color_attachments: alloc::vec![Attachment {
                resource: crate::graph::ResourceId::new(0), // Swapchain
                load_op,
                store_op: StoreOp::Store,
                clear_value: self.clear_color.map(ClearValue::Color).unwrap_or_default(),
            }],
            depth_attachment: self.clear_depth.map(|depth| Attachment {
                resource: crate::graph::ResourceId::new(1), // Depth buffer
                load_op: LoadOp::Clear,
                store_op: StoreOp::DontCare,
                clear_value: ClearValue::Depth(depth),
            }),
        });

        // End render pass
        self.frame.graph.add_node(RenderNode::EndRenderPass);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// DRAW BUILDERS
// ═══════════════════════════════════════════════════════════════════════════

/// Builder for drawing a mesh
pub struct DrawBuilder<'b, 'a> {
    render: RenderBuilder<'b, 'a>,
    mesh: Option<&'b GpuMesh>,
    instances: u32,
}

impl<'b, 'a> DrawBuilder<'b, 'a> {
    /// Sets the shaders for this draw call
    ///
    /// The shaders are automatically compiled from `#[lumina::shader]` functions.
    pub fn with<V, F>(self, _vertex: V, _fragment: F) -> DrawWithShadersBuilder<'b, 'a> {
        DrawWithShadersBuilder { draw: self }
    }

    /// Sets the number of instances
    pub fn instances(mut self, count: u32) -> Self {
        self.instances = count;
        self
    }
}

/// Builder for drawing with shaders
pub struct DrawWithShadersBuilder<'b, 'a> {
    draw: DrawBuilder<'b, 'a>,
}

impl<'b, 'a> DrawWithShadersBuilder<'b, 'a> {
    /// Binds uniforms for this draw call
    pub fn uniforms<U: crate::types::GpuUniforms>(self, _uniforms: &U) -> Self {
        // TODO: Record uniform binding
        self
    }

    /// Binds a texture
    pub fn bind_texture<F>(self, _texture: &GpuTexture<F>) -> Self {
        // TODO: Record texture binding
        self
    }

    /// Sets depth test mode
    pub fn depth_test(mut self, test: DepthTest) -> Self {
        self.draw.render.depth_test = Some(test);
        self
    }

    /// Sets cull mode
    pub fn cull(mut self, mode: CullMode) -> Self {
        self.draw.render.cull_mode = Some(mode);
        self
    }

    /// Sets blend mode
    pub fn blend(mut self, mode: BlendMode) -> Self {
        self.draw.render.blend_mode = Some(mode);
        self
    }

    /// Submits the draw call
    pub fn submit(self) {
        let draw = self.draw;
        let render = draw.render;

        // Begin render pass
        let load_op = if render.clear_color.is_some() {
            LoadOp::Clear
        } else {
            LoadOp::Load
        };

        render.frame.graph.add_node(RenderNode::BeginRenderPass {
            color_attachments: alloc::vec![Attachment {
                resource: crate::graph::ResourceId::new(0),
                load_op,
                store_op: StoreOp::Store,
                clear_value: render
                    .clear_color
                    .map(ClearValue::Color)
                    .unwrap_or_default(),
            }],
            depth_attachment: render.clear_depth.map(|depth| Attachment {
                resource: crate::graph::ResourceId::new(1),
                load_op: LoadOp::Clear,
                store_op: StoreOp::DontCare,
                clear_value: ClearValue::Depth(depth),
            }),
        });

        // TODO: Bind pipeline, vertex buffers, draw

        if let Some(mesh) = draw.mesh {
            let index_count = mesh.index_count() as u32;
            if index_count > 0 {
                render.frame.graph.add_node(RenderNode::DrawIndexed {
                    index_count,
                    instance_count: draw.instances,
                    first_index: 0,
                    vertex_offset: 0,
                    first_instance: 0,
                });
            } else {
                render.frame.graph.add_node(RenderNode::Draw {
                    vertex_count: mesh.vertex_count() as u32,
                    instance_count: draw.instances,
                    first_vertex: 0,
                    first_instance: 0,
                });
            }
        }

        // End render pass
        render.frame.graph.add_node(RenderNode::EndRenderPass);
    }
}

/// Builder for drawing points
pub struct DrawPointsBuilder<'b, 'a, T: GpuData> {
    render: RenderBuilder<'b, 'a>,
    buffer: &'b GpuBuffer<T>,
}

impl<'b, 'a, T: GpuData> DrawPointsBuilder<'b, 'a, T> {
    /// Submits the draw call
    pub fn submit(self) {
        let count = self.buffer.len() as u32;

        self.render.frame.graph.add_node(RenderNode::Draw {
            vertex_count: count,
            instance_count: 1,
            first_vertex: 0,
            first_instance: 0,
        });
    }
}

/// Builder for fullscreen draws
pub struct DrawFullscreenBuilder<'b, 'a> {
    render: RenderBuilder<'b, 'a>,
}

impl<'b, 'a> DrawFullscreenBuilder<'b, 'a> {
    /// Sets the fragment shader
    pub fn with<F>(self, _fragment: F) -> DrawFullscreenWithShaderBuilder<'b, 'a> {
        DrawFullscreenWithShaderBuilder { draw: self }
    }
}

/// Builder for fullscreen draw with shader
pub struct DrawFullscreenWithShaderBuilder<'b, 'a> {
    draw: DrawFullscreenBuilder<'b, 'a>,
}

impl<'b, 'a> DrawFullscreenWithShaderBuilder<'b, 'a> {
    /// Binds resources
    pub fn bind<R>(self, _resource: &R) -> Self {
        self
    }

    /// Submits the draw call
    pub fn submit(self) {
        // Draw fullscreen triangle
        self.draw.render.frame.graph.add_node(RenderNode::Draw {
            vertex_count: 3,
            instance_count: 1,
            first_vertex: 0,
            first_instance: 0,
        });
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RENDER TO BUILDER
// ═══════════════════════════════════════════════════════════════════════════

/// Builder for rendering to custom targets
pub struct RenderToBuilder<'b, 'a, T> {
    frame: &'b mut Frame<'a>,
    target: &'b T,
}

impl<'b, 'a, T> RenderToBuilder<'b, 'a, T> {
    pub(crate) fn new(frame: &'b mut Frame<'a>, target: &'b T) -> Self {
        Self { frame, target }
    }

    /// Clears all attachments
    pub fn clear_all(self) -> Self {
        self
    }

    /// Draws a mesh
    pub fn draw(self, _meshes: &[GpuMesh]) -> RenderToDrawBuilder<'b, 'a, T> {
        RenderToDrawBuilder { render: self }
    }
}

/// Builder for drawing to custom targets
pub struct RenderToDrawBuilder<'b, 'a, T> {
    render: RenderToBuilder<'b, 'a, T>,
}

impl<'b, 'a, T> RenderToDrawBuilder<'b, 'a, T> {
    /// Sets the shaders
    pub fn with<V, F>(self, _vertex: V, _fragment: F) -> Self {
        self
    }

    /// Submits the render operation
    pub fn submit(self) {
        // TODO: Record render pass to custom target
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// COMPUTE BUILDER
// ═══════════════════════════════════════════════════════════════════════════

/// Builder for compute operations
pub struct ComputeBuilder<'b, 'a> {
    frame: &'b mut Frame<'a>,
}

impl<'b, 'a> ComputeBuilder<'b, 'a> {
    pub(crate) fn new(frame: &'b mut Frame<'a>) -> Self {
        Self { frame }
    }

    /// Dispatches a compute kernel
    pub fn dispatch<K>(self, _kernel: K) -> DispatchBuilder<'b, 'a> {
        DispatchBuilder { frame: self.frame }
    }
}

/// Builder for compute dispatch
pub struct DispatchBuilder<'b, 'a> {
    frame: &'b mut Frame<'a>,
}

impl<'b, 'a> DispatchBuilder<'b, 'a> {
    /// Binds a buffer for reading
    pub fn read<T: GpuData>(self, _buffer: &GpuBuffer<T>) -> Self {
        // Record read dependency
        self
    }

    /// Binds a buffer for writing
    pub fn write<T: GpuData>(self, _buffer: &mut GpuBuffer<T>) -> Self {
        // Record write dependency
        self
    }

    /// Sets kernel arguments
    pub fn args<A>(self, _args: A) -> Self {
        self
    }

    /// Dispatches the compute kernel
    pub fn groups(self, x: u32, y: u32, z: u32) {
        self.frame.graph.add_node(RenderNode::Dispatch {
            group_count_x: x,
            group_count_y: y,
            group_count_z: z,
        });
    }

    /// Submits as an async compute operation
    pub fn submit_async(self) -> ComputeHandle {
        // TODO: Record async compute
        ComputeHandle { id: 0 }
    }
}

/// Handle to an async compute operation
pub struct ComputeHandle {
    id: u32,
}
