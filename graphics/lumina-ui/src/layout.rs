//! # Layout System
//!
//! CSS Flexbox-inspired layout system.

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::Rect;

/// Layout node
#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub style: LayoutStyle,
    pub computed: ComputedLayout,
    pub children: Vec<LayoutNode>,
}

impl LayoutNode {
    pub fn new(style: LayoutStyle) -> Self {
        Self {
            style,
            computed: ComputedLayout::default(),
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child: LayoutNode) {
        self.children.push(child);
    }

    /// Compute layout for this node and children
    pub fn compute(&mut self, available: Rect) {
        self.computed.rect = available;

        match self.style.display {
            Display::Flex => self.compute_flex(available),
            Display::Block => self.compute_block(available),
            Display::None => {},
        }
    }

    fn compute_flex(&mut self, available: Rect) {
        if self.children.is_empty() {
            return;
        }

        let padding = self.style.padding;
        let gap = self.style.gap;

        let content_x = available.x + padding.left;
        let content_y = available.y + padding.top;
        let content_width = available.width - padding.left - padding.right;
        let content_height = available.height - padding.top - padding.bottom;

        let is_row = self.style.flex_direction == FlexDirection::Row
            || self.style.flex_direction == FlexDirection::RowReverse;

        let main_size = if is_row {
            content_width
        } else {
            content_height
        };
        let cross_size = if is_row {
            content_height
        } else {
            content_width
        };

        // Calculate total flex grow and base sizes
        let mut total_flex_grow = 0.0f32;
        let mut total_base_size = 0.0f32;
        let num_children = self.children.len() as f32;
        let total_gap = gap * (num_children - 1.0).max(0.0);

        for child in &self.children {
            total_flex_grow += child.style.flex_grow;

            let base = if is_row {
                child.style.size.width.resolve(main_size)
            } else {
                child.style.size.height.resolve(main_size)
            };
            total_base_size += base.unwrap_or(0.0);
        }

        let free_space = (main_size - total_base_size - total_gap).max(0.0);

        // Position children
        let mut main_pos = match self.style.flex_direction {
            FlexDirection::Row => content_x,
            FlexDirection::RowReverse => content_x + content_width,
            FlexDirection::Column => content_y,
            FlexDirection::ColumnReverse => content_y + content_height,
        };

        for child in &mut self.children {
            let base = if is_row {
                child.style.size.width.resolve(main_size)
            } else {
                child.style.size.height.resolve(main_size)
            };

            let flex_size = if total_flex_grow > 0.0 {
                (child.style.flex_grow / total_flex_grow) * free_space
            } else {
                0.0
            };

            let child_main_size = base.unwrap_or(0.0) + flex_size;

            let child_cross_size = if is_row {
                child
                    .style
                    .size
                    .height
                    .resolve(cross_size)
                    .unwrap_or(cross_size)
            } else {
                child
                    .style
                    .size
                    .width
                    .resolve(cross_size)
                    .unwrap_or(cross_size)
            };

            // Calculate cross position based on align_items
            let cross_offset = match self.style.align_items {
                AlignItems::Start => 0.0,
                AlignItems::End => cross_size - child_cross_size,
                AlignItems::Center => (cross_size - child_cross_size) / 2.0,
                AlignItems::Stretch => 0.0,
            };

            let child_cross_size = if self.style.align_items == AlignItems::Stretch {
                cross_size
            } else {
                child_cross_size
            };

            let child_rect = if is_row {
                let x = match self.style.flex_direction {
                    FlexDirection::RowReverse => main_pos - child_main_size,
                    _ => main_pos,
                };
                Rect::new(
                    x,
                    content_y + cross_offset,
                    child_main_size,
                    child_cross_size,
                )
            } else {
                let y = match self.style.flex_direction {
                    FlexDirection::ColumnReverse => main_pos - child_main_size,
                    _ => main_pos,
                };
                Rect::new(
                    content_x + cross_offset,
                    y,
                    child_cross_size,
                    child_main_size,
                )
            };

            child.compute(child_rect);

            match self.style.flex_direction {
                FlexDirection::Row | FlexDirection::Column => {
                    main_pos += child_main_size + gap;
                },
                FlexDirection::RowReverse | FlexDirection::ColumnReverse => {
                    main_pos -= child_main_size + gap;
                },
            }
        }
    }

    fn compute_block(&mut self, available: Rect) {
        let padding = self.style.padding;
        let content_y = available.y + padding.top;
        let content_width = available.width - padding.left - padding.right;

        let mut y = content_y;

        for child in &mut self.children {
            let height = child
                .style
                .size
                .height
                .resolve(available.height)
                .unwrap_or(24.0);

            let child_rect = Rect::new(available.x + padding.left, y, content_width, height);

            child.compute(child_rect);
            y += height;
        }
    }
}

/// Layout style
#[derive(Debug, Clone)]
pub struct LayoutStyle {
    pub display: Display,
    pub position: Position,
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub align_content: AlignContent,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Dimension,
    pub size: Size,
    pub min_size: Size,
    pub max_size: Size,
    pub margin: Edges,
    pub padding: Edges,
    pub gap: f32,
    pub aspect_ratio: Option<f32>,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            display: Display::Flex,
            position: Position::Relative,
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Stretch,
            align_content: AlignContent::Stretch,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::Auto,
            size: Size::auto(),
            min_size: Size::auto(),
            max_size: Size::auto(),
            margin: Edges::zero(),
            padding: Edges::zero(),
            gap: 0.0,
            aspect_ratio: None,
        }
    }
}

