//! Input Device Support for Helix UEFI Bootloader
//!
//! This module provides comprehensive input device support including
//! keyboard, mouse, and touch input for the UEFI boot environment.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                       Input System Stack                                │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Application    │  Boot Menu  │  Text Input  │  Password Entry         │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Event Layer    │  Key Events  │  Mouse Events  │  Touch Events        │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Protocol       │  Simple Text  │  Pointer  │  Absolute Pointer        │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Hardware       │  PS/2  │  USB HID  │  I2C Touch                      │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - Keyboard input with scancode translation
//! - Mouse/pointer support
//! - Touch screen support
//! - Key combination handling
//! - Hotkey support
//! - International keyboard layouts

#![no_std]

use core::fmt;

// =============================================================================
// SCAN CODES
// =============================================================================

/// EFI scan codes (UEFI specification)
pub mod scancode {
    /// Null key
    pub const NULL: u16 = 0x00;
    /// Cursor Up
    pub const UP: u16 = 0x01;
    /// Cursor Down
    pub const DOWN: u16 = 0x02;
    /// Cursor Right
    pub const RIGHT: u16 = 0x03;
    /// Cursor Left
    pub const LEFT: u16 = 0x04;
    /// Home
    pub const HOME: u16 = 0x05;
    /// End
    pub const END: u16 = 0x06;
    /// Insert
    pub const INSERT: u16 = 0x07;
    /// Delete
    pub const DELETE: u16 = 0x08;
    /// Page Up
    pub const PAGE_UP: u16 = 0x09;
    /// Page Down
    pub const PAGE_DOWN: u16 = 0x0A;
    /// F1
    pub const F1: u16 = 0x0B;
    /// F2
    pub const F2: u16 = 0x0C;
    /// F3
    pub const F3: u16 = 0x0D;
    /// F4
    pub const F4: u16 = 0x0E;
    /// F5
    pub const F5: u16 = 0x0F;
    /// F6
    pub const F6: u16 = 0x10;
    /// F7
    pub const F7: u16 = 0x11;
    /// F8
    pub const F8: u16 = 0x12;
    /// F9
    pub const F9: u16 = 0x13;
    /// F10
    pub const F10: u16 = 0x14;
    /// F11
    pub const F11: u16 = 0x15;
    /// F12
    pub const F12: u16 = 0x16;
    /// Escape
    pub const ESCAPE: u16 = 0x17;
    /// Pause
    pub const PAUSE: u16 = 0x48;
    /// F13
    pub const F13: u16 = 0x68;
    /// F14
    pub const F14: u16 = 0x69;
    /// F15
    pub const F15: u16 = 0x6A;
    /// F16
    pub const F16: u16 = 0x6B;
    /// F17
    pub const F17: u16 = 0x6C;
    /// F18
    pub const F18: u16 = 0x6D;
    /// F19
    pub const F19: u16 = 0x6E;
    /// F20
    pub const F20: u16 = 0x6F;
    /// F21
    pub const F21: u16 = 0x70;
    /// F22
    pub const F22: u16 = 0x71;
    /// F23
    pub const F23: u16 = 0x72;
    /// F24
    pub const F24: u16 = 0x73;
    /// Mute
    pub const MUTE: u16 = 0x7F;
    /// Volume Up
    pub const VOLUME_UP: u16 = 0x80;
    /// Volume Down
    pub const VOLUME_DOWN: u16 = 0x81;
    /// Brightness Up
    pub const BRIGHTNESS_UP: u16 = 0x100;
    /// Brightness Down
    pub const BRIGHTNESS_DOWN: u16 = 0x101;
    /// Suspend
    pub const SUSPEND: u16 = 0x102;
    /// Hibernate
    pub const HIBERNATE: u16 = 0x103;
    /// Toggle Display
    pub const TOGGLE_DISPLAY: u16 = 0x104;
    /// Recovery
    pub const RECOVERY: u16 = 0x105;
    /// Eject
    pub const EJECT: u16 = 0x106;
}

