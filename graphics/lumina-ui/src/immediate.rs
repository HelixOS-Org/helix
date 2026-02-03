//! # Immediate Mode UI
//!
//! Simple, stateless UI for tools and debug overlays.

use alloc::string::String;
use alloc::vec::Vec;

use crate::{Color, FontId, Rect, TextAlign, TextStyle, UiContext, WidgetId};

/// Immediate mode UI helper
pub struct Immediate<'a> {
    ctx: &'a mut UiContext,
    cursor: [f32; 2],
    line_height: f32,
    column_width: f32,
}

impl<'a> Immediate<'a> {
    /// Create immediate mode context
    pub fn new(ctx: &'a mut UiContext) -> Self {
        Self {
            ctx,
            cursor: [10.0, 10.0],
            line_height: 20.0,
            column_width: 200.0,
        }
    }

    /// Set cursor position
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.cursor = [x, y];
    }

    /// Move to next line
    pub fn next_line(&mut self) {
        self.cursor[0] = 10.0;
        self.cursor[1] += self.line_height;
    }

    /// Same line (for horizontal layout)
    pub fn same_line(&mut self) {
        self.cursor[0] += self.column_width;
    }

    /// Draw a label
    pub fn label(&mut self, text: &str) {
        self.ctx.draw_text(self.cursor, text, TextStyle {
            color: self.ctx.theme().colors.text,
            ..Default::default()
        });
        self.next_line();
    }

    /// Draw a header
    pub fn header(&mut self, text: &str) {
        self.ctx.draw_text(self.cursor, text, TextStyle {
            font: self.ctx.theme().fonts.bold,
            size: 18.0,
            color: self.ctx.theme().colors.text,
            ..Default::default()
        });
        self.cursor[1] += 28.0;
    }

    /// Draw a button, returns true if clicked
    pub fn button(&mut self, label: &str) -> bool {
        let rect = Rect::new(self.cursor[0], self.cursor[1], 100.0, 24.0);

        let hovered = self.ctx.is_hovered(rect);
        let clicked = self.ctx.is_clicked(rect);

        let color = if hovered {
            self.ctx.theme().colors.primary
        } else {
            self.ctx.theme().colors.secondary
        };

        self.ctx.draw_rounded_rect(rect, color, 4.0);
        self.ctx
            .draw_text([rect.x + 8.0, rect.y + 4.0], label, TextStyle {
                color: Color::WHITE,
                ..Default::default()
            });

        self.next_line();
        clicked
    }

    /// Draw a checkbox, returns new value if changed
    pub fn checkbox(&mut self, label: &str, value: &mut bool) -> bool {
        let box_size = 18.0;
        let box_rect = Rect::new(self.cursor[0], self.cursor[1] + 1.0, box_size, box_size);

        let clicked = self.ctx.is_clicked(box_rect);
        if clicked {
            *value = !*value;
        }

        // Box background
        self.ctx.draw_rounded_rect(
            box_rect,
            if *value {
                self.ctx.theme().colors.primary
            } else {
                self.ctx.theme().colors.secondary
            },
            3.0,
        );

        // Checkmark
        if *value {
            // Would draw checkmark icon
        }

        // Label
        self.ctx.draw_text(
            [self.cursor[0] + box_size + 8.0, self.cursor[1]],
            label,
            TextStyle {
                color: self.ctx.theme().colors.text,
                ..Default::default()
            },
        );

        self.next_line();
        clicked
    }

    /// Draw a slider, returns new value
    pub fn slider(&mut self, label: &str, value: &mut f32, min: f32, max: f32) -> bool {
        let track_width = 150.0;
        let track_height = 6.0;
        let handle_size = 14.0;

        // Label
        self.ctx.draw_text(self.cursor, label, TextStyle {
            color: self.ctx.theme().colors.text,
            ..Default::default()
        });

        let track_x = self.cursor[0] + 100.0;
        let track_y = self.cursor[1] + (self.line_height - track_height) / 2.0;

        // Track
        let track_rect = Rect::new(track_x, track_y, track_width, track_height);
        self.ctx.draw_rounded_rect(
            track_rect,
            self.ctx.theme().colors.secondary,
            track_height / 2.0,
        );

        // Handle position
        let t = (*value - min) / (max - min);
        let handle_x = track_x + t * (track_width - handle_size);
        let handle_y = track_y + (track_height - handle_size) / 2.0;

        let handle_rect = Rect::new(handle_x, handle_y, handle_size, handle_size);

        // Handle interaction
        let dragging = self.ctx.is_pressed(track_rect.expand(10.0));
        let mut changed = false;

        if dragging {
            let mouse_x = self.ctx.input.mouse_pos[0];
            let new_t = ((mouse_x - track_x) / track_width).clamp(0.0, 1.0);
            let new_value = min + new_t * (max - min);
            if (*value - new_value).abs() > 0.001 {
                *value = new_value;
                changed = true;
            }
        }

        // Handle
        self.ctx.draw_rounded_rect(
            handle_rect,
            self.ctx.theme().colors.primary,
            handle_size / 2.0,
        );

        // Value text
        let value_text = alloc::format!("{:.2}", value);
        self.ctx.draw_text(
            [track_x + track_width + 10.0, self.cursor[1]],
            &value_text,
            TextStyle {
                color: self.ctx.theme().colors.text_secondary,
                ..Default::default()
            },
        );

        self.next_line();
        changed
    }

    /// Draw a progress bar
    pub fn progress(&mut self, label: &str, value: f32) {
        let bar_width = 200.0;
        let bar_height = 16.0;

        // Label
        self.ctx.draw_text(self.cursor, label, TextStyle {
            color: self.ctx.theme().colors.text,
            ..Default::default()
        });

        let bar_x = self.cursor[0] + 100.0;
        let bar_rect = Rect::new(bar_x, self.cursor[1], bar_width, bar_height);

        // Background
        self.ctx.draw_rounded_rect(
            bar_rect,
            self.ctx.theme().colors.secondary,
            bar_height / 2.0,
        );

        // Fill
        let fill_width = bar_width * value.clamp(0.0, 1.0);
        if fill_width > 0.0 {
            let fill_rect = Rect::new(bar_x, self.cursor[1], fill_width, bar_height);
            self.ctx.draw_rounded_rect(
                fill_rect,
                self.ctx.theme().colors.primary,
                bar_height / 2.0,
            );
        }

        // Percentage
        let pct_text = alloc::format!("{:.0}%", value * 100.0);
        self.ctx.draw_text(
            [bar_x + bar_width / 2.0 - 15.0, self.cursor[1]],
            &pct_text,
            TextStyle {
                color: Color::WHITE,
                size: 12.0,
                ..Default::default()
            },
        );

        self.next_line();
    }

    /// Draw a color picker (simplified)
    pub fn color(&mut self, label: &str, color: &mut Color) -> bool {
        let preview_size = 24.0;

        // Label
        self.ctx.draw_text(self.cursor, label, TextStyle {
            color: self.ctx.theme().colors.text,
            ..Default::default()
        });

        // Color preview
        let preview_rect = Rect::new(
            self.cursor[0] + 100.0,
            self.cursor[1] - 2.0,
            preview_size,
            preview_size,
        );
        self.ctx.draw_rounded_rect(preview_rect, *color, 4.0);

        // Hex value
        let r = (color.r * 255.0) as u32;
        let g = (color.g * 255.0) as u32;
        let b = (color.b * 255.0) as u32;
        let hex = alloc::format!("#{:02X}{:02X}{:02X}", r, g, b);
        self.ctx
            .draw_text([self.cursor[0] + 130.0, self.cursor[1]], &hex, TextStyle {
                font: self.ctx.theme().fonts.mono,
                color: self.ctx.theme().colors.text_secondary,
                ..Default::default()
            });

        self.next_line();
        false // Would open color picker on click
    }

    /// Draw a separator line
    pub fn separator(&mut self) {
        let line_y = self.cursor[1] + self.line_height / 2.0;
        self.ctx.draw(crate::RenderCommand::Line {
            start: [self.cursor[0], line_y],
            end: [self.cursor[0] + 280.0, line_y],
            color: self.ctx.theme().colors.border,
            thickness: 1.0,
        });
        self.next_line();
    }

    /// Begin a collapsible section
    pub fn collapsing(&mut self, label: &str, id: WidgetId) -> CollapsingSection<'_> {
        let header_rect = Rect::new(self.cursor[0], self.cursor[1], 280.0, 24.0);

        let clicked = self.ctx.is_clicked(header_rect);

        // Would track open state
        let open = true;

        // Header background
        self.ctx
            .draw_rounded_rect(header_rect, self.ctx.theme().colors.surface, 4.0);

        // Arrow
        let arrow = if open { "▼" } else { "▶" };
        self.ctx.draw_text(
            [self.cursor[0] + 8.0, self.cursor[1] + 4.0],
            arrow,
            TextStyle {
                size: 10.0,
                color: self.ctx.theme().colors.text_secondary,
                ..Default::default()
            },
        );

        // Label
        self.ctx.draw_text(
            [self.cursor[0] + 24.0, self.cursor[1] + 4.0],
            label,
            TextStyle {
                color: self.ctx.theme().colors.text,
                ..Default::default()
            },
        );

        self.next_line();

        CollapsingSection {
            imm: self,
            open,
            indent: 16.0,
        }
    }

    /// Draw a text input field
    pub fn text_input(&mut self, label: &str, value: &mut String, id: WidgetId) -> bool {
        let field_width = 180.0;
        let field_height = 24.0;

        // Label
        self.ctx.draw_text(self.cursor, label, TextStyle {
            color: self.ctx.theme().colors.text,
            ..Default::default()
        });

        let field_x = self.cursor[0] + 100.0;
        let field_rect = Rect::new(field_x, self.cursor[1] - 2.0, field_width, field_height);

        let focused = self.ctx.is_clicked(field_rect);

        // Background
        self.ctx.draw_rounded_rect(
            field_rect,
            if focused {
                self.ctx.theme().colors.surface
            } else {
                self.ctx.theme().colors.secondary
            },
            4.0,
        );

        // Border
        self.ctx.draw(crate::RenderCommand::RectOutline {
            rect: field_rect,
            color: if focused {
                self.ctx.theme().colors.primary
            } else {
                self.ctx.theme().colors.border
            },
            thickness: 1.0,
            corner_radius: 4.0,
        });

        // Text
        let display_text = if value.is_empty() { "..." } else { value };
        self.ctx
            .draw_text([field_x + 8.0, self.cursor[1]], display_text, TextStyle {
                color: if value.is_empty() {
                    self.ctx.theme().colors.text_secondary
                } else {
                    self.ctx.theme().colors.text
                },
                ..Default::default()
            });

        self.next_line();
        focused
    }

    /// Draw an integer input
    pub fn int_input(&mut self, label: &str, value: &mut i32) -> bool {
        let mut changed = false;
        let field_width = 80.0;
        let button_size = 20.0;

        // Label
        self.ctx.draw_text(self.cursor, label, TextStyle {
            color: self.ctx.theme().colors.text,
            ..Default::default()
        });

        let field_x = self.cursor[0] + 100.0;

        // Decrease button
        let dec_rect = Rect::new(field_x, self.cursor[1], button_size, button_size);
        if self.ctx.is_clicked(dec_rect) {
            *value -= 1;
            changed = true;
        }
        self.ctx
            .draw_rounded_rect(dec_rect, self.ctx.theme().colors.secondary, 3.0);
        self.ctx
            .draw_text([dec_rect.x + 6.0, dec_rect.y + 2.0], "-", TextStyle {
                color: self.ctx.theme().colors.text,
                ..Default::default()
            });

        // Value display
        let value_text = alloc::format!("{}", value);
        self.ctx.draw_text(
            [field_x + button_size + 10.0, self.cursor[1]],
            &value_text,
            TextStyle {
                font: self.ctx.theme().fonts.mono,
                color: self.ctx.theme().colors.text,
                ..Default::default()
            },
        );

        // Increase button
        let inc_rect = Rect::new(
            field_x + field_width - button_size,
            self.cursor[1],
            button_size,
            button_size,
        );
        if self.ctx.is_clicked(inc_rect) {
            *value += 1;
            changed = true;
        }
        self.ctx
            .draw_rounded_rect(inc_rect, self.ctx.theme().colors.secondary, 3.0);
        self.ctx
            .draw_text([inc_rect.x + 5.0, inc_rect.y + 2.0], "+", TextStyle {
                color: self.ctx.theme().colors.text,
                ..Default::default()
            });

        self.next_line();
        changed
    }
}

/// Collapsing section helper
pub struct CollapsingSection<'a> {
    imm: &'a mut Immediate<'a>,
    open: bool,
    indent: f32,
}

impl<'a> CollapsingSection<'a> {
    pub fn is_open(&self) -> bool {
        self.open
    }
}
