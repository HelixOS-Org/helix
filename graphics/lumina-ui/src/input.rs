//! # Input Handling
//!
//! Mouse, keyboard, and touch input processing.

use alloc::vec::Vec;

use crate::Rect;

/// Input handler
pub struct InputHandler {
    state: InputState,
    prev_state: InputState,
    focus: Option<u64>,
    hot: Option<u64>,
    captured: Option<u64>,
    events: Vec<InputEvent>,
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            state: InputState::default(),
            prev_state: InputState::default(),
            focus: None,
            hot: None,
            captured: None,
            events: Vec::new(),
        }
    }

    /// Begin input processing for a frame
    pub fn begin_frame(&mut self, state: InputState) {
        self.prev_state = core::mem::replace(&mut self.state, state);
        self.events.clear();
        self.hot = None;
    }

    /// End input processing
    pub fn end_frame(&mut self) {
        // Reset click states
    }

    /// Get current input state
    pub fn state(&self) -> &InputState {
        &self.state
    }

    /// Check if mouse is over a rect
    pub fn is_hovered(&self, rect: Rect) -> bool {
        rect.contains(self.state.mouse_pos)
    }

    /// Check if mouse button was just pressed over rect
    pub fn is_pressed(&self, rect: Rect, button: MouseButton) -> bool {
        let idx = button as usize;
        self.is_hovered(rect) && self.state.mouse_down[idx] && !self.prev_state.mouse_down[idx]
    }

    /// Check if mouse button was just released over rect
    pub fn is_released(&self, rect: Rect, button: MouseButton) -> bool {
        let idx = button as usize;
        self.is_hovered(rect) && !self.state.mouse_down[idx] && self.prev_state.mouse_down[idx]
    }

    /// Check if a click occurred (press + release) over rect
    pub fn is_clicked(&self, rect: Rect, button: MouseButton) -> bool {
        self.is_released(rect, button)
    }

    /// Check if mouse is being held down over rect
    pub fn is_held(&self, rect: Rect, button: MouseButton) -> bool {
        let idx = button as usize;
        self.is_hovered(rect) && self.state.mouse_down[idx]
    }

    /// Set focus to a widget
    pub fn set_focus(&mut self, id: u64) {
        self.focus = Some(id);
    }

    /// Clear focus
    pub fn clear_focus(&mut self) {
        self.focus = None;
    }

    /// Check if widget has focus
    pub fn has_focus(&self, id: u64) -> bool {
        self.focus == Some(id)
    }

    /// Set hot widget (hovered)
    pub fn set_hot(&mut self, id: u64) {
        self.hot = Some(id);
    }

    /// Get hot widget
    pub fn hot(&self) -> Option<u64> {
        self.hot
    }

    /// Capture mouse input
    pub fn capture(&mut self, id: u64) {
        self.captured = Some(id);
    }

    /// Release mouse capture
    pub fn release_capture(&mut self) {
        self.captured = None;
    }

    /// Check if widget has capture
    pub fn has_capture(&self, id: u64) -> bool {
        self.captured == Some(id)
    }

    /// Get mouse delta
    pub fn mouse_delta(&self) -> [f32; 2] {
        [
            self.state.mouse_pos[0] - self.prev_state.mouse_pos[0],
            self.state.mouse_pos[1] - self.prev_state.mouse_pos[1],
        ]
    }

    /// Check if key was just pressed
    pub fn is_key_pressed(&self, key: Key) -> bool {
        let idx = key as usize;
        self.state.keys_down[idx] && !self.prev_state.keys_down[idx]
    }

    /// Check if key was just released
    pub fn is_key_released(&self, key: Key) -> bool {
        let idx = key as usize;
        !self.state.keys_down[idx] && self.prev_state.keys_down[idx]
    }

    /// Check if key is held
    pub fn is_key_held(&self, key: Key) -> bool {
        self.state.keys_down[key as usize]
    }

    /// Get scroll delta
    pub fn scroll_delta(&self) -> [f32; 2] {
        self.state.scroll_delta
    }

    /// Get text input this frame
    pub fn text_input(&self) -> &str {
        &self.state.text_input
    }

    /// Push an event
    pub fn push_event(&mut self, event: InputEvent) {
        self.events.push(event);
    }

    /// Get events
    pub fn events(&self) -> &[InputEvent] {
        &self.events
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Input state
#[derive(Debug, Clone, Default)]
pub struct InputState {
    pub mouse_pos: [f32; 2],
    pub mouse_down: [bool; 5],
    pub scroll_delta: [f32; 2],
    pub keys_down: [bool; 256],
    pub modifiers: Modifiers,
    pub text_input: alloc::string::String,
}

/// Keyboard modifiers
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub super_key: bool,
}

/// Mouse button
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left    = 0,
    Right   = 1,
    Middle  = 2,
    Button4 = 3,
    Button5 = 4,
}