/// PS/2 keyboard scan codes (Set 1)
pub mod ps2_scancode {
    /// Escape
    pub const ESC: u8 = 0x01;
    /// 1
    pub const KEY_1: u8 = 0x02;
    /// 2
    pub const KEY_2: u8 = 0x03;
    /// 3
    pub const KEY_3: u8 = 0x04;
    /// 4
    pub const KEY_4: u8 = 0x05;
    /// 5
    pub const KEY_5: u8 = 0x06;
    /// 6
    pub const KEY_6: u8 = 0x07;
    /// 7
    pub const KEY_7: u8 = 0x08;
    /// 8
    pub const KEY_8: u8 = 0x09;
    /// 9
    pub const KEY_9: u8 = 0x0A;
    /// 0
    pub const KEY_0: u8 = 0x0B;
    /// Minus
    pub const MINUS: u8 = 0x0C;
    /// Equals
    pub const EQUALS: u8 = 0x0D;
    /// Backspace
    pub const BACKSPACE: u8 = 0x0E;
    /// Tab
    pub const TAB: u8 = 0x0F;
    /// Q
    pub const Q: u8 = 0x10;
    /// W
    pub const W: u8 = 0x11;
    /// E
    pub const E: u8 = 0x12;
    /// R
    pub const R: u8 = 0x13;
    /// T
    pub const T: u8 = 0x14;
    /// Y
    pub const Y: u8 = 0x15;
    /// U
    pub const U: u8 = 0x16;
    /// I
    pub const I: u8 = 0x17;
    /// O
    pub const O: u8 = 0x18;
    /// P
    pub const P: u8 = 0x19;
    /// Left Bracket
    pub const LEFT_BRACKET: u8 = 0x1A;
    /// Right Bracket
    pub const RIGHT_BRACKET: u8 = 0x1B;
    /// Enter
    pub const ENTER: u8 = 0x1C;
    /// Left Control
    pub const LEFT_CTRL: u8 = 0x1D;
    /// A
    pub const A: u8 = 0x1E;
    /// S
    pub const S: u8 = 0x1F;
    /// D
    pub const D: u8 = 0x20;
    /// F
    pub const F: u8 = 0x21;
    /// G
    pub const G: u8 = 0x22;
    /// H
    pub const H: u8 = 0x23;
    /// J
    pub const J: u8 = 0x24;
    /// K
    pub const K: u8 = 0x25;
    /// L
    pub const L: u8 = 0x26;
    /// Semicolon
    pub const SEMICOLON: u8 = 0x27;
    /// Apostrophe
    pub const APOSTROPHE: u8 = 0x28;
    /// Grave/Backtick
    pub const GRAVE: u8 = 0x29;
    /// Left Shift
    pub const LEFT_SHIFT: u8 = 0x2A;
    /// Backslash
    pub const BACKSLASH: u8 = 0x2B;
    /// Z
    pub const Z: u8 = 0x2C;
    /// X
    pub const X: u8 = 0x2D;
    /// C
    pub const C: u8 = 0x2E;
    /// V
    pub const V: u8 = 0x2F;
    /// B
    pub const B: u8 = 0x30;
    /// N
    pub const N: u8 = 0x31;
    /// M
    pub const M: u8 = 0x32;
    /// Comma
    pub const COMMA: u8 = 0x33;
    /// Period
    pub const PERIOD: u8 = 0x34;
    /// Slash
    pub const SLASH: u8 = 0x35;
    /// Right Shift
    pub const RIGHT_SHIFT: u8 = 0x36;
    /// Keypad Asterisk
    pub const KP_ASTERISK: u8 = 0x37;
    /// Left Alt
    pub const LEFT_ALT: u8 = 0x38;
    /// Space
    pub const SPACE: u8 = 0x39;
    /// Caps Lock
    pub const CAPS_LOCK: u8 = 0x3A;
    /// F1
    pub const F1: u8 = 0x3B;
    /// F2
    pub const F2: u8 = 0x3C;
    /// F3
    pub const F3: u8 = 0x3D;
    /// F4
    pub const F4: u8 = 0x3E;
    /// F5
    pub const F5: u8 = 0x3F;
    /// F6
    pub const F6: u8 = 0x40;
    /// F7
    pub const F7: u8 = 0x41;
    /// F8
    pub const F8: u8 = 0x42;
    /// F9
    pub const F9: u8 = 0x43;
    /// F10
    pub const F10: u8 = 0x44;
    /// Num Lock
    pub const NUM_LOCK: u8 = 0x45;
    /// Scroll Lock
    pub const SCROLL_LOCK: u8 = 0x46;
    /// Keypad 7 / Home
    pub const KP_7: u8 = 0x47;
    /// Keypad 8 / Up
    pub const KP_8: u8 = 0x48;
    /// Keypad 9 / Page Up
    pub const KP_9: u8 = 0x49;
    /// Keypad Minus
    pub const KP_MINUS: u8 = 0x4A;
    /// Keypad 4 / Left
    pub const KP_4: u8 = 0x4B;
    /// Keypad 5
    pub const KP_5: u8 = 0x4C;
    /// Keypad 6 / Right
    pub const KP_6: u8 = 0x4D;
    /// Keypad Plus
    pub const KP_PLUS: u8 = 0x4E;
    /// Keypad 1 / End
    pub const KP_1: u8 = 0x4F;
    /// Keypad 2 / Down
    pub const KP_2: u8 = 0x50;
    /// Keypad 3 / Page Down
    pub const KP_3: u8 = 0x51;
    /// Keypad 0 / Insert
    pub const KP_0: u8 = 0x52;
    /// Keypad Period / Delete
    pub const KP_PERIOD: u8 = 0x53;
    /// F11
    pub const F11: u8 = 0x57;
    /// F12
    pub const F12: u8 = 0x58;
    /// Extended key prefix
    pub const EXTENDED: u8 = 0xE0;
    /// Release prefix
    pub const RELEASE: u8 = 0x80;
}

