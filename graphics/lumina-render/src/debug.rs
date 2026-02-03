//! Debug Rendering - Visualization & Profiling
//!
//! Debug tools for rendering:
//! - Debug primitives (lines, boxes, spheres)
//! - GPU profiling and timing
//! - Render pass visualization
//! - Resource inspection
//! - Performance overlays

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use crate::graph::{RenderGraph, VirtualTextureHandle};
use crate::resource::{BufferDesc, TextureDesc, TextureFormat};

/// Debug renderer for visualization.
pub struct DebugRenderer {
    /// Configuration.
    config: DebugConfig,
    /// Line buffer.
    lines: Vec<DebugLine>,
    /// Box buffer.
    boxes: Vec<DebugBox>,
    /// Sphere buffer.
    spheres: Vec<DebugSphere>,
    /// Text labels.
    labels: Vec<DebugLabel>,
    /// Profiler.
    profiler: GpuProfiler,
    /// Statistics.
    stats: RenderStats,
}

impl DebugRenderer {
    /// Create new debug renderer.
    pub fn new(config: DebugConfig) -> Self {
        Self {
            config,
            lines: Vec::new(),
            boxes: Vec::new(),
            spheres: Vec::new(),
            labels: Vec::new(),
            profiler: GpuProfiler::new(config.profiler_query_count),
            stats: RenderStats::default(),
        }
    }

    /// Draw a line.
    pub fn line(&mut self, start: [f32; 3], end: [f32; 3], color: [f32; 4]) {
        self.lines.push(DebugLine { start, end, color });
    }

    /// Draw a wire box.
    pub fn wire_box(&mut self, min: [f32; 3], max: [f32; 3], color: [f32; 4]) {
        self.boxes.push(DebugBox {
            min,
            max,
            color,
            filled: false,
        });
    }

    /// Draw a filled box.
    pub fn solid_box(&mut self, min: [f32; 3], max: [f32; 3], color: [f32; 4]) {
        self.boxes.push(DebugBox {
            min,
            max,
            color,
            filled: true,
        });
    }

    /// Draw a wire sphere.
    pub fn wire_sphere(&mut self, center: [f32; 3], radius: f32, color: [f32; 4]) {
        self.spheres.push(DebugSphere {
            center,
            radius,
            color,
            filled: false,
        });
    }

    /// Draw a solid sphere.
    pub fn solid_sphere(&mut self, center: [f32; 3], radius: f32, color: [f32; 4]) {
        self.spheres.push(DebugSphere {
            center,
            radius,
            color,
            filled: true,
        });
    }

    /// Draw a frustum.
    pub fn frustum(&mut self, corners: [[f32; 3]; 8], color: [f32; 4]) {
        // Near plane
        self.line(corners[0], corners[1], color);
        self.line(corners[1], corners[3], color);
        self.line(corners[3], corners[2], color);
        self.line(corners[2], corners[0], color);

        // Far plane
        self.line(corners[4], corners[5], color);
        self.line(corners[5], corners[7], color);
        self.line(corners[7], corners[6], color);
        self.line(corners[6], corners[4], color);

        // Connecting edges
        self.line(corners[0], corners[4], color);
        self.line(corners[1], corners[5], color);
        self.line(corners[2], corners[6], color);
        self.line(corners[3], corners[7], color);
    }

    /// Draw an arrow.
    pub fn arrow(&mut self, start: [f32; 3], end: [f32; 3], color: [f32; 4]) {
        self.line(start, end, color);

        // Arrow head
        let dir = normalize(sub(end, start));
        let len = length(sub(end, start));
        let head_len = len * 0.1;

        // Simple perpendicular vectors
        let perp1 = if dir[0].abs() < 0.9 {
            normalize(cross(dir, [1.0, 0.0, 0.0]))
        } else {
            normalize(cross(dir, [0.0, 1.0, 0.0]))
        };
        let perp2 = cross(dir, perp1);

        let head_base = [
            end[0] - dir[0] * head_len,
            end[1] - dir[1] * head_len,
            end[2] - dir[2] * head_len,
        ];

        let head_radius = head_len * 0.3;
        for i in 0..4 {
            let angle = i as f32 * core::f32::consts::PI * 0.5;
            let offset = [
                perp1[0] * angle.cos() + perp2[0] * angle.sin(),
                perp1[1] * angle.cos() + perp2[1] * angle.sin(),
                perp1[2] * angle.cos() + perp2[2] * angle.sin(),
            ];
            let point = [
                head_base[0] + offset[0] * head_radius,
                head_base[1] + offset[1] * head_radius,
                head_base[2] + offset[2] * head_radius,
            ];
            self.line(end, point, color);
        }
    }

