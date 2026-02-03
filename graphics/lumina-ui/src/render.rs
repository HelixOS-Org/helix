//! # GPU Renderer
//!
//! GPU rendering backend for UI.

use alloc::vec::Vec;

use crate::{Color, Rect, RenderCommand, TextStyle};

/// UI renderer
pub struct UiRenderer {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    draw_calls: Vec<DrawCall>,
    clip_stack: Vec<Rect>,
}

impl UiRenderer {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            draw_calls: Vec::new(),
            clip_stack: Vec::new(),
        }
    }

    /// Begin a new frame
    pub fn begin(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.draw_calls.clear();
        self.clip_stack.clear();
    }

    /// Process render commands
    pub fn process(&mut self, commands: &[RenderCommand]) {
        for command in commands {
            match command {
                RenderCommand::Rect {
                    rect,
                    color,
                    corner_radius,
                } => {
                    self.draw_rect(*rect, *color, *corner_radius);
                },
                RenderCommand::RectOutline {
                    rect,
                    color,
                    thickness,
                    corner_radius,
                } => {
                    self.draw_rect_outline(*rect, *color, *thickness, *corner_radius);
                },
                RenderCommand::Text {
                    position,
                    text,
                    style,
                } => {
                    self.draw_text(*position, text, style);
                },
                RenderCommand::Image {
                    rect,
                    texture,
                    uv,
                    tint,
                } => {
                    self.draw_image(*rect, *texture, *uv, *tint);
                },
                RenderCommand::Line {
                    start,
                    end,
                    color,
                    thickness,
                } => {
                    self.draw_line(*start, *end, *color, *thickness);
                },
                RenderCommand::Triangle { points, color } => {
                    self.draw_triangle(*points, *color);
                },
                RenderCommand::Circle {
                    center,
                    radius,
                    color,
                } => {
                    self.draw_circle(*center, *radius, *color);
                },
                RenderCommand::Clip { rect } => {
                    self.push_clip(*rect);
                },
                RenderCommand::PopClip => {
                    self.pop_clip();
                },
                RenderCommand::Custom { id: _, data: _ } => {
                    // Handle custom commands
                },
            }
        }
    }

    /// End frame and get render data
    pub fn end(&mut self) -> RenderData {
        RenderData {
            vertices: self.vertices.clone(),
            indices: self.indices.clone(),
            draw_calls: self.draw_calls.clone(),
        }
    }

    fn draw_rect(&mut self, rect: Rect, color: Color, corner_radius: f32) {
        let base_idx = self.vertices.len() as u32;

        if corner_radius <= 0.0 {
            // Simple quad
            self.vertices.push(Vertex {
                position: [rect.x, rect.y],
                uv: [0.0, 0.0],
                color: color_to_u32(color),
            });
            self.vertices.push(Vertex {
                position: [rect.x + rect.width, rect.y],
                uv: [1.0, 0.0],
                color: color_to_u32(color),
            });
            self.vertices.push(Vertex {
                position: [rect.x + rect.width, rect.y + rect.height],
                uv: [1.0, 1.0],
                color: color_to_u32(color),
            });
            self.vertices.push(Vertex {
                position: [rect.x, rect.y + rect.height],
                uv: [0.0, 1.0],
                color: color_to_u32(color),
            });

            self.indices.extend_from_slice(&[
                base_idx,
                base_idx + 1,
                base_idx + 2,
                base_idx,
                base_idx + 2,
                base_idx + 3,
            ]);
        } else {
            // Rounded rect with SDF
            self.vertices.push(Vertex {
                position: [rect.x, rect.y],
                uv: [-1.0, -1.0],
                color: color_to_u32(color),
            });
            self.vertices.push(Vertex {
                position: [rect.x + rect.width, rect.y],
                uv: [1.0, -1.0],
                color: color_to_u32(color),
            });
            self.vertices.push(Vertex {
                position: [rect.x + rect.width, rect.y + rect.height],
                uv: [1.0, 1.0],
                color: color_to_u32(color),
            });
            self.vertices.push(Vertex {
                position: [rect.x, rect.y + rect.height],
                uv: [-1.0, 1.0],
                color: color_to_u32(color),
            });

            self.indices.extend_from_slice(&[
                base_idx,
                base_idx + 1,
                base_idx + 2,
                base_idx,
                base_idx + 2,
                base_idx + 3,
            ]);
        }

        self.draw_calls.push(DrawCall {
            draw_type: DrawType::Rect { corner_radius },
            index_offset: self.indices.len() as u32 - 6,
            index_count: 6,
            texture: None,
            clip_rect: self.current_clip(),
        });
    }

    fn draw_rect_outline(&mut self, rect: Rect, color: Color, thickness: f32, _corner_radius: f32) {
        // Draw as 4 lines
        self.draw_line(
            [rect.x, rect.y],
            [rect.x + rect.width, rect.y],
            color,
            thickness,
        );
        self.draw_line(
            [rect.x + rect.width, rect.y],
            [rect.x + rect.width, rect.y + rect.height],
            color,
            thickness,
        );
        self.draw_line(
            [rect.x + rect.width, rect.y + rect.height],
            [rect.x, rect.y + rect.height],
            color,
            thickness,
        );
        self.draw_line(
            [rect.x, rect.y + rect.height],
            [rect.x, rect.y],
            color,
            thickness,
        );
    }

    fn draw_text(&mut self, position: [f32; 2], text: &str, style: &TextStyle) {
        // Would generate text quads using font atlas
        let base_idx = self.vertices.len() as u32;
        let mut x = position[0];
        let char_width = style.size * 0.6;

        for _ in text.chars() {
            self.vertices.push(Vertex {
                position: [x, position[1]],
                uv: [0.0, 0.0],
                color: color_to_u32(style.color),
            });
            self.vertices.push(Vertex {
                position: [x + char_width, position[1]],
                uv: [1.0, 0.0],
                color: color_to_u32(style.color),
            });
            self.vertices.push(Vertex {
                position: [x + char_width, position[1] + style.size],
                uv: [1.0, 1.0],
                color: color_to_u32(style.color),
            });
            self.vertices.push(Vertex {
                position: [x, position[1] + style.size],
                uv: [0.0, 1.0],
                color: color_to_u32(style.color),
            });

            x += char_width;
        }

        let char_count = text.chars().count() as u32;
        for i in 0..char_count {
            let idx = base_idx + i * 4;
            self.indices
                .extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);
        }

        self.draw_calls.push(DrawCall {
            draw_type: DrawType::Text,
            index_offset: self.indices.len() as u32 - char_count * 6,
            index_count: char_count * 6,
            texture: Some(0), // Font atlas
            clip_rect: self.current_clip(),
        });
    }

    fn draw_image(&mut self, rect: Rect, texture: crate::TextureId, uv: [f32; 4], tint: Color) {
        let base_idx = self.vertices.len() as u32;

        self.vertices.push(Vertex {
            position: [rect.x, rect.y],
            uv: [uv[0], uv[1]],
            color: color_to_u32(tint),
        });
        self.vertices.push(Vertex {
            position: [rect.x + rect.width, rect.y],
            uv: [uv[2], uv[1]],
            color: color_to_u32(tint),
        });
        self.vertices.push(Vertex {
            position: [rect.x + rect.width, rect.y + rect.height],
            uv: [uv[2], uv[3]],
            color: color_to_u32(tint),
        });
        self.vertices.push(Vertex {
            position: [rect.x, rect.y + rect.height],
            uv: [uv[0], uv[3]],
            color: color_to_u32(tint),
        });

        self.indices.extend_from_slice(&[
            base_idx,
            base_idx + 1,
            base_idx + 2,
            base_idx,
            base_idx + 2,
            base_idx + 3,
        ]);

        self.draw_calls.push(DrawCall {
            draw_type: DrawType::Image,
            index_offset: self.indices.len() as u32 - 6,
            index_count: 6,
            texture: Some(texture.0),
            clip_rect: self.current_clip(),
        });
    }

    fn draw_line(&mut self, start: [f32; 2], end: [f32; 2], color: Color, thickness: f32) {
        let base_idx = self.vertices.len() as u32;

        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let len = (dx * dx + dy * dy).sqrt();

        if len < 0.001 {
            return;
        }

        let nx = -dy / len * thickness * 0.5;
        let ny = dx / len * thickness * 0.5;

        self.vertices.push(Vertex {
            position: [start[0] + nx, start[1] + ny],
            uv: [0.0, 0.0],
            color: color_to_u32(color),
        });
        self.vertices.push(Vertex {
            position: [end[0] + nx, end[1] + ny],
            uv: [1.0, 0.0],
            color: color_to_u32(color),
        });
        self.vertices.push(Vertex {
            position: [end[0] - nx, end[1] - ny],
            uv: [1.0, 1.0],
            color: color_to_u32(color),
        });
        self.vertices.push(Vertex {
            position: [start[0] - nx, start[1] - ny],
            uv: [0.0, 1.0],
            color: color_to_u32(color),
        });

        self.indices.extend_from_slice(&[
            base_idx,
            base_idx + 1,
            base_idx + 2,
            base_idx,
            base_idx + 2,
            base_idx + 3,
        ]);

        self.draw_calls.push(DrawCall {
            draw_type: DrawType::Line,
            index_offset: self.indices.len() as u32 - 6,
            index_count: 6,
            texture: None,
            clip_rect: self.current_clip(),
        });
    }

    fn draw_triangle(&mut self, points: [[f32; 2]; 3], color: Color) {
        let base_idx = self.vertices.len() as u32;

        for point in &points {
            self.vertices.push(Vertex {
                position: *point,
                uv: [0.0, 0.0],
                color: color_to_u32(color),
            });
        }

        self.indices
            .extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2]);

        self.draw_calls.push(DrawCall {
            draw_type: DrawType::Triangle,
            index_offset: self.indices.len() as u32 - 3,
            index_count: 3,
            texture: None,
            clip_rect: self.current_clip(),
        });
    }

    fn draw_circle(&mut self, center: [f32; 2], radius: f32, color: Color) {
        let base_idx = self.vertices.len() as u32;
        const SEGMENTS: u32 = 32;

        // Center vertex
        self.vertices.push(Vertex {
            position: center,
            uv: [0.5, 0.5],
            color: color_to_u32(color),
        });

        // Perimeter vertices
        for i in 0..SEGMENTS {
            let angle = (i as f32 / SEGMENTS as f32) * core::f32::consts::TAU;
            self.vertices.push(Vertex {
                position: [
                    center[0] + angle.cos() * radius,
                    center[1] + angle.sin() * radius,
                ],
                uv: [0.5 + angle.cos() * 0.5, 0.5 + angle.sin() * 0.5],
                color: color_to_u32(color),
            });
        }

        // Indices
        for i in 0..SEGMENTS {
            self.indices.push(base_idx);
            self.indices.push(base_idx + 1 + i);
            self.indices.push(base_idx + 1 + (i + 1) % SEGMENTS);
        }

        self.draw_calls.push(DrawCall {
            draw_type: DrawType::Circle,
            index_offset: self.indices.len() as u32 - SEGMENTS * 3,
            index_count: SEGMENTS * 3,
            texture: None,
            clip_rect: self.current_clip(),
        });
    }

    fn push_clip(&mut self, rect: Rect) {
        let clipped = if let Some(current) = self.clip_stack.last() {
            intersect_rects(*current, rect)
        } else {
            Some(rect)
        };

        if let Some(r) = clipped {
            self.clip_stack.push(r);
        }
    }

    fn pop_clip(&mut self) {
        self.clip_stack.pop();
    }

    fn current_clip(&self) -> Option<Rect> {
        self.clip_stack.last().copied()
    }
}