/// USB HID keyboard usage codes
pub mod usb_hid {
    /// No key pressed
    pub const NONE: u8 = 0x00;
    /// Error roll over
    pub const ERROR_ROLL_OVER: u8 = 0x01;
    /// POST Fail
    pub const POST_FAIL: u8 = 0x02;
    /// Error undefined
    pub const ERROR_UNDEFINED: u8 = 0x03;
    /// A
    pub const A: u8 = 0x04;
    /// B
    pub const B: u8 = 0x05;
    /// C
    pub const C: u8 = 0x06;
    /// D
    pub const D: u8 = 0x07;
    /// E
    pub const E: u8 = 0x08;
    /// F
    pub const F: u8 = 0x09;
    /// G
    pub const G: u8 = 0x0A;
    /// H
    pub const H: u8 = 0x0B;
    /// I
    pub const I: u8 = 0x0C;
    /// J
    pub const J: u8 = 0x0D;
    /// K
    pub const K: u8 = 0x0E;
    /// L
    pub const L: u8 = 0x0F;
    /// M
    pub const M: u8 = 0x10;
    /// N
    pub const N: u8 = 0x11;
    /// O
    pub const O: u8 = 0x12;
    /// P
    pub const P: u8 = 0x13;
    /// Q
    pub const Q: u8 = 0x14;
    /// R
    pub const R: u8 = 0x15;
    /// S
    pub const S: u8 = 0x16;
    /// T
    pub const T: u8 = 0x17;
    /// U
    pub const U: u8 = 0x18;
    /// V
    pub const V: u8 = 0x19;
    /// W
    pub const W: u8 = 0x1A;
    /// X
    pub const X: u8 = 0x1B;
    /// Y
    pub const Y: u8 = 0x1C;
    /// Z
    pub const Z: u8 = 0x1D;
    /// 1
    pub const KEY_1: u8 = 0x1E;
    /// 2
    pub const KEY_2: u8 = 0x1F;
    /// 3
    pub const KEY_3: u8 = 0x20;
    /// 4
    pub const KEY_4: u8 = 0x21;
    /// 5
    pub const KEY_5: u8 = 0x22;
    /// 6
    pub const KEY_6: u8 = 0x23;
    /// 7
    pub const KEY_7: u8 = 0x24;
    /// 8
    pub const KEY_8: u8 = 0x25;
    /// 9
    pub const KEY_9: u8 = 0x26;
    /// 0
    pub const KEY_0: u8 = 0x27;
    /// Enter
    pub const ENTER: u8 = 0x28;
    /// Escape
    pub const ESCAPE: u8 = 0x29;
    /// Backspace
    pub const BACKSPACE: u8 = 0x2A;
    /// Tab
    pub const TAB: u8 = 0x2B;
    /// Space
    pub const SPACE: u8 = 0x2C;
    /// Minus
    pub const MINUS: u8 = 0x2D;
    /// Equals
    pub const EQUALS: u8 = 0x2E;
    /// Left Bracket
    pub const LEFT_BRACKET: u8 = 0x2F;
    /// Right Bracket
    pub const RIGHT_BRACKET: u8 = 0x30;
    /// Backslash
    pub const BACKSLASH: u8 = 0x31;
    /// Non-US Hash
    pub const NON_US_HASH: u8 = 0x32;
    /// Semicolon
    pub const SEMICOLON: u8 = 0x33;
    /// Apostrophe
    pub const APOSTROPHE: u8 = 0x34;
    /// Grave
    pub const GRAVE: u8 = 0x35;
    /// Comma
    pub const COMMA: u8 = 0x36;
    /// Period
    pub const PERIOD: u8 = 0x37;
    /// Slash
    pub const SLASH: u8 = 0x38;
    /// Caps Lock
    pub const CAPS_LOCK: u8 = 0x39;
    /// F1
    pub const F1: u8 = 0x3A;
    /// F2
    pub const F2: u8 = 0x3B;
    /// F3
    pub const F3: u8 = 0x3C;
    /// F4
    pub const F4: u8 = 0x3D;
    /// F5
    pub const F5: u8 = 0x3E;
    /// F6
    pub const F6: u8 = 0x3F;
    /// F7
    pub const F7: u8 = 0x40;
    /// F8
    pub const F8: u8 = 0x41;
    /// F9
    pub const F9: u8 = 0x42;
    /// F10
    pub const F10: u8 = 0x43;
    /// F11
    pub const F11: u8 = 0x44;
    /// F12
    pub const F12: u8 = 0x45;
    /// Print Screen
    pub const PRINT_SCREEN: u8 = 0x46;
    /// Scroll Lock
    pub const SCROLL_LOCK: u8 = 0x47;
    /// Pause
    pub const PAUSE: u8 = 0x48;
    /// Insert
    pub const INSERT: u8 = 0x49;
    /// Home
    pub const HOME: u8 = 0x4A;
    /// Page Up
    pub const PAGE_UP: u8 = 0x4B;
    /// Delete
    pub const DELETE: u8 = 0x4C;
    /// End
    pub const END: u8 = 0x4D;
    /// Page Down
    pub const PAGE_DOWN: u8 = 0x4E;
    /// Right Arrow
    pub const RIGHT: u8 = 0x4F;
    /// Left Arrow
    pub const LEFT: u8 = 0x50;
    /// Down Arrow
    pub const DOWN: u8 = 0x51;
    /// Up Arrow
    pub const UP: u8 = 0x52;
    /// Num Lock
    pub const NUM_LOCK: u8 = 0x53;
    /// Keypad /
    pub const KP_DIVIDE: u8 = 0x54;
    /// Keypad *
    pub const KP_MULTIPLY: u8 = 0x55;
    /// Keypad -
    pub const KP_MINUS: u8 = 0x56;
    /// Keypad +
    pub const KP_PLUS: u8 = 0x57;
    /// Keypad Enter
    pub const KP_ENTER: u8 = 0x58;
    /// Keypad 1
    pub const KP_1: u8 = 0x59;
    /// Keypad 2
    pub const KP_2: u8 = 0x5A;
    /// Keypad 3
    pub const KP_3: u8 = 0x5B;
    /// Keypad 4
    pub const KP_4: u8 = 0x5C;
    /// Keypad 5
    pub const KP_5: u8 = 0x5D;
    /// Keypad 6
    pub const KP_6: u8 = 0x5E;
    /// Keypad 7
    pub const KP_7: u8 = 0x5F;
    /// Keypad 8
    pub const KP_8: u8 = 0x60;
    /// Keypad 9
    pub const KP_9: u8 = 0x61;
    /// Keypad 0
    pub const KP_0: u8 = 0x62;
    /// Keypad .
    pub const KP_PERIOD: u8 = 0x63;
    /// Non-US Backslash
    pub const NON_US_BACKSLASH: u8 = 0x64;
    /// Application
    pub const APPLICATION: u8 = 0x65;
    /// Power
    pub const POWER: u8 = 0x66;
    /// Keypad =
    pub const KP_EQUALS: u8 = 0x67;
    /// F13
    pub const F13: u8 = 0x68;
    /// F14
    pub const F14: u8 = 0x69;
    /// F15
    pub const F15: u8 = 0x6A;
    /// F16
    pub const F16: u8 = 0x6B;
    /// F17
    pub const F17: u8 = 0x6C;
    /// F18
    pub const F18: u8 = 0x6D;
    /// F19
    pub const F19: u8 = 0x6E;
    /// F20
    pub const F20: u8 = 0x6F;
    /// F21
    pub const F21: u8 = 0x70;
    /// F22
    pub const F22: u8 = 0x71;
    /// F23
    pub const F23: u8 = 0x72;
    /// F24
    pub const F24: u8 = 0x73;