    /// Draw coordinate axes.
    pub fn axes(&mut self, origin: [f32; 3], size: f32) {
        self.arrow(origin, [origin[0] + size, origin[1], origin[2]], [
            1.0, 0.0, 0.0, 1.0,
        ]);
        self.arrow(origin, [origin[0], origin[1] + size, origin[2]], [
            0.0, 1.0, 0.0, 1.0,
        ]);
        self.arrow(origin, [origin[0], origin[1], origin[2] + size], [
            0.0, 0.0, 1.0, 1.0,
        ]);
    }

    /// Draw a grid.
    pub fn grid(&mut self, center: [f32; 3], size: f32, divisions: u32, color: [f32; 4]) {
        let half_size = size * 0.5;
        let step = size / divisions as f32;

        for i in 0..=divisions {
            let offset = -half_size + step * i as f32;

            // X-aligned lines
            self.line(
                [center[0] - half_size, center[1], center[2] + offset],
                [center[0] + half_size, center[1], center[2] + offset],
                color,
            );

            // Z-aligned lines
            self.line(
                [center[0] + offset, center[1], center[2] - half_size],
                [center[0] + offset, center[1], center[2] + half_size],
                color,
            );
        }
    }

    /// Add a text label.
    pub fn label(&mut self, position: [f32; 3], text: &str, color: [f32; 4]) {
        self.labels.push(DebugLabel {
            position,
            text: String::from(text),
            color,
        });
    }

    /// Clear all debug primitives.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.boxes.clear();
        self.spheres.clear();
        self.labels.clear();
    }

    /// Add debug render pass to graph.
    pub fn add_pass(
        &self,
        graph: &mut RenderGraph,
        color: VirtualTextureHandle,
        depth: VirtualTextureHandle,
    ) {
        // Upload line vertices
        if !self.lines.is_empty() {
            let line_buffer =
                graph.create_buffer(BufferDesc::vertex((self.lines.len() * 2 * 32) as u64));

            graph.add_graphics_pass("debug_lines", |builder| {
                builder
                    .read_buffer(line_buffer)
                    .color_attachment(color)
                    .depth_attachment(depth);
            });
        }

        // Render boxes
        if !self.boxes.is_empty() {
            graph.add_graphics_pass("debug_boxes", |builder| {
                builder.color_attachment(color).depth_attachment(depth);
            });
        }

        // Render spheres
        if !self.spheres.is_empty() {
            graph.add_graphics_pass("debug_spheres", |builder| {
                builder.color_attachment(color).depth_attachment(depth);
            });
        }
    }

    /// Get profiler.
    pub fn profiler(&mut self) -> &mut GpuProfiler {
        &mut self.profiler
    }

    /// Get statistics.
    pub fn stats(&self) -> &RenderStats {
        &self.stats
    }

    /// Update statistics.
    pub fn update_stats(&mut self, stats: RenderStats) {
        self.stats = stats;
    }
}

/// Debug configuration.
#[derive(Debug, Clone)]
pub struct DebugConfig {
    /// Maximum lines.
    pub max_lines: usize,
    /// Maximum boxes.
    pub max_boxes: usize,
    /// Maximum spheres.
    pub max_spheres: usize,
    /// Profiler query count.
    pub profiler_query_count: u32,
    /// Enable depth test.
    pub depth_test: bool,
    /// Line width.
    pub line_width: f32,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            max_lines: 100000,
            max_boxes: 10000,
            max_spheres: 1000,
            profiler_query_count: 256,
            depth_test: true,
            line_width: 1.0,
        }
    }
}

