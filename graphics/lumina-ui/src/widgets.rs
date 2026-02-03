//! # UI Widgets
//!
//! Complete widget library.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use crate::retained::{RenderContext, Widget, WidgetEvent};
use crate::{Color, Rect, RenderCommand, TextStyle, UiContext, WidgetId};

/// Text input widget
pub struct TextInput {
    pub value: String,
    pub placeholder: String,
    pub password: bool,
    pub max_length: Option<usize>,
    cursor_pos: usize,
    selection: Option<(usize, usize)>,
}

impl TextInput {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            placeholder: String::new(),
            password: false,
            max_length: None,
            cursor_pos: 0,
            selection: None,
        }
    }

    pub fn with_placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn password(mut self) -> Self {
        self.password = true;
        self
    }
}

impl Default for TextInput {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for TextInput {
    fn preferred_size(&self) -> [f32; 2] {
        [200.0, 28.0]
    }

    fn render(&self, ctx: &mut RenderContext) {
        // Background
        let bg_color = if ctx.state.focused {
            Color::hex(0x3C3C3C)
        } else {
            Color::hex(0x2D2D2D)
        };
        ctx.draw_rounded_rect(ctx.layout, bg_color, 4.0);

        // Border
        let border_color = if ctx.state.focused {
            Color::hex(0x007ACC)
        } else {
            Color::hex(0x474747)
        };
        ctx.commands.push(RenderCommand::RectOutline {
            rect: ctx.layout,
            color: border_color,
            thickness: 1.0,
            corner_radius: 4.0,
        });

        // Text
        let display_text = if self.value.is_empty() {
            &self.placeholder
        } else if self.password {
            &"•".repeat(self.value.len())
        } else {
            &self.value
        };

        let text_color = if self.value.is_empty() {
            Color::hex(0x808080)
        } else {
            Color::hex(0xCCCCCC)
        };

        ctx.draw_text(
            [ctx.layout.x + 8.0, ctx.layout.y + 6.0],
            display_text,
            TextStyle {
                color: text_color,
                ..Default::default()
            },
        );

        // Cursor
        if ctx.state.focused {
            let cursor_x = ctx.layout.x + 8.0 + (self.cursor_pos as f32 * 7.0);
            ctx.commands.push(RenderCommand::Line {
                start: [cursor_x, ctx.layout.y + 4.0],
                end: [cursor_x, ctx.layout.y + ctx.layout.height - 4.0],
                color: Color::WHITE,
                thickness: 1.0,
            });
        }
    }

    fn on_event(&mut self, event: &WidgetEvent) -> bool {
        match event {
            WidgetEvent::TextInput { text } => {
                if let Some(max) = self.max_length {
                    if self.value.len() >= max {
                        return false;
                    }
                }
                self.value.insert_str(self.cursor_pos, text);
                self.cursor_pos += text.len();
                true
            },
            WidgetEvent::KeyDown { key } => {
                match *key {
                    8 => {
                        // Backspace
                        if self.cursor_pos > 0 {
                            self.cursor_pos -= 1;
                            self.value.remove(self.cursor_pos);
                        }
                        true
                    },
                    127 => {
                        // Delete
                        if self.cursor_pos < self.value.len() {
                            self.value.remove(self.cursor_pos);
                        }
                        true
                    },
                    37 => {
                        // Left
                        if self.cursor_pos > 0 {
                            self.cursor_pos -= 1;
                        }
                        true
                    },
                    39 => {
                        // Right
                        if self.cursor_pos < self.value.len() {
                            self.cursor_pos += 1;
                        }
                        true
                    },
                    _ => false,
                }
            },
            _ => false,
        }
    }
}

/// Dropdown/Select widget
pub struct Dropdown {
    pub options: Vec<String>,
    pub selected: Option<usize>,
    pub placeholder: String,
    open: bool,
}

impl Dropdown {
    pub fn new(options: Vec<String>) -> Self {
        Self {
            options,
            selected: None,
            placeholder: "Select...".into(),
            open: false,
        }
    }

    pub fn with_selected(mut self, index: usize) -> Self {
        self.selected = Some(index);
        self
    }
}

impl Widget for Dropdown {
    fn preferred_size(&self) -> [f32; 2] {
        [200.0, 28.0]
    }