    // Modifier keys
    /// Left Control
    pub const LEFT_CTRL: u8 = 0xE0;
    /// Left Shift
    pub const LEFT_SHIFT: u8 = 0xE1;
    /// Left Alt
    pub const LEFT_ALT: u8 = 0xE2;
    /// Left GUI (Windows key)
    pub const LEFT_GUI: u8 = 0xE3;
    /// Right Control
    pub const RIGHT_CTRL: u8 = 0xE4;
    /// Right Shift
    pub const RIGHT_SHIFT: u8 = 0xE5;
    /// Right Alt
    pub const RIGHT_ALT: u8 = 0xE6;
    /// Right GUI
    pub const RIGHT_GUI: u8 = 0xE7;
}

// =============================================================================
// KEY INPUT STRUCTURES
// =============================================================================

/// Key modifiers
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyModifiers {
    /// Shift key pressed
    pub shift: bool,
    /// Control key pressed
    pub control: bool,
    /// Alt key pressed
    pub alt: bool,
    /// Logo/Windows key pressed
    pub logo: bool,
    /// Caps Lock active
    pub caps_lock: bool,
    /// Num Lock active
    pub num_lock: bool,
    /// Scroll Lock active
    pub scroll_lock: bool,
}

impl KeyModifiers {
    /// Create new empty modifiers
    pub const fn new() -> Self {
        Self {
            shift: false,
            control: false,
            alt: false,
            logo: false,
            caps_lock: false,
            num_lock: false,
            scroll_lock: false,
        }
    }