/// Debug line.
#[derive(Debug, Clone)]
pub struct DebugLine {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub color: [f32; 4],
}

/// Debug box.
#[derive(Debug, Clone)]
pub struct DebugBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub color: [f32; 4],
    pub filled: bool,
}

/// Debug sphere.
#[derive(Debug, Clone)]
pub struct DebugSphere {
    pub center: [f32; 3],
    pub radius: f32,
    pub color: [f32; 4],
    pub filled: bool,
}

/// Debug label.
#[derive(Debug, Clone)]
pub struct DebugLabel {
    pub position: [f32; 3],
    pub text: String,
    pub color: [f32; 4],
}

/// GPU profiler for timing.
pub struct GpuProfiler {
    /// Query capacity.
    capacity: u32,
    /// Active scopes.
    scopes: Vec<ProfileScope>,
    /// Completed results.
    results: Vec<ProfileResult>,
    /// Next query index.
    next_query: u32,
    /// Is enabled.
    enabled: bool,
}

impl GpuProfiler {
    /// Create new profiler.
    pub fn new(capacity: u32) -> Self {
        Self {
            capacity,
            scopes: Vec::new(),
            results: Vec::new(),
            next_query: 0,
            enabled: true,
        }
    }

    /// Begin a profiling scope.
    pub fn begin(&mut self, name: &str) -> ProfileScopeId {
        if !self.enabled || self.next_query + 2 > self.capacity {
            return ProfileScopeId(u32::MAX);
        }

        let id = ProfileScopeId(self.scopes.len() as u32);
        let begin_query = self.next_query;
        self.next_query += 1;

        self.scopes.push(ProfileScope {
            name: String::from(name),
            begin_query,
            end_query: u32::MAX,
            parent: None,
            depth: 0,
        });

        id
    }

    /// End a profiling scope.
    pub fn end(&mut self, id: ProfileScopeId) {
        if id.0 as usize >= self.scopes.len() {
            return;
        }

        let end_query = self.next_query;
        self.next_query += 1;
        self.scopes[id.0 as usize].end_query = end_query;
    }

    /// Reset for new frame.
    pub fn reset(&mut self) {
        self.scopes.clear();
        self.next_query = 0;
    }

    /// Read back results.
    pub fn read_results(&mut self, timestamps: &[u64], frequency: u64) {
        self.results.clear();

        for scope in &self.scopes {
            if scope.end_query == u32::MAX {
                continue;
            }

            let begin_ts = timestamps
                .get(scope.begin_query as usize)
                .copied()
                .unwrap_or(0);
            let end_ts = timestamps
                .get(scope.end_query as usize)
                .copied()
                .unwrap_or(0);

            let duration_ns = if end_ts > begin_ts {
                ((end_ts - begin_ts) as f64 / frequency as f64 * 1_000_000_000.0) as u64
            } else {
                0
            };

            self.results.push(ProfileResult {
                name: scope.name.clone(),
                duration_ns,
                depth: scope.depth,
            });
        }
    }

    /// Get results.
    pub fn results(&self) -> &[ProfileResult] {
        &self.results
    }

    /// Enable/disable profiler.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Is profiler enabled?
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get total GPU time in milliseconds.
    pub fn total_gpu_time_ms(&self) -> f64 {
        let total_ns: u64 = self
            .results
            .iter()
            .filter(|r| r.depth == 0)
            .map(|r| r.duration_ns)
            .sum();
        total_ns as f64 / 1_000_000.0
    }
}

/// Profile scope ID.
#[derive(Debug, Clone, Copy)]
pub struct ProfileScopeId(u32);

/// Profile scope.
struct ProfileScope {
    name: String,
    begin_query: u32,
    end_query: u32,
    parent: Option<u32>,
    depth: u32,
}