impl Default for UiRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Vertex format
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: u32,
}

/// Render data for GPU submission
#[derive(Debug, Clone)]
pub struct RenderData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub draw_calls: Vec<DrawCall>,
}

/// Draw call
#[derive(Debug, Clone)]
pub struct DrawCall {
    pub draw_type: DrawType,
    pub index_offset: u32,
    pub index_count: u32,
    pub texture: Option<u64>,
    pub clip_rect: Option<Rect>,
}

/// Draw type
#[derive(Debug, Clone, Copy)]
pub enum DrawType {
    Rect { corner_radius: f32 },
    Line,
    Triangle,
    Circle,
    Text,
    Image,
}

fn color_to_u32(color: Color) -> u32 {
    let r = (color.r * 255.0) as u32;
    let g = (color.g * 255.0) as u32;
    let b = (color.b * 255.0) as u32;
    let a = (color.a * 255.0) as u32;
    (a << 24) | (b << 16) | (g << 8) | r
}

fn intersect_rects(a: Rect, b: Rect) -> Option<Rect> {
    let x1 = a.x.max(b.x);
    let y1 = a.y.max(b.y);
    let x2 = (a.x + a.width).min(b.x + b.width);
    let y2 = (a.y + a.height).min(b.y + b.height);

    if x2 > x1 && y2 > y1 {
        Some(Rect::new(x1, y1, x2 - x1, y2 - y1))
    } else {
        None
    }
}
