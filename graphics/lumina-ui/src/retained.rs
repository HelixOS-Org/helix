//! # Retained Mode UI
//!
//! High-performance reactive UI with widget tree.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::{Color, InputState, Rect, RenderCommand, TextStyle, WidgetId};

/// UI widget tree
pub struct WidgetTree {
    root: Option<WidgetId>,
    widgets: BTreeMap<WidgetId, WidgetNode>,
    next_id: u64,
    dirty: bool,
}

impl WidgetTree {
    pub fn new() -> Self {
        Self {
            root: None,
            widgets: BTreeMap::new(),
            next_id: 1,
            dirty: true,
        }
    }

    /// Set root widget
    pub fn set_root(&mut self, widget: impl Widget + 'static) {
        let id = self.add_widget(widget);
        self.root = Some(id);
    }

    /// Add a widget
    pub fn add_widget(&mut self, widget: impl Widget + 'static) -> WidgetId {
        let id = WidgetId::new(self.next_id);
        self.next_id += 1;

        self.widgets.insert(id, WidgetNode {
            widget: Box::new(widget),
            parent: None,
            children: Vec::new(),
            layout: Rect::default(),
            state: WidgetState::default(),
        });

        self.dirty = true;
        id
    }

    /// Add child to widget
    pub fn add_child(&mut self, parent: WidgetId, child: WidgetId) {
        if let Some(parent_node) = self.widgets.get_mut(&parent) {
            parent_node.children.push(child);
        }
        if let Some(child_node) = self.widgets.get_mut(&child) {
            child_node.parent = Some(parent);
        }
        self.dirty = true;
    }

    /// Remove a widget
    pub fn remove(&mut self, id: WidgetId) {
        if let Some(node) = self.widgets.remove(&id) {
            // Remove from parent
            if let Some(parent_id) = node.parent {
                if let Some(parent) = self.widgets.get_mut(&parent_id) {
                    parent.children.retain(|c| *c != id);
                }
            }
            // Remove children recursively
            for child in node.children {
                self.remove(child);
            }
        }
        self.dirty = true;
    }

    /// Update the tree
    pub fn update(&mut self, input: &InputState, dt: f32) {
        if self.dirty {
            self.layout();
            self.dirty = false;
        }

        // Update widgets
        if let Some(root) = self.root {
            self.update_widget(root, input, dt);
        }
    }

    fn update_widget(&mut self, id: WidgetId, input: &InputState, dt: f32) {
        // Get widget info
        let (children, layout) = {
            let node = match self.widgets.get(&id) {
                Some(n) => n,
                None => return,
            };
            (node.children.clone(), node.layout)
        };

        // Update state based on input
        let hovered = layout.contains(input.mouse_pos);
        let pressed = hovered && input.mouse_down[0];
        let clicked = hovered && input.mouse_clicked[0];

        if let Some(node) = self.widgets.get_mut(&id) {
            node.state.hovered = hovered;
            node.state.pressed = pressed;
            node.state.focused = clicked || node.state.focused;
        }

        // Update children
        for child in children {
            self.update_widget(child, input, dt);
        }
    }

    /// Layout the tree
    fn layout(&mut self) {
        if let Some(root) = self.root {
            let screen = Rect::new(0.0, 0.0, 1920.0, 1080.0); // Would get actual size
            self.layout_widget(root, screen);
        }
    }

    fn layout_widget(&mut self, id: WidgetId, available: Rect) {
        let children: Vec<WidgetId> = {
            let node = match self.widgets.get_mut(&id) {
                Some(n) => n,
                None => return,
            };
            node.layout = available;
            node.children.clone()
        };

        // Simple vertical layout for children
        let padding = 8.0;
        let spacing = 4.0;
        let mut y = available.y + padding;

        for child in children {
            let child_height = 24.0; // Would calculate
            let child_rect = Rect::new(
                available.x + padding,
                y,
                available.width - padding * 2.0,
                child_height,
            );
            self.layout_widget(child, child_rect);
            y += child_height + spacing;
        }
    }

    /// Render the tree
    pub fn render(&self, commands: &mut Vec<RenderCommand>) {
        if let Some(root) = self.root {
            self.render_widget(root, commands);
        }
    }

    fn render_widget(&self, id: WidgetId, commands: &mut Vec<RenderCommand>) {
        if let Some(node) = self.widgets.get(&id) {
            // Render widget
            let mut ctx = RenderContext {
                commands,
                layout: node.layout,
                state: &node.state,
            };
            node.widget.render(&mut ctx);

            // Render children
            for child in &node.children {
                self.render_widget(*child, commands);
            }
        }
    }
}

impl Default for WidgetTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Widget node in tree
struct WidgetNode {
    widget: Box<dyn Widget>,
    parent: Option<WidgetId>,
    children: Vec<WidgetId>,
    layout: Rect,
    state: WidgetState,
}

/// Widget state
#[derive(Debug, Clone, Default)]
pub struct WidgetState {
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
    pub disabled: bool,
    pub visible: bool,
}

/// Widget trait
pub trait Widget {
    /// Get preferred size
    fn preferred_size(&self) -> [f32; 2] {
        [0.0, 0.0] // Auto
    }

    /// Render the widget
    fn render(&self, ctx: &mut RenderContext);

    /// Handle event
    fn on_event(&mut self, _event: &WidgetEvent) -> bool {
        false
    }
}