/// Profile result.
#[derive(Debug, Clone)]
pub struct ProfileResult {
    /// Scope name.
    pub name: String,
    /// Duration in nanoseconds.
    pub duration_ns: u64,
    /// Nesting depth.
    pub depth: u32,
}

impl ProfileResult {
    /// Get duration in milliseconds.
    pub fn duration_ms(&self) -> f64 {
        self.duration_ns as f64 / 1_000_000.0
    }
}

/// Render statistics.
#[derive(Debug, Clone, Default)]
pub struct RenderStats {
    /// Draw calls.
    pub draw_calls: u32,
    /// Dispatch calls.
    pub dispatch_calls: u32,
    /// Triangles rendered.
    pub triangles: u64,
    /// Vertices processed.
    pub vertices: u64,
    /// Pixels shaded.
    pub pixels: u64,
    /// Texture memory used.
    pub texture_memory: u64,
    /// Buffer memory used.
    pub buffer_memory: u64,
    /// Frame time in milliseconds.
    pub frame_time_ms: f64,
    /// GPU time in milliseconds.
    pub gpu_time_ms: f64,
    /// Pass statistics.
    pub passes: Vec<PassStats>,
}

impl RenderStats {
    /// Get FPS.
    pub fn fps(&self) -> f64 {
        1000.0 / self.frame_time_ms.max(0.001)
    }

    /// Get total memory in MB.
    pub fn total_memory_mb(&self) -> f64 {
        (self.texture_memory + self.buffer_memory) as f64 / (1024.0 * 1024.0)
    }
}

/// Per-pass statistics.
#[derive(Debug, Clone)]
pub struct PassStats {
    /// Pass name.
    pub name: String,
    /// GPU time in milliseconds.
    pub gpu_time_ms: f64,
    /// Draw calls.
    pub draw_calls: u32,
    /// Triangles.
    pub triangles: u64,
}

/// Debug visualization modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugVisualization {
    /// No debug visualization.
    None,
    /// Albedo only.
    Albedo,
    /// Normals.
    Normals,
    /// Roughness.
    Roughness,
    /// Metallic.
    Metallic,
    /// Depth.
    Depth,
    /// Motion vectors.
    MotionVectors,
    /// Wireframe.
    Wireframe,
    /// Overdraw.
    Overdraw,
    /// Meshlet colors.
    Meshlets,
    /// Triangle colors.
    Triangles,
    /// LOD levels.
    LodLevels,
    /// Light complexity.
    LightComplexity,
    /// Shadow cascades.
    ShadowCascades,
    /// GI probes.
    GIProbes,
}

/// Debug visualization renderer.
pub struct DebugVisualizationRenderer {
    /// Current mode.
    mode: DebugVisualization,
}

impl DebugVisualizationRenderer {
    /// Create new renderer.
    pub fn new() -> Self {
        Self {
            mode: DebugVisualization::None,
        }
    }

    /// Set visualization mode.
    pub fn set_mode(&mut self, mode: DebugVisualization) {
        self.mode = mode;
    }

    /// Get current mode.
    pub fn mode(&self) -> DebugVisualization {
        self.mode
    }

    /// Add visualization pass.
    pub fn add_pass(
        &self,
        graph: &mut RenderGraph,
        gbuffer_albedo: VirtualTextureHandle,
        gbuffer_normal: VirtualTextureHandle,
        gbuffer_material: VirtualTextureHandle,
        depth: VirtualTextureHandle,
        motion_vectors: VirtualTextureHandle,
        width: u32,
        height: u32,
    ) -> VirtualTextureHandle {
        let output = graph.create_texture(TextureDesc {
            format: TextureFormat::Rgba8Srgb,
            width,
            height,
            ..Default::default()
        });

        graph.add_compute_pass("debug_visualization", |builder| {
            builder
                .read_texture(gbuffer_albedo)
                .read_texture(gbuffer_normal)
                .read_texture(gbuffer_material)
                .read_texture(depth)
                .read_texture(motion_vectors)
                .storage_image(output);
        });

        output
    }
}