    fn render(&self, ctx: &mut RenderContext) {
        // Main button
        ctx.draw_rounded_rect(ctx.layout, Color::hex(0x3C3C3C), 4.0);

        // Text
        let text = self
            .selected
            .and_then(|i| self.options.get(i))
            .map(|s| s.as_str())
            .unwrap_or(&self.placeholder);

        ctx.draw_text([ctx.layout.x + 8.0, ctx.layout.y + 6.0], text, TextStyle {
            color: if self.selected.is_some() {
                Color::hex(0xCCCCCC)
            } else {
                Color::hex(0x808080)
            },
            ..Default::default()
        });

        // Arrow
        ctx.draw_text(
            [ctx.layout.x + ctx.layout.width - 20.0, ctx.layout.y + 6.0],
            if self.open { "▲" } else { "▼" },
            TextStyle {
                size: 10.0,
                color: Color::hex(0x808080),
                ..Default::default()
            },
        );

        // Dropdown list
        if self.open {
            let list_rect = Rect::new(
                ctx.layout.x,
                ctx.layout.y + ctx.layout.height + 2.0,
                ctx.layout.width,
                (self.options.len() as f32 * 24.0).min(200.0),
            );

            ctx.draw_rounded_rect(list_rect, Color::hex(0x2D2D2D), 4.0);

            for (i, option) in self.options.iter().enumerate() {
                let item_y = list_rect.y + i as f32 * 24.0;
                let selected = self.selected == Some(i);

                if selected {
                    let item_rect = Rect::new(list_rect.x, item_y, list_rect.width, 24.0);
                    ctx.draw_rect(item_rect, Color::hex(0x094771));
                }

                ctx.draw_text([list_rect.x + 8.0, item_y + 4.0], option, TextStyle {
                    color: Color::hex(0xCCCCCC),
                    ..Default::default()
                });
            }
        }
    }

    fn on_event(&mut self, event: &WidgetEvent) -> bool {
        match event {
            WidgetEvent::Click { .. } => {
                self.open = !self.open;
                true
            },
            _ => false,
        }
    }
}

/// Toggle/Switch widget
pub struct Toggle {
    pub value: bool,
    pub label: String,
}

impl Toggle {
    pub fn new(value: bool) -> Self {
        Self {
            value,
            label: String::new(),
        }
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.into();
        self
    }
}

impl Widget for Toggle {
    fn preferred_size(&self) -> [f32; 2] {
        [48.0, 24.0]
    }

    fn render(&self, ctx: &mut RenderContext) {
        let track_width = 40.0;
        let track_height = 20.0;
        let knob_size = 16.0;

        let track_rect = Rect::new(
            ctx.layout.x,
            ctx.layout.y + (ctx.layout.height - track_height) / 2.0,
            track_width,
            track_height,
        );

        // Track
        let track_color = if self.value {
            Color::hex(0x007ACC)
        } else {
            Color::hex(0x3C3C3C)
        };
        ctx.draw_rounded_rect(track_rect, track_color, track_height / 2.0);

        // Knob
        let knob_x = if self.value {
            track_rect.x + track_width - knob_size - 2.0
        } else {
            track_rect.x + 2.0
        };
        let knob_rect = Rect::new(
            knob_x,
            track_rect.y + (track_height - knob_size) / 2.0,
            knob_size,
            knob_size,
        );
        ctx.draw_rounded_rect(knob_rect, Color::WHITE, knob_size / 2.0);

        // Label
        if !self.label.is_empty() {
            ctx.draw_text(
                [track_rect.x + track_width + 8.0, ctx.layout.y + 4.0],
                &self.label,
                TextStyle {
                    color: Color::hex(0xCCCCCC),
                    ..Default::default()
                },
            );
        }
    }

    fn on_event(&mut self, event: &WidgetEvent) -> bool {
        if let WidgetEvent::Click { .. } = event {
            self.value = !self.value;
            true
        } else {
            false
        }
    }
}

/// Slider widget
pub struct Slider {
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: Option<f32>,
}

impl Slider {
    pub fn new(value: f32, min: f32, max: f32) -> Self {
        Self {
            value,
            min,
            max,
            step: None,
        }
    }