    /// Check if any modifier is pressed
    pub const fn any(&self) -> bool {
        self.shift || self.control || self.alt || self.logo
    }

    /// Check if Ctrl+Alt combination
    pub const fn ctrl_alt(&self) -> bool {
        self.control && self.alt
    }

    /// Check if Ctrl+Shift combination
    pub const fn ctrl_shift(&self) -> bool {
        self.control && self.shift
    }
}

/// Key event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventType {
    /// Key pressed
    Press,
    /// Key released
    Release,
    /// Key repeated (auto-repeat)
    Repeat,
}

/// Key input event
#[derive(Debug, Clone, Copy)]
pub struct KeyEvent {
    /// EFI scan code
    pub scancode: u16,
    /// Unicode character (0 if not printable)
    pub unicode: u16,
    /// Key modifiers
    pub modifiers: KeyModifiers,
    /// Event type
    pub event_type: KeyEventType,
    /// Timestamp (if available)
    pub timestamp: u64,
}

impl KeyEvent {
    /// Create new key event
    pub const fn new(scancode: u16, unicode: u16) -> Self {
        Self {
            scancode,
            unicode,
            modifiers: KeyModifiers::new(),
            event_type: KeyEventType::Press,
            timestamp: 0,
        }
    }

    /// Create press event
    pub const fn press(scancode: u16, unicode: u16, modifiers: KeyModifiers) -> Self {
        Self {
            scancode,
            unicode,
            modifiers,
            event_type: KeyEventType::Press,
            timestamp: 0,
        }
    }

    /// Check if this is a printable character
    pub const fn is_printable(&self) -> bool {
        self.unicode >= 0x20 && self.unicode < 0x7F
    }

    /// Check if this is a function key
    pub const fn is_function_key(&self) -> bool {
        self.scancode >= scancode::F1 && self.scancode <= scancode::F24
    }

    /// Check if this is a navigation key
    pub const fn is_navigation_key(&self) -> bool {
        matches!(
            self.scancode,
            scancode::UP
                | scancode::DOWN
                | scancode::LEFT
                | scancode::RIGHT
                | scancode::HOME
                | scancode::END
                | scancode::PAGE_UP
                | scancode::PAGE_DOWN
        )
    }

    /// Check if Enter key
    pub const fn is_enter(&self) -> bool {
        self.unicode == 0x0D || self.unicode == 0x0A
    }

    /// Check if Escape key
    pub const fn is_escape(&self) -> bool {
        self.scancode == scancode::ESCAPE
    }

    /// Check if Backspace key
    pub const fn is_backspace(&self) -> bool {
        self.unicode == 0x08
    }

    /// Check if Tab key
    pub const fn is_tab(&self) -> bool {
        self.unicode == 0x09
    }

    /// Get character as ASCII
    pub const fn as_char(&self) -> Option<char> {
        if self.unicode > 0 && self.unicode < 0x80 {
            Some(self.unicode as u8 as char)
        } else {
            None
        }
    }
}

/// Special key codes for boot menu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootMenuKey {
    /// Select current item
    Select,
    /// Move up
    Up,
    /// Move down
    Down,
    /// Exit menu
    Exit,
    /// Refresh display
    Refresh,
    /// Enter setup
    Setup,
    /// Boot from device
    BootDevice,
    /// Network boot
    NetworkBoot,
    /// Recovery mode
    Recovery,
    /// Firmware update
    FirmwareUpdate,
    /// Unknown key
    Unknown,
}

impl From<&KeyEvent> for BootMenuKey {
    fn from(key: &KeyEvent) -> Self {
        match key.scancode {
            scancode::UP => BootMenuKey::Up,
            scancode::DOWN => BootMenuKey::Down,
            scancode::ESCAPE => BootMenuKey::Exit,
            scancode::F1 => BootMenuKey::Setup,
            scancode::F2 => BootMenuKey::BootDevice,
            scancode::F10 => BootMenuKey::Setup,
            scancode::F11 => BootMenuKey::BootDevice,
            scancode::F12 => BootMenuKey::NetworkBoot,
            scancode::RECOVERY => BootMenuKey::Recovery,
            _ if key.is_enter() => BootMenuKey::Select,
            _ => BootMenuKey::Unknown,
        }
    }
}