/// Computed layout
#[derive(Debug, Clone, Default)]
pub struct ComputedLayout {
    pub rect: Rect,
}

/// Display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    Flex,
    Block,
    None,
}

/// Position mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Relative,
    Absolute,
}

/// Flex direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

/// Flex wrap
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexWrap {
    NoWrap,
    Wrap,
    WrapReverse,
}

/// Justify content
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyContent {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Align items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignItems {
    Start,
    End,
    Center,
    Stretch,
}

/// Align content
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignContent {
    Start,
    End,
    Center,
    Stretch,
    SpaceBetween,
    SpaceAround,
}

/// Dimension
#[derive(Debug, Clone, Copy)]
pub enum Dimension {
    Auto,
    Pixels(f32),
    Percent(f32),
}

impl Dimension {
    pub fn resolve(&self, parent: f32) -> Option<f32> {
        match self {
            Dimension::Auto => None,
            Dimension::Pixels(px) => Some(*px),
            Dimension::Percent(pct) => Some(parent * pct / 100.0),
        }
    }
}

/// Size
#[derive(Debug, Clone, Copy)]
pub struct Size {
    pub width: Dimension,
    pub height: Dimension,
}

impl Size {
    pub fn auto() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Auto,
        }
    }

    pub fn pixels(width: f32, height: f32) -> Self {
        Self {
            width: Dimension::Pixels(width),
            height: Dimension::Pixels(height),
        }
    }

    pub fn percent(width: f32, height: f32) -> Self {
        Self {
            width: Dimension::Percent(width),
            height: Dimension::Percent(height),
        }
    }
}

/// Edge values (margin/padding)
#[derive(Debug, Clone, Copy)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Edges {
    pub fn zero() -> Self {
        Self {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }

    pub fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }
}

/// Layout builder
pub struct LayoutBuilder {
    style: LayoutStyle,
    children: Vec<LayoutNode>,
}

impl LayoutBuilder {
    pub fn new() -> Self {
        Self {
            style: LayoutStyle::default(),
            children: Vec::new(),
        }
    }

    pub fn flex_row(mut self) -> Self {
        self.style.display = Display::Flex;
        self.style.flex_direction = FlexDirection::Row;
        self
    }

    pub fn flex_column(mut self) -> Self {
        self.style.display = Display::Flex;
        self.style.flex_direction = FlexDirection::Column;
        self
    }

    pub fn justify_center(mut self) -> Self {
        self.style.justify_content = JustifyContent::Center;
        self
    }

    pub fn justify_between(mut self) -> Self {
        self.style.justify_content = JustifyContent::SpaceBetween;
        self
    }

    pub fn align_center(mut self) -> Self {
        self.style.align_items = AlignItems::Center;
        self
    }

    pub fn gap(mut self, value: f32) -> Self {
        self.style.gap = value;
        self
    }

    pub fn padding(mut self, value: f32) -> Self {
        self.style.padding = Edges::all(value);
        self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.style.size = Size::pixels(width, height);
        self
    }

    pub fn flex_grow(mut self, value: f32) -> Self {
        self.style.flex_grow = value;
        self
    }

    pub fn child(mut self, node: LayoutNode) -> Self {
        self.children.push(node);
        self
    }

    pub fn build(self) -> LayoutNode {
        let mut node = LayoutNode::new(self.style);
        node.children = self.children;
        node
    }
}

impl Default for LayoutBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flex_row_layout() {
        let mut root = LayoutBuilder::new()
            .flex_row()
            .gap(10.0)
            .child(LayoutBuilder::new().size(100.0, 50.0).build())
            .child(LayoutBuilder::new().size(100.0, 50.0).build())
            .build();

        root.compute(Rect::new(0.0, 0.0, 500.0, 100.0));

        assert_eq!(root.children[0].computed.rect.x, 0.0);
        assert_eq!(root.children[1].computed.rect.x, 110.0);
    }
}