    pub fn with_step(mut self, step: f32) -> Self {
        self.step = Some(step);
        self
    }

    fn normalized(&self) -> f32 {
        (self.value - self.min) / (self.max - self.min)
    }
}

impl Widget for Slider {
    fn preferred_size(&self) -> [f32; 2] {
        [200.0, 20.0]
    }

    fn render(&self, ctx: &mut RenderContext) {
        let track_height = 4.0;
        let knob_size = 16.0;

        let track_rect = Rect::new(
            ctx.layout.x + knob_size / 2.0,
            ctx.layout.y + (ctx.layout.height - track_height) / 2.0,
            ctx.layout.width - knob_size,
            track_height,
        );

        // Track background
        ctx.draw_rounded_rect(track_rect, Color::hex(0x3C3C3C), track_height / 2.0);

        // Track fill
        let fill_width = track_rect.width * self.normalized();
        if fill_width > 0.0 {
            let fill_rect = Rect::new(track_rect.x, track_rect.y, fill_width, track_height);
            ctx.draw_rounded_rect(fill_rect, Color::hex(0x007ACC), track_height / 2.0);
        }

        // Knob
        let knob_x = ctx.layout.x + self.normalized() * (ctx.layout.width - knob_size);
        let knob_rect = Rect::new(
            knob_x,
            ctx.layout.y + (ctx.layout.height - knob_size) / 2.0,
            knob_size,
            knob_size,
        );

        let knob_color = if ctx.state.pressed {
            Color::hex(0x005A9E)
        } else if ctx.state.hovered {
            Color::hex(0x0078D4)
        } else {
            Color::hex(0x007ACC)
        };

        ctx.draw_rounded_rect(knob_rect, knob_color, knob_size / 2.0);
    }

    fn on_event(&mut self, event: &WidgetEvent) -> bool {
        if let WidgetEvent::MouseMove { pos, .. } = event {
            // Would handle drag
            false
        } else {
            false
        }
    }
}

/// Progress bar widget
pub struct ProgressBar {
    pub value: f32,
    pub show_percentage: bool,
    pub indeterminate: bool,
}

impl ProgressBar {
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            show_percentage: true,
            indeterminate: false,
        }
    }

    pub fn indeterminate() -> Self {
        Self {
            value: 0.0,
            show_percentage: false,
            indeterminate: true,
        }
    }
}

impl Widget for ProgressBar {
    fn preferred_size(&self) -> [f32; 2] {
        [200.0, 8.0]
    }

    fn render(&self, ctx: &mut RenderContext) {
        // Background
        ctx.draw_rounded_rect(ctx.layout, Color::hex(0x3C3C3C), ctx.layout.height / 2.0);

        // Fill
        if !self.indeterminate {
            let fill_width = ctx.layout.width * self.value;
            if fill_width > 0.0 {
                let fill_rect =
                    Rect::new(ctx.layout.x, ctx.layout.y, fill_width, ctx.layout.height);
                ctx.draw_rounded_rect(fill_rect, Color::hex(0x007ACC), ctx.layout.height / 2.0);
            }
        }

        // Percentage text
        if self.show_percentage && ctx.layout.height >= 16.0 {
            let text = alloc::format!("{:.0}%", self.value * 100.0);
            ctx.draw_text(
                [ctx.layout.x + ctx.layout.width / 2.0 - 12.0, ctx.layout.y],
                &text,
                TextStyle {
                    size: 10.0,
                    color: Color::WHITE,
                    ..Default::default()
                },
            );
        }
    }
}

/// Tabs widget
pub struct Tabs {
    pub tabs: Vec<String>,
    pub selected: usize,
}

impl Tabs {
    pub fn new(tabs: Vec<String>) -> Self {
        Self { tabs, selected: 0 }
    }
}

impl Widget for Tabs {
    fn preferred_size(&self) -> [f32; 2] {
        [self.tabs.len() as f32 * 100.0, 32.0]
    }