// =============================================================================
// MOUSE/POINTER INPUT
// =============================================================================

/// Mouse button state
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MouseButtons {
    /// Left button pressed
    pub left: bool,
    /// Right button pressed
    pub right: bool,
    /// Middle button pressed
    pub middle: bool,
    /// Button 4 (back)
    pub button4: bool,
    /// Button 5 (forward)
    pub button5: bool,
}

impl MouseButtons {
    /// Create new empty state
    pub const fn new() -> Self {
        Self {
            left: false,
            right: false,
            middle: false,
            button4: false,
            button5: false,
        }
    }

    /// Check if any button is pressed
    pub const fn any(&self) -> bool {
        self.left || self.right || self.middle || self.button4 || self.button5
    }

    /// Create from button mask
    pub const fn from_mask(mask: u8) -> Self {
        Self {
            left: (mask & 0x01) != 0,
            right: (mask & 0x02) != 0,
            middle: (mask & 0x04) != 0,
            button4: (mask & 0x08) != 0,
            button5: (mask & 0x10) != 0,
        }
    }

    /// Convert to button mask
    pub const fn to_mask(&self) -> u8 {
        let mut mask = 0u8;
        if self.left {
            mask |= 0x01;
        }
        if self.right {
            mask |= 0x02;
        }
        if self.middle {
            mask |= 0x04;
        }
        if self.button4 {
            mask |= 0x08;
        }
        if self.button5 {
            mask |= 0x10;
        }
        mask
    }
}

/// Mouse event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventType {
    /// Mouse moved
    Move,
    /// Button pressed
    ButtonDown,
    /// Button released
    ButtonUp,
    /// Mouse wheel scrolled
    Scroll,
}

/// Mouse/pointer event
#[derive(Debug, Clone, Copy)]
pub struct MouseEvent {
    /// X position (relative or absolute)
    pub x: i32,
    /// Y position (relative or absolute)
    pub y: i32,
    /// Scroll wheel delta (vertical)
    pub scroll_y: i16,
    /// Scroll wheel delta (horizontal)
    pub scroll_x: i16,
    /// Button state
    pub buttons: MouseButtons,
    /// Event type
    pub event_type: MouseEventType,
    /// True if coordinates are absolute
    pub absolute: bool,
    /// Timestamp
    pub timestamp: u64,
}

impl MouseEvent {
    /// Create new relative move event
    pub const fn relative_move(dx: i32, dy: i32, buttons: MouseButtons) -> Self {
        Self {
            x: dx,
            y: dy,
            scroll_y: 0,
            scroll_x: 0,
            buttons,
            event_type: MouseEventType::Move,
            absolute: false,
            timestamp: 0,
        }
    }

    /// Create new absolute position event
    pub const fn absolute_move(x: i32, y: i32, buttons: MouseButtons) -> Self {
        Self {
            x,
            y,
            scroll_y: 0,
            scroll_x: 0,
            buttons,
            event_type: MouseEventType::Move,
            absolute: true,
            timestamp: 0,
        }
    }

    /// Create scroll event
    pub const fn scroll(scroll_y: i16, scroll_x: i16) -> Self {
        Self {
            x: 0,
            y: 0,
            scroll_y,
            scroll_x,
            buttons: MouseButtons::new(),
            event_type: MouseEventType::Scroll,
            absolute: false,
            timestamp: 0,
        }
    }
}

// =============================================================================
// TOUCH INPUT
// =============================================================================

/// Touch point state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchState {
    /// Touch started (finger down)
    Down,
    /// Touch moved
    Move,
    /// Touch ended (finger up)
    Up,
    /// Touch cancelled
    Cancel,
}

/// Single touch point
#[derive(Debug, Clone, Copy)]
pub struct TouchPoint {
    /// Touch point ID
    pub id: u32,
    /// X coordinate
    pub x: i32,
    /// Y coordinate
    pub y: i32,
    /// Touch pressure (0-255)
    pub pressure: u8,
    /// Touch width
    pub width: u16,
    /// Touch height
    pub height: u16,
    /// Touch state
    pub state: TouchState,
}

impl TouchPoint {
    /// Create new touch point
    pub const fn new(id: u32, x: i32, y: i32, state: TouchState) -> Self {
        Self {
            id,
            x,
            y,
            pressure: 255,
            width: 1,
            height: 1,
            state,
        }
    }
}

/// Touch event (multi-touch capable)
#[derive(Debug, Clone)]
pub struct TouchEvent {
    /// Touch points
    pub points: [Option<TouchPoint>; 10],
    /// Number of active points
    pub point_count: usize,
    /// Timestamp
    pub timestamp: u64,
}