/// Key code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Key {
    // Letters
    A         = 65,
    B         = 66,
    C         = 67,
    D         = 68,
    E         = 69,
    F         = 70,
    G         = 71,
    H         = 72,
    I         = 73,
    J         = 74,
    K         = 75,
    L         = 76,
    M         = 77,
    N         = 78,
    O         = 79,
    P         = 80,
    Q         = 81,
    R         = 82,
    S         = 83,
    T         = 84,
    U         = 85,
    V         = 86,
    W         = 87,
    X         = 88,
    Y         = 89,
    Z         = 90,

    // Numbers
    Num0      = 48,
    Num1      = 49,
    Num2      = 50,
    Num3      = 51,
    Num4      = 52,
    Num5      = 53,
    Num6      = 54,
    Num7      = 55,
    Num8      = 56,
    Num9      = 57,

    // Function keys
    F1        = 112,
    F2        = 113,
    F3        = 114,
    F4        = 115,
    F5        = 116,
    F6        = 117,
    F7        = 118,
    F8        = 119,
    F9        = 120,
    F10       = 121,
    F11       = 122,
    F12       = 123,

    // Control keys
    Escape    = 27,
    Tab       = 9,
    CapsLock  = 20,
    Shift     = 16,
    Ctrl      = 17,
    Alt       = 18,
    Space     = 32,
    Enter     = 13,
    Backspace = 8,
    Delete    = 46,
    Insert    = 45,
    Home      = 36,
    End       = 35,
    PageUp    = 33,
    PageDown  = 34,

    // Arrow keys
    Left      = 37,
    Up        = 38,
    Right     = 39,
    Down      = 40,
}

/// Input event
#[derive(Debug, Clone)]
pub enum InputEvent {
    MouseMove {
        pos: [f32; 2],
    },
    MouseDown {
        button: MouseButton,
        pos: [f32; 2],
    },
    MouseUp {
        button: MouseButton,
        pos: [f32; 2],
    },
    MouseWheel {
        delta: [f32; 2],
    },
    KeyDown {
        key: Key,
    },
    KeyUp {
        key: Key,
    },
    TextInput {
        text: alloc::string::String,
    },
    Touch {
        id: u32,
        phase: TouchPhase,
        pos: [f32; 2],
    },
}

/// Touch phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

/// Gesture recognizer
pub struct GestureRecognizer {
    touches: alloc::collections::BTreeMap<u32, TouchState>,
    gesture: Option<Gesture>,
}

impl GestureRecognizer {
    pub fn new() -> Self {
        Self {
            touches: alloc::collections::BTreeMap::new(),
            gesture: None,
        }
    }

    /// Process touch event
    pub fn process(&mut self, event: &InputEvent) -> Option<Gesture> {
        if let InputEvent::Touch { id, phase, pos } = event {
            match phase {
                TouchPhase::Started => {
                    self.touches.insert(*id, TouchState {
                        start_pos: *pos,
                        current_pos: *pos,
                        start_time: 0.0, // Would use actual time
                    });
                },
                TouchPhase::Moved => {
                    if let Some(touch) = self.touches.get_mut(id) {
                        touch.current_pos = *pos;
                    }
                },
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.touches.remove(id);
                },
            }

            self.detect_gesture()
        } else {
            None
        }
    }

    fn detect_gesture(&mut self) -> Option<Gesture> {
        let touch_count = self.touches.len();

        match touch_count {
            1 => {
                let touch = self.touches.values().next()?;
                let delta = [
                    touch.current_pos[0] - touch.start_pos[0],
                    touch.current_pos[1] - touch.start_pos[1],
                ];
                let distance = (delta[0] * delta[0] + delta[1] * delta[1]).sqrt();

                if distance > 10.0 {
                    Some(Gesture::Pan { delta })
                } else {
                    None
                }
            },
            2 => {
                let touches: Vec<_> = self.touches.values().collect();
                let start_dist = distance(touches[0].start_pos, touches[1].start_pos);
                let current_dist = distance(touches[0].current_pos, touches[1].current_pos);

                if (current_dist - start_dist).abs() > 10.0 {
                    Some(Gesture::Pinch {
                        scale: current_dist / start_dist,
                        center: midpoint(touches[0].current_pos, touches[1].current_pos),
                    })
                } else {
                    None
                }
            },
            _ => None,
        }
    }
}

impl Default for GestureRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Touch state
struct TouchState {
    start_pos: [f32; 2],
    current_pos: [f32; 2],
    start_time: f64,
}

/// Gesture type
#[derive(Debug, Clone)]
pub enum Gesture {
    Tap { pos: [f32; 2] },
    DoubleTap { pos: [f32; 2] },
    LongPress { pos: [f32; 2] },
    Pan { delta: [f32; 2] },
    Pinch { scale: f32, center: [f32; 2] },
    Rotate { angle: f32, center: [f32; 2] },
    Swipe { direction: SwipeDirection },
}

/// Swipe direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwipeDirection {
    Left,
    Right,
    Up,
    Down,
}

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    (dx * dx + dy * dy).sqrt()
}

fn midpoint(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]
}