/// Texture inspector for debugging.
pub struct TextureInspector {
    /// Textures to inspect.
    textures: Vec<InspectedTexture>,
    /// Selected texture index.
    selected: Option<usize>,
    /// Display settings.
    settings: InspectorSettings,
}

impl TextureInspector {
    /// Create new inspector.
    pub fn new() -> Self {
        Self {
            textures: Vec::new(),
            selected: None,
            settings: InspectorSettings::default(),
        }
    }

    /// Add texture to inspect.
    pub fn add(&mut self, name: &str, handle: VirtualTextureHandle) {
        self.textures.push(InspectedTexture {
            name: String::from(name),
            handle,
        });
    }

    /// Clear all textures.
    pub fn clear(&mut self) {
        self.textures.clear();
        self.selected = None;
    }

    /// Select texture by index.
    pub fn select(&mut self, index: usize) {
        if index < self.textures.len() {
            self.selected = Some(index);
        }
    }

    /// Get selected texture.
    pub fn selected(&self) -> Option<&InspectedTexture> {
        self.selected.and_then(|i| self.textures.get(i))
    }

    /// Get settings.
    pub fn settings(&self) -> &InspectorSettings {
        &self.settings
    }

    /// Get mutable settings.
    pub fn settings_mut(&mut self) -> &mut InspectorSettings {
        &mut self.settings
    }
}

/// Inspected texture.
#[derive(Debug, Clone)]
pub struct InspectedTexture {
    /// Texture name.
    pub name: String,
    /// Texture handle.
    pub handle: VirtualTextureHandle,
}

/// Texture inspector settings.
#[derive(Debug, Clone)]
pub struct InspectorSettings {
    /// Mip level to display.
    pub mip_level: u32,
    /// Array layer to display.
    pub array_layer: u32,
    /// Channel mask (RGBA).
    pub channel_mask: [bool; 4],
    /// Exposure.
    pub exposure: f32,
    /// Range remap min.
    pub range_min: f32,
    /// Range remap max.
    pub range_max: f32,
}

impl Default for InspectorSettings {
    fn default() -> Self {
        Self {
            mip_level: 0,
            array_layer: 0,
            channel_mask: [true, true, true, true],
            exposure: 1.0,
            range_min: 0.0,
            range_max: 1.0,
        }
    }
}

// Helper functions

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = length(v);
    if len > 0.0 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_renderer() {
        let mut renderer = DebugRenderer::new(DebugConfig::default());

        renderer.line([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [1.0, 0.0, 0.0, 1.0]);
        renderer.wire_box([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0], [0.0, 1.0, 0.0, 1.0]);
        renderer.wire_sphere([0.0, 0.0, 0.0], 1.0, [0.0, 0.0, 1.0, 1.0]);

        assert_eq!(renderer.lines.len(), 1);
        assert_eq!(renderer.boxes.len(), 1);
        assert_eq!(renderer.spheres.len(), 1);

        renderer.clear();
        assert!(renderer.lines.is_empty());
    }

    #[test]
    fn test_gpu_profiler() {
        let mut profiler = GpuProfiler::new(256);

        let scope1 = profiler.begin("Pass1");
        profiler.end(scope1);

        let scope2 = profiler.begin("Pass2");
        profiler.end(scope2);

        assert_eq!(profiler.scopes.len(), 2);
    }

    #[test]
    fn test_profile_result() {
        let result = ProfileResult {
            name: String::from("test"),
            duration_ns: 1_500_000,
            depth: 0,
        };

        assert!((result.duration_ms() - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_render_stats() {
        let stats = RenderStats {
            frame_time_ms: 16.67,
            texture_memory: 256 * 1024 * 1024,
            buffer_memory: 64 * 1024 * 1024,
            ..Default::default()
        };

        assert!((stats.fps() - 60.0).abs() < 1.0);
        assert!((stats.total_memory_mb() - 320.0).abs() < 1.0);
    }
}