impl TouchEvent {
    /// Create new empty touch event
    pub const fn new() -> Self {
        Self {
            points: [None; 10],
            point_count: 0,
            timestamp: 0,
        }
    }

    /// Add touch point
    pub fn add_point(&mut self, point: TouchPoint) {
        if self.point_count < 10 {
            self.points[self.point_count] = Some(point);
            self.point_count += 1;
        }
    }

    /// Get primary touch point
    pub fn primary(&self) -> Option<&TouchPoint> {
        self.points[0].as_ref()
    }

    /// Check if this is a tap gesture (single quick touch)
    pub fn is_tap(&self) -> bool {
        self.point_count == 1
            && self.points[0]
                .map(|p| p.state == TouchState::Up)
                .unwrap_or(false)
    }

    /// Check if this is a two-finger gesture
    pub const fn is_two_finger(&self) -> bool {
        self.point_count == 2
    }
}

impl Default for TouchEvent {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// KEYBOARD LAYOUTS
// =============================================================================

/// Keyboard layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardLayout {
    /// US English (QWERTY)
    UsEnglish,
    /// UK English
    UkEnglish,
    /// German (QWERTZ)
    German,
    /// French (AZERTY)
    French,
    /// Spanish
    Spanish,
    /// Italian
    Italian,
    /// Portuguese
    Portuguese,
    /// Russian
    Russian,
    /// Japanese
    Japanese,
    /// Korean
    Korean,
    /// Chinese Simplified
    ChineseSimplified,
}

impl KeyboardLayout {
    /// Get layout name
    pub const fn name(&self) -> &'static str {
        match self {
            KeyboardLayout::UsEnglish => "US English",
            KeyboardLayout::UkEnglish => "UK English",
            KeyboardLayout::German => "German",
            KeyboardLayout::French => "French",
            KeyboardLayout::Spanish => "Spanish",
            KeyboardLayout::Italian => "Italian",
            KeyboardLayout::Portuguese => "Portuguese",
            KeyboardLayout::Russian => "Russian",
            KeyboardLayout::Japanese => "Japanese",
            KeyboardLayout::Korean => "Korean",
            KeyboardLayout::ChineseSimplified => "Chinese Simplified",
        }
    }

    /// Get locale code
    pub const fn locale(&self) -> &'static str {
        match self {
            KeyboardLayout::UsEnglish => "en-US",
            KeyboardLayout::UkEnglish => "en-GB",
            KeyboardLayout::German => "de-DE",
            KeyboardLayout::French => "fr-FR",
            KeyboardLayout::Spanish => "es-ES",
            KeyboardLayout::Italian => "it-IT",
            KeyboardLayout::Portuguese => "pt-PT",
            KeyboardLayout::Russian => "ru-RU",
            KeyboardLayout::Japanese => "ja-JP",
            KeyboardLayout::Korean => "ko-KR",
            KeyboardLayout::ChineseSimplified => "zh-CN",
        }
    }
}

impl Default for KeyboardLayout {
    fn default() -> Self {
        KeyboardLayout::UsEnglish
    }
}

// =============================================================================
// HOTKEY SUPPORT
// =============================================================================

/// Hotkey definition
#[derive(Debug, Clone, Copy)]
pub struct Hotkey {
    /// Scan code
    pub scancode: u16,
    /// Required modifiers
    pub modifiers: KeyModifiers,
    /// Action to perform
    pub action: HotkeyAction,
}

/// Hotkey action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    /// Enter BIOS setup
    EnterSetup,
    /// Boot from device
    BootDevice,
    /// Network boot
    NetworkBoot,
    /// Recovery mode
    Recovery,
    /// Boot menu
    BootMenu,
    /// Safe mode
    SafeMode,
    /// Command line
    CommandLine,
    /// Firmware update
    FirmwareUpdate,
    /// Reboot
    Reboot,
    /// Shutdown
    Shutdown,
    /// Custom action
    Custom(u32),
}

/// Common boot hotkeys
pub mod hotkeys {
    use super::*;

    /// F2 - Enter Setup
    pub const SETUP_F2: Hotkey = Hotkey {
        scancode: scancode::F2,
        modifiers: KeyModifiers::new(),
        action: HotkeyAction::EnterSetup,
    };

    /// F10 - Enter Setup (HP)
    pub const SETUP_F10: Hotkey = Hotkey {
        scancode: scancode::F10,
        modifiers: KeyModifiers::new(),
        action: HotkeyAction::EnterSetup,
    };

    /// F12 - Boot Menu
    pub const BOOT_MENU_F12: Hotkey = Hotkey {
        scancode: scancode::F12,
        modifiers: KeyModifiers::new(),
        action: HotkeyAction::BootMenu,
    };