/// Render context
pub struct RenderContext<'a> {
    pub commands: &'a mut Vec<RenderCommand>,
    pub layout: Rect,
    pub state: &'a WidgetState,
}

impl<'a> RenderContext<'a> {
    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        self.commands.push(RenderCommand::Rect {
            rect,
            color,
            corner_radius: 0.0,
        });
    }

    pub fn draw_rounded_rect(&mut self, rect: Rect, color: Color, radius: f32) {
        self.commands.push(RenderCommand::Rect {
            rect,
            color,
            corner_radius: radius,
        });
    }

    pub fn draw_text(&mut self, pos: [f32; 2], text: &str, style: TextStyle) {
        self.commands.push(RenderCommand::Text {
            position: pos,
            text: text.into(),
            style,
        });
    }
}

/// Widget event
#[derive(Debug, Clone)]
pub enum WidgetEvent {
    Click { button: u8, pos: [f32; 2] },
    DoubleClick { button: u8, pos: [f32; 2] },
    MouseEnter,
    MouseLeave,
    MouseMove { pos: [f32; 2], delta: [f32; 2] },
    KeyDown { key: u8 },
    KeyUp { key: u8 },
    TextInput { text: String },
    Focus,
    Blur,
    Scroll { delta: [f32; 2] },
}

/// Container widget
pub struct Container {
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    pub border_radius: f32,
    pub padding: f32,
}

impl Default for Container {
    fn default() -> Self {
        Self {
            background: None,
            border_color: None,
            border_radius: 0.0,
            padding: 0.0,
        }
    }
}

impl Widget for Container {
    fn render(&self, ctx: &mut RenderContext) {
        if let Some(bg) = self.background {
            ctx.draw_rounded_rect(ctx.layout, bg, self.border_radius);
        }

        if let Some(border) = self.border_color {
            ctx.commands.push(RenderCommand::RectOutline {
                rect: ctx.layout,
                color: border,
                thickness: 1.0,
                corner_radius: self.border_radius,
            });
        }
    }
}

/// Label widget
pub struct Label {
    pub text: String,
    pub style: TextStyle,
}

impl Label {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.into(),
            style: TextStyle::default(),
        }
    }

    pub fn with_style(mut self, style: TextStyle) -> Self {
        self.style = style;
        self
    }
}

impl Widget for Label {
    fn preferred_size(&self) -> [f32; 2] {
        // Would calculate based on text
        [self.text.len() as f32 * 8.0, 20.0]
    }

    fn render(&self, ctx: &mut RenderContext) {
        ctx.draw_text([ctx.layout.x, ctx.layout.y], &self.text, self.style.clone());
    }
}

/// Button widget
pub struct Button {
    pub label: String,
    pub on_click: Option<Box<dyn Fn()>>,
}

impl Button {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.into(),
            on_click: None,
        }
    }

    pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_click = Some(Box::new(callback));
        self
    }
}

impl Widget for Button {
    fn preferred_size(&self) -> [f32; 2] {
        [100.0, 28.0]
    }

    fn render(&self, ctx: &mut RenderContext) {
        let color = if ctx.state.pressed {
            Color::hex(0x005A9E)
        } else if ctx.state.hovered {
            Color::hex(0x0078D4)
        } else {
            Color::hex(0x0066B8)
        };

        ctx.draw_rounded_rect(ctx.layout, color, 4.0);

        let text_x = ctx.layout.x + (ctx.layout.width - self.label.len() as f32 * 7.0) / 2.0;
        let text_y = ctx.layout.y + (ctx.layout.height - 14.0) / 2.0;

        ctx.draw_text([text_x, text_y], &self.label, TextStyle {
            color: Color::WHITE,
            ..Default::default()
        });
    }

    fn on_event(&mut self, event: &WidgetEvent) -> bool {
        if let WidgetEvent::Click { .. } = event {
            if let Some(ref callback) = self.on_click {
                callback();
            }
            true
        } else {
            false
        }
    }
}

/// Image widget
pub struct Image {
    pub texture: crate::TextureId,
    pub tint: Color,
    pub preserve_aspect: bool,
}

impl Widget for Image {
    fn render(&self, ctx: &mut RenderContext) {
        ctx.commands.push(RenderCommand::Image {
            rect: ctx.layout,
            texture: self.texture,
            uv: [0.0, 0.0, 1.0, 1.0],
            tint: self.tint,
        });
    }
}

/// Scroll view widget
pub struct ScrollView {
    pub scroll_offset: [f32; 2],
    pub content_size: [f32; 2],
}

impl ScrollView {
    pub fn new() -> Self {
        Self {
            scroll_offset: [0.0, 0.0],
            content_size: [0.0, 0.0],
        }
    }
}

impl Default for ScrollView {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for ScrollView {
    fn render(&self, ctx: &mut RenderContext) {
        // Background
        ctx.draw_rect(ctx.layout, Color::hex(0x1E1E1E));

        // Would set clip rect for children
        ctx.commands.push(RenderCommand::Clip { rect: ctx.layout });
    }

    fn on_event(&mut self, event: &WidgetEvent) -> bool {
        if let WidgetEvent::Scroll { delta } = event {
            self.scroll_offset[0] += delta[0];
            self.scroll_offset[1] += delta[1];

            // Clamp
            self.scroll_offset[0] = self.scroll_offset[0].max(0.0);
            self.scroll_offset[1] = self.scroll_offset[1].max(0.0);

            true
        } else {
            false
        }
    }
}
