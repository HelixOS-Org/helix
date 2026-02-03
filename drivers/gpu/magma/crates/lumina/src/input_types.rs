//! Input Types for Lumina
//!
//! This module provides input handling types for interactive
//! graphics applications including mouse, keyboard, and gamepad.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Input Handles
// ============================================================================

/// Input manager handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct InputManagerHandle(pub u64);

impl InputManagerHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for InputManagerHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Gamepad handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GamepadHandle(pub u64);

impl GamepadHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for GamepadHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Mouse Input
// ============================================================================

/// Mouse state
#[derive(Clone, Copy, Debug, Default)]
pub struct MouseState {
    /// Position X
    pub x: f32,
    /// Position Y
    pub y: f32,
    /// Delta X
    pub delta_x: f32,
    /// Delta Y
    pub delta_y: f32,
    /// Scroll X
    pub scroll_x: f32,
    /// Scroll Y
    pub scroll_y: f32,
    /// Button state
    pub buttons: MouseButtons,
    /// Previous button state
    pub prev_buttons: MouseButtons,
}

impl MouseState {
    /// Creates state
    pub fn new() -> Self {
        Self::default()
    }

    /// Position
    pub fn position(&self) -> (f32, f32) {
        (self.x, self.y)
    }

    /// Delta
    pub fn delta(&self) -> (f32, f32) {
        (self.delta_x, self.delta_y)
    }

    /// Scroll
    pub fn scroll(&self) -> (f32, f32) {
        (self.scroll_x, self.scroll_y)
    }

    /// Is button pressed
    pub fn is_pressed(&self, button: MouseButton) -> bool {
        self.buttons.is_set(button)
    }

    /// Was button just pressed
    pub fn just_pressed(&self, button: MouseButton) -> bool {
        self.buttons.is_set(button) && !self.prev_buttons.is_set(button)
    }

    /// Was button just released
    pub fn just_released(&self, button: MouseButton) -> bool {
        !self.buttons.is_set(button) && self.prev_buttons.is_set(button)
    }

    /// Update position
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.delta_x = x - self.x;
        self.delta_y = y - self.y;
        self.x = x;
        self.y = y;
    }

    /// Update for new frame
    pub fn new_frame(&mut self) {
        self.delta_x = 0.0;
        self.delta_y = 0.0;
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
        self.prev_buttons = self.buttons;
    }
}

/// Mouse button
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum MouseButton {
    /// Left button
    Left = 0,
    /// Right button
    Right = 1,
    /// Middle button
    Middle = 2,
    /// Extra button 1
    Extra1 = 3,
    /// Extra button 2
    Extra2 = 4,
}

/// Mouse button flags
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct MouseButtons(pub u32);

impl MouseButtons {
    /// No buttons
    pub const NONE: Self = Self(0);
    /// Left button
    pub const LEFT: Self = Self(1 << 0);
    /// Right button
    pub const RIGHT: Self = Self(1 << 1);
    /// Middle button
    pub const MIDDLE: Self = Self(1 << 2);
    /// Extra 1
    pub const EXTRA1: Self = Self(1 << 3);
    /// Extra 2
    pub const EXTRA2: Self = Self(1 << 4);

    /// Is set
    pub const fn is_set(&self, button: MouseButton) -> bool {
        (self.0 & (1 << button as u32)) != 0
    }

    /// Set button
    pub fn set(&mut self, button: MouseButton) {
        self.0 |= 1 << button as u32;
    }

    /// Clear button
    pub fn clear(&mut self, button: MouseButton) {
        self.0 &= !(1 << button as u32);
    }

    /// Toggle button
    pub fn toggle(&mut self, button: MouseButton) {
        self.0 ^= 1 << button as u32;
    }
}

impl core::ops::BitOr for MouseButtons {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for MouseButtons {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

// ============================================================================
// Keyboard Input
// ============================================================================

/// Keyboard state
#[derive(Clone, Debug, Default)]
pub struct KeyboardState {
    /// Pressed keys
    pub keys: KeySet,
    /// Previous keys
    pub prev_keys: KeySet,
    /// Modifier state
    pub modifiers: KeyModifiers,
}

impl KeyboardState {
    /// Creates state
    pub fn new() -> Self {
        Self::default()
    }

    /// Is key pressed
    pub fn is_pressed(&self, key: KeyCode) -> bool {
        self.keys.is_set(key)
    }

    /// Was key just pressed
    pub fn just_pressed(&self, key: KeyCode) -> bool {
        self.keys.is_set(key) && !self.prev_keys.is_set(key)
    }

    /// Was key just released
    pub fn just_released(&self, key: KeyCode) -> bool {
        !self.keys.is_set(key) && self.prev_keys.is_set(key)
    }