    fn render(&self, ctx: &mut RenderContext) {
        let tab_width = ctx.layout.width / self.tabs.len() as f32;

        for (i, tab) in self.tabs.iter().enumerate() {
            let tab_rect = Rect::new(
                ctx.layout.x + i as f32 * tab_width,
                ctx.layout.y,
                tab_width,
                ctx.layout.height,
            );

            let selected = i == self.selected;

            // Background
            if selected {
                ctx.draw_rect(tab_rect, Color::hex(0x1E1E1E));
            }

            // Text
            ctx.draw_text([tab_rect.x + 12.0, tab_rect.y + 8.0], tab, TextStyle {
                color: if selected {
                    Color::WHITE
                } else {
                    Color::hex(0x808080)
                },
                ..Default::default()
            });

            // Active indicator
            if selected {
                let indicator_rect = Rect::new(
                    tab_rect.x,
                    tab_rect.y + tab_rect.height - 2.0,
                    tab_rect.width,
                    2.0,
                );
                ctx.draw_rect(indicator_rect, Color::hex(0x007ACC));
            }
        }
    }

    fn on_event(&mut self, event: &WidgetEvent) -> bool {
        if let WidgetEvent::Click { pos, .. } = event {
            // Would calculate which tab was clicked
            false
        } else {
            false
        }
    }
}

/// Tooltip widget
pub struct Tooltip {
    pub text: String,
    pub visible: bool,
    pub position: [f32; 2],
}

impl Tooltip {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.into(),
            visible: false,
            position: [0.0, 0.0],
        }
    }

    pub fn show(&mut self, x: f32, y: f32) {
        self.visible = true;
        self.position = [x, y];
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }
}

impl Widget for Tooltip {
    fn render(&self, ctx: &mut RenderContext) {
        if !self.visible {
            return;
        }

        let padding = 8.0;
        let text_width = self.text.len() as f32 * 7.0;
        let rect = Rect::new(
            self.position[0],
            self.position[1],
            text_width + padding * 2.0,
            20.0 + padding,
        );

        // Shadow
        let shadow_rect = Rect::new(rect.x + 2.0, rect.y + 2.0, rect.width, rect.height);
        ctx.draw_rounded_rect(shadow_rect, Color::rgba(0.0, 0.0, 0.0, 0.3), 4.0);

        // Background
        ctx.draw_rounded_rect(rect, Color::hex(0x3C3C3C), 4.0);

        // Text
        ctx.draw_text(
            [rect.x + padding, rect.y + padding / 2.0],
            &self.text,
            TextStyle {
                size: 12.0,
                color: Color::hex(0xCCCCCC),
                ..Default::default()
            },
        );
    }
}

/// Modal dialog
pub struct Modal {
    pub title: String,
    pub visible: bool,
    pub width: f32,
    pub height: f32,
}

impl Modal {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.into(),
            visible: false,
            width: 400.0,
            height: 300.0,
        }
    }

    pub fn open(&mut self) {
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
    }
}

impl Widget for Modal {
    fn render(&self, ctx: &mut RenderContext) {
        if !self.visible {
            return;
        }

        // Overlay
        ctx.draw_rect(ctx.layout, Color::rgba(0.0, 0.0, 0.0, 0.5));

        // Modal window
        let modal_rect = Rect::new(
            ctx.layout.x + (ctx.layout.width - self.width) / 2.0,
            ctx.layout.y + (ctx.layout.height - self.height) / 2.0,
            self.width,
            self.height,
        );

        ctx.draw_rounded_rect(modal_rect, Color::hex(0x252526), 8.0);

        // Title bar
        let title_rect = Rect::new(modal_rect.x, modal_rect.y, modal_rect.width, 32.0);
        ctx.draw_rounded_rect(
            Rect::new(title_rect.x, title_rect.y, title_rect.width, 8.0),
            Color::hex(0x3C3C3C),
            8.0,
        );
        ctx.draw_rect(
            Rect::new(title_rect.x, title_rect.y + 8.0, title_rect.width, 24.0),
            Color::hex(0x3C3C3C),
        );

        ctx.draw_text(
            [title_rect.x + 16.0, title_rect.y + 8.0],
            &self.title,
            TextStyle {
                color: Color::WHITE,
                ..Default::default()
            },
        );

        // Close button
        ctx.draw_text(
            [title_rect.x + title_rect.width - 24.0, title_rect.y + 8.0],
            "×",
            TextStyle {
                size: 16.0,
                color: Color::hex(0x808080),
                ..Default::default()
            },
        );
    }
}