    /// F8 - Safe Mode
    pub const SAFE_MODE_F8: Hotkey = Hotkey {
        scancode: scancode::F8,
        modifiers: KeyModifiers::new(),
        action: HotkeyAction::SafeMode,
    };

    /// Escape - Boot Menu
    pub const BOOT_MENU_ESC: Hotkey = Hotkey {
        scancode: scancode::ESCAPE,
        modifiers: KeyModifiers::new(),
        action: HotkeyAction::BootMenu,
    };

    /// Del - Enter Setup
    pub const SETUP_DEL: Hotkey = Hotkey {
        scancode: scancode::DELETE,
        modifiers: KeyModifiers::new(),
        action: HotkeyAction::EnterSetup,
    };
}

// =============================================================================
// INPUT BUFFER
// =============================================================================

/// Circular input buffer for key events
pub struct KeyBuffer<const N: usize> {
    /// Buffer storage
    buffer: [KeyEvent; N],
    /// Read position
    read: usize,
    /// Write position
    write: usize,
    /// Number of items
    count: usize,
}

impl<const N: usize> KeyBuffer<N> {
    /// Create new empty buffer
    pub const fn new() -> Self {
        Self {
            buffer: [KeyEvent::new(0, 0); N],
            read: 0,
            write: 0,
            count: 0,
        }
    }

    /// Check if buffer is empty
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Check if buffer is full
    pub const fn is_full(&self) -> bool {
        self.count == N
    }

    /// Get number of items
    pub const fn len(&self) -> usize {
        self.count
    }

    /// Push key event
    pub fn push(&mut self, event: KeyEvent) -> bool {
        if self.is_full() {
            return false;
        }
        self.buffer[self.write] = event;
        self.write = (self.write + 1) % N;
        self.count += 1;
        true
    }

    /// Pop key event
    pub fn pop(&mut self) -> Option<KeyEvent> {
        if self.is_empty() {
            return None;
        }
        let event = self.buffer[self.read];
        self.read = (self.read + 1) % N;
        self.count -= 1;
        Some(event)
    }

    /// Peek at next event without removing
    pub fn peek(&self) -> Option<&KeyEvent> {
        if self.is_empty() {
            None
        } else {
            Some(&self.buffer[self.read])
        }
    }

    /// Clear buffer
    pub fn clear(&mut self) {
        self.read = 0;
        self.write = 0;
        self.count = 0;
    }
}

impl<const N: usize> Default for KeyBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/// Input error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputError {
    /// No input device found
    NoDevice,
    /// Device not ready
    NotReady,
    /// Timeout waiting for input
    Timeout,
    /// Buffer overflow
    BufferOverflow,
    /// Invalid scancode
    InvalidScancode,
    /// Device error
    DeviceError,
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputError::NoDevice => write!(f, "No input device found"),
            InputError::NotReady => write!(f, "Device not ready"),
            InputError::Timeout => write!(f, "Input timeout"),
            InputError::BufferOverflow => write!(f, "Input buffer overflow"),
            InputError::InvalidScancode => write!(f, "Invalid scancode"),
            InputError::DeviceError => write!(f, "Device error"),
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_modifiers() {
        let mods = KeyModifiers {
            shift: true,
            control: true,
            ..Default::default()
        };
        assert!(mods.any());
        assert!(mods.ctrl_shift());
        assert!(!mods.ctrl_alt());
    }

    #[test]
    fn test_key_event() {
        let event = KeyEvent::new(scancode::F1, 0);
        assert!(event.is_function_key());
        assert!(!event.is_printable());
        assert!(!event.is_enter());

        let enter = KeyEvent::new(0, 0x0D);
        assert!(enter.is_enter());
    }

    #[test]
    fn test_mouse_buttons() {
        let buttons = MouseButtons::from_mask(0x03);
        assert!(buttons.left);
        assert!(buttons.right);
        assert!(!buttons.middle);
        assert_eq!(buttons.to_mask(), 0x03);
    }

    #[test]
    fn test_touch_event() {
        let mut event = TouchEvent::new();
        event.add_point(TouchPoint::new(0, 100, 200, TouchState::Down));
        assert_eq!(event.point_count, 1);
        assert!(event.primary().is_some());
    }

    #[test]
    fn test_key_buffer() {
        let mut buffer = KeyBuffer::<16>::new();
        assert!(buffer.is_empty());

        buffer.push(KeyEvent::new(scancode::F1, 0));
        assert_eq!(buffer.len(), 1);

        let event = buffer.pop().unwrap();
        assert_eq!(event.scancode, scancode::F1);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_keyboard_layout() {
        let layout = KeyboardLayout::French;
        assert_eq!(layout.name(), "French");
        assert_eq!(layout.locale(), "fr-FR");
    }
}