    /// Update for new frame
    pub fn new_frame(&mut self) {
        self.prev_keys = self.keys.clone();
    }

    /// Press key
    pub fn press(&mut self, key: KeyCode) {
        self.keys.set(key);
        self.update_modifiers(key, true);
    }

    /// Release key
    pub fn release(&mut self, key: KeyCode) {
        self.keys.clear(key);
        self.update_modifiers(key, false);
    }

    fn update_modifiers(&mut self, key: KeyCode, pressed: bool) {
        match key {
            KeyCode::LeftShift | KeyCode::RightShift => {
                if pressed {
                    self.modifiers.0 |= KeyModifiers::SHIFT.0;
                } else {
                    self.modifiers.0 &= !KeyModifiers::SHIFT.0;
                }
            }
            KeyCode::LeftControl | KeyCode::RightControl => {
                if pressed {
                    self.modifiers.0 |= KeyModifiers::CTRL.0;
                } else {
                    self.modifiers.0 &= !KeyModifiers::CTRL.0;
                }
            }
            KeyCode::LeftAlt | KeyCode::RightAlt => {
                if pressed {
                    self.modifiers.0 |= KeyModifiers::ALT.0;
                } else {
                    self.modifiers.0 &= !KeyModifiers::ALT.0;
                }
            }
            _ => {}
        }
    }
}

/// Key set (256 bits)
#[derive(Clone, Debug, Default)]
pub struct KeySet {
    /// Bit array
    bits: [u64; 4],
}

impl KeySet {
    /// Creates empty set
    pub fn new() -> Self {
        Self::default()
    }

    /// Is key set
    pub fn is_set(&self, key: KeyCode) -> bool {
        let index = key as usize;
        let word = index / 64;
        let bit = index % 64;
        if word < 4 {
            (self.bits[word] & (1u64 << bit)) != 0
        } else {
            false
        }
    }

    /// Set key
    pub fn set(&mut self, key: KeyCode) {
        let index = key as usize;
        let word = index / 64;
        let bit = index % 64;
        if word < 4 {
            self.bits[word] |= 1u64 << bit;
        }
    }

    /// Clear key
    pub fn clear(&mut self, key: KeyCode) {
        let index = key as usize;
        let word = index / 64;
        let bit = index % 64;
        if word < 4 {
            self.bits[word] &= !(1u64 << bit);
        }
    }

    /// Clear all
    pub fn clear_all(&mut self) {
        self.bits = [0; 4];
    }
}

/// Key modifiers
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct KeyModifiers(pub u32);

impl KeyModifiers {
    /// None
    pub const NONE: Self = Self(0);
    /// Shift
    pub const SHIFT: Self = Self(1 << 0);
    /// Control
    pub const CTRL: Self = Self(1 << 1);
    /// Alt
    pub const ALT: Self = Self(1 << 2);
    /// Super/Windows/Command
    pub const SUPER: Self = Self(1 << 3);
    /// Caps Lock
    pub const CAPS_LOCK: Self = Self(1 << 4);
    /// Num Lock
    pub const NUM_LOCK: Self = Self(1 << 5);

    /// Has shift
    pub const fn shift(&self) -> bool {
        (self.0 & Self::SHIFT.0) != 0
    }

    /// Has control
    pub const fn ctrl(&self) -> bool {
        (self.0 & Self::CTRL.0) != 0
    }

    /// Has alt
    pub const fn alt(&self) -> bool {
        (self.0 & Self::ALT.0) != 0
    }
}

impl core::ops::BitOr for KeyModifiers {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Key code
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum KeyCode {
    // Letters
    A = 0,
    B = 1,
    C = 2,
    D = 3,
    E = 4,
    F = 5,
    G = 6,
    H = 7,
    I = 8,
    J = 9,
    K = 10,
    L = 11,
    M = 12,
    N = 13,
    O = 14,
    P = 15,
    Q = 16,
    R = 17,
    S = 18,
    T = 19,
    U = 20,
    V = 21,
    W = 22,
    X = 23,
    Y = 24,
    Z = 25,

    // Numbers
    Key0 = 26,
    Key1 = 27,
    Key2 = 28,
    Key3 = 29,
    Key4 = 30,
    Key5 = 31,
    Key6 = 32,
    Key7 = 33,
    Key8 = 34,
    Key9 = 35,

    // Function keys
    F1 = 36,
    F2 = 37,
    F3 = 38,
    F4 = 39,
    F5 = 40,
    F6 = 41,
    F7 = 42,
    F8 = 43,
    F9 = 44,
    F10 = 45,
    F11 = 46,
    F12 = 47,

    // Arrow keys
    Up = 48,
    Down = 49,
    Left = 50,
    Right = 51,

    // Modifiers
    LeftShift = 52,
    RightShift = 53,
    LeftControl = 54,
    RightControl = 55,
    LeftAlt = 56,
    RightAlt = 57,
    LeftSuper = 58,
    RightSuper = 59,

    // Special keys
    Escape = 60,
    Enter = 61,
    Tab = 62,
    Backspace = 63,
    Insert = 64,
    Delete = 65,
    Home = 66,
    End = 67,
    PageUp = 68,
    PageDown = 69,
    Space = 70,
    CapsLock = 71,
    NumLock = 72,
    ScrollLock = 73,
    PrintScreen = 74,
    Pause = 75,

    // Punctuation
    Minus = 76,
    Equal = 77,
    LeftBracket = 78,
    RightBracket = 79,
    Backslash = 80,
    Semicolon = 81,
    Apostrophe = 82,
    Grave = 83,
    Comma = 84,
    Period = 85,
    Slash = 86,

    // Numpad
    Numpad0 = 87,
    Numpad1 = 88,
    Numpad2 = 89,
    Numpad3 = 90,
    Numpad4 = 91,
    Numpad5 = 92,
    Numpad6 = 93,
    Numpad7 = 94,
    Numpad8 = 95,
    Numpad9 = 96,
    NumpadAdd = 97,
    NumpadSubtract = 98,
    NumpadMultiply = 99,
    NumpadDivide = 100,
    NumpadDecimal = 101,
    NumpadEnter = 102,
}

// ============================================================================
// Gamepad Input
// ============================================================================

/// Gamepad state
#[derive(Clone, Copy, Debug, Default)]
pub struct GamepadState {
    /// Connected
    pub connected: bool,
    /// Buttons
    pub buttons: GamepadButtons,
    /// Previous buttons
    pub prev_buttons: GamepadButtons,
    /// Left stick
    pub left_stick: [f32; 2],
    /// Right stick
    pub right_stick: [f32; 2],
    /// Left trigger
    pub left_trigger: f32,
    /// Right trigger
    pub right_trigger: f32,
}

impl GamepadState {
    /// Creates state
    pub fn new() -> Self {
        Self::default()
    }

    /// Is button pressed
    pub fn is_pressed(&self, button: GamepadButton) -> bool {
        self.buttons.is_set(button)
    }

    /// Just pressed
    pub fn just_pressed(&self, button: GamepadButton) -> bool {
        self.buttons.is_set(button) && !self.prev_buttons.is_set(button)
    }

    /// Just released
    pub fn just_released(&self, button: GamepadButton) -> bool {
        !self.buttons.is_set(button) && self.prev_buttons.is_set(button)
    }

    /// Update for new frame
    pub fn new_frame(&mut self) {
        self.prev_buttons = self.buttons;
    }

    /// Left stick with deadzone
    pub fn left_stick_deadzone(&self, deadzone: f32) -> [f32; 2] {
        Self::apply_deadzone(self.left_stick, deadzone)
    }

    /// Right stick with deadzone
    pub fn right_stick_deadzone(&self, deadzone: f32) -> [f32; 2] {
        Self::apply_deadzone(self.right_stick, deadzone)
    }

    fn apply_deadzone(stick: [f32; 2], deadzone: f32) -> [f32; 2] {
        let mag = (stick[0] * stick[0] + stick[1] * stick[1]).sqrt();
        if mag < deadzone {
            [0.0, 0.0]
        } else {
            let scale = (mag - deadzone) / (1.0 - deadzone) / mag;
            [stick[0] * scale, stick[1] * scale]
        }
    }
}

/// Gamepad button
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum GamepadButton {
    /// A / Cross
    South = 0,
    /// B / Circle
    East = 1,
    /// X / Square
    West = 2,
    /// Y / Triangle
    North = 3,
    /// Left bumper
    LeftBumper = 4,
    /// Right bumper
    RightBumper = 5,
    /// Left trigger (digital)
    LeftTrigger = 6,
    /// Right trigger (digital)
    RightTrigger = 7,
    /// Select / Back
    Select = 8,
    /// Start
    Start = 9,
    /// Left stick click
    LeftStick = 10,
    /// Right stick click
    RightStick = 11,
    /// D-pad up
    DPadUp = 12,
    /// D-pad down
    DPadDown = 13,
    /// D-pad left
    DPadLeft = 14,
    /// D-pad right
    DPadRight = 15,
    /// Guide / Home
    Guide = 16,
}

/// Gamepad button flags
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct GamepadButtons(pub u32);

impl GamepadButtons {
    /// No buttons
    pub const NONE: Self = Self(0);

    /// Is set
    pub const fn is_set(&self, button: GamepadButton) -> bool {
        (self.0 & (1 << button as u32)) != 0
    }

    /// Set button
    pub fn set(&mut self, button: GamepadButton) {
        self.0 |= 1 << button as u32;
    }

    /// Clear button
    pub fn clear(&mut self, button: GamepadButton) {
        self.0 &= !(1 << button as u32);
    }
}

// ============================================================================
// Touch Input
// ============================================================================

/// Touch state
#[derive(Clone, Debug, Default)]
pub struct TouchState {
    /// Active touches
    pub touches: Vec<Touch>,
    /// Previous touches
    pub prev_touches: Vec<Touch>,
}

impl TouchState {
    /// Creates state
    pub fn new() -> Self {
        Self::default()
    }

    /// Touch count
    pub fn touch_count(&self) -> usize {
        self.touches.len()
    }

    /// Get touch by index
    pub fn get(&self, index: usize) -> Option<&Touch> {
        self.touches.get(index)
    }

    /// Get touch by ID
    pub fn get_by_id(&self, id: u64) -> Option<&Touch> {
        self.touches.iter().find(|t| t.id == id)
    }

    /// Update for new frame
    pub fn new_frame(&mut self) {
        self.prev_touches = self.touches.clone();
    }
}

/// Touch point
#[derive(Clone, Copy, Debug)]
pub struct Touch {
    /// Touch ID
    pub id: u64,
    /// Position X
    pub x: f32,
    /// Position Y
    pub y: f32,
    /// Pressure (0-1)
    pub pressure: f32,
    /// Phase
    pub phase: TouchPhase,
}

impl Touch {
    /// Creates touch
    pub fn new(id: u64, x: f32, y: f32) -> Self {
        Self {
            id,
            x,
            y,
            pressure: 1.0,
            phase: TouchPhase::Began,
        }
    }

    /// Position
    pub fn position(&self) -> (f32, f32) {
        (self.x, self.y)
    }
}

/// Touch phase
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TouchPhase {
    /// Touch started
    #[default]
    Began = 0,
    /// Touch moved
    Moved = 1,
    /// Touch stationary
    Stationary = 2,
    /// Touch ended
    Ended = 3,
    /// Touch cancelled
    Cancelled = 4,
}

// ============================================================================
// Input Actions
// ============================================================================

/// Input action
#[derive(Clone, Debug)]
pub struct InputAction {
    /// Action name
    pub name: &'static str,
    /// Bindings
    pub bindings: Vec<InputBinding>,
}

impl InputAction {
    /// Creates action
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            bindings: Vec::new(),
        }
    }

    /// With key binding
    pub fn with_key(mut self, key: KeyCode) -> Self {
        self.bindings.push(InputBinding::Key(key));
        self
    }

    /// With mouse button
    pub fn with_mouse(mut self, button: MouseButton) -> Self {
        self.bindings.push(InputBinding::Mouse(button));
        self
    }

    /// With gamepad button
    pub fn with_gamepad(mut self, button: GamepadButton) -> Self {
        self.bindings.push(InputBinding::Gamepad(button));
        self
    }
}

/// Input binding
#[derive(Clone, Copy, Debug)]
pub enum InputBinding {
    /// Keyboard key
    Key(KeyCode),
    /// Mouse button
    Mouse(MouseButton),
    /// Gamepad button
    Gamepad(GamepadButton),
    /// Gamepad axis
    GamepadAxis(GamepadAxis, f32),
}

/// Gamepad axis
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum GamepadAxis {
    /// Left stick X
    LeftStickX = 0,
    /// Left stick Y
    LeftStickY = 1,
    /// Right stick X
    RightStickX = 2,
    /// Right stick Y
    RightStickY = 3,
    /// Left trigger
    LeftTrigger = 4,
    /// Right trigger
    RightTrigger = 5,
}

// ============================================================================
// Input GPU Data
// ============================================================================

/// Input GPU data (for shaders)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct InputGpuData {
    /// Mouse position (x, y, delta_x, delta_y)
    pub mouse: [f32; 4],
    /// Mouse buttons (bitfield)
    pub mouse_buttons: u32,
    /// Key modifiers
    pub key_modifiers: u32,
    /// Gamepad left stick
    pub gamepad_left_stick: [f32; 2],
    /// Gamepad right stick
    pub gamepad_right_stick: [f32; 2],
    /// Gamepad triggers
    pub gamepad_triggers: [f32; 2],
    /// Padding
    pub _padding: [f32; 2],
}

// ============================================================================
// Statistics
// ============================================================================

/// Input statistics
#[derive(Clone, Debug, Default)]
pub struct InputStats {
    /// Keys pressed
    pub keys_pressed: u32,
    /// Mouse buttons pressed
    pub mouse_buttons_pressed: u32,
    /// Gamepads connected
    pub gamepads_connected: u32,
    /// Touch count
    pub touch_count: u32,
}
