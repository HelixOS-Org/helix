//! Lumina context and application management
//!
//! This module provides the main `Lumina` context that manages the
//! graphics device, swapchain, and frame submission.

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::{Error, Result};
use crate::frame::Frame;
use crate::graph::RenderGraph;
use crate::pipeline::PipelineCache;

/// The main Lumina context
///
/// This is the entry point for all Lumina operations. It manages:
/// - Device initialization
/// - Swapchain management
/// - Frame submission
/// - Resource lifetime
pub struct Lumina {
    /// Device name
    device_name: String,
    /// Window width
    width: u32,
    /// Window height
    height: u32,
    /// VSync enabled
    vsync: bool,
    /// Current frame index
    frame_index: u32,
    /// Time since start (seconds)
    time: f32,
    /// Delta time (seconds)
    delta_time: f32,
    /// Render graph
    graph: RenderGraph,
    /// Pipeline cache
    pipeline_cache: PipelineCache,
    /// Should close
    should_close: bool,
}

impl Lumina {
    /// Creates a new Lumina builder
    ///
    /// # Example
    ///
    /// ```rust
    /// let app = Lumina::init("My Application")?
    ///     .window(1280, 720)
    ///     .vsync(true)
    ///     .build()?;
    /// ```
    pub fn init(name: &str) -> Result<LuminaBuilder> {
        Ok(LuminaBuilder {
            name: String::from(name),
            width: 1280,
            height: 720,
            vsync: true,
            resizable: true,
            fullscreen: false,
        })
    }

    /// Returns the window width
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the window height
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns the window size as (width, height)
    #[inline]
    pub fn window_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns the aspect ratio
    #[inline]
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    /// Returns time since start in seconds
    #[inline]
    pub fn time(&self) -> f32 {
        self.time
    }

    /// Returns the device name
    #[inline]
    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    /// Runs the main application loop
    ///
    /// The callback receives a `Frame` for rendering and an `Input` for
    /// handling user input. Return `true` to continue, `false` to exit.
    ///
    /// # Example
    ///
    /// ```rust
    /// app.run(|frame, input| {
    ///     frame.render()
    ///         .clear(Color::BLACK)
    ///         .draw(&mesh)
    ///         .submit();
    ///     
    ///     !input.should_close()
    /// })?;
    /// ```
    pub fn run<F>(mut self, mut callback: F) -> Result<()>
    where
        F: FnMut(&mut Frame, &Input) -> bool,
    {
        let mut last_time = 0.0f32;

        loop {
            // Update timing
            self.time = self.get_time();
            self.delta_time = self.time - last_time;
            last_time = self.time;

            // Begin frame
            let mut frame = Frame {
                frame_index: self.frame_index,
                time: self.time,
                delta_time: self.delta_time,
                graph: &mut self.graph,
                width: self.width,
                height: self.height,
            };

            // Get input state
            let input = Input {
                should_close: self.should_close,
                keys: [false; 256],
                mouse_position: (0.0, 0.0),
                mouse_buttons: [false; 8],
            };

            // Call user callback
            if !callback(&mut frame, &input) {
                break;
            }

            // End frame - compile and submit render graph
            let compiled = core::mem::replace(&mut self.graph, RenderGraph::new()).compile();
            self.submit_frame(compiled)?;

            self.frame_index += 1;
            self.pipeline_cache.next_frame();
        }

        Ok(())
    }

    /// Begins a new frame for manual control
    pub fn begin_frame(&mut self) -> Frame<'_> {
        self.time = self.get_time();

        Frame {
            frame_index: self.frame_index,
            time: self.time,
            delta_time: self.delta_time,
            graph: &mut self.graph,
            width: self.width,
            height: self.height,
        }
    }

    /// Gets current time (placeholder)
    fn get_time(&self) -> f32 {
        // TODO: Use actual timer
        self.frame_index as f32 / 60.0
    }

    /// Submits a compiled frame to the GPU
    fn submit_frame(&mut self, _compiled: crate::graph::CompiledGraph) -> Result<()> {
        // TODO: Submit to Magma driver
        Ok(())
    }
}

/// Builder for creating a Lumina context
pub struct LuminaBuilder {
    name: String,
    width: u32,
    height: u32,
    vsync: bool,
    resizable: bool,
    fullscreen: bool,
}

impl LuminaBuilder {
    /// Sets the window size
    pub fn window(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Enables or disables VSync
    pub fn vsync(mut self, enabled: bool) -> Self {
        self.vsync = enabled;
        self
    }

    /// Makes the window resizable
    pub fn resizable(mut self, enabled: bool) -> Self {
        self.resizable = enabled;
        self
    }

    /// Enables fullscreen mode
    pub fn fullscreen(mut self, enabled: bool) -> Self {
        self.fullscreen = enabled;
        self
    }

    /// Builds the Lumina context
    pub fn build(self) -> Result<Lumina> {
        // TODO: Initialize Magma driver
        // TODO: Create window
        // TODO: Create swapchain

        Ok(Lumina {
            device_name: String::from("Magma Virtual Device"),
            width: self.width,
            height: self.height,
            vsync: self.vsync,
            frame_index: 0,
            time: 0.0,
            delta_time: 0.0,
            graph: RenderGraph::new(),
            pipeline_cache: PipelineCache::new(1024),
            should_close: false,
        })
    }
}

/// Input state for the current frame
pub struct Input {
    /// Whether close was requested
    should_close: bool,
    /// Key states (true = pressed)
    keys: [bool; 256],
    /// Mouse position (pixels)
    mouse_position: (f32, f32),
    /// Mouse button states
    mouse_buttons: [bool; 8],
}

impl Input {
    /// Returns true if close was requested
    #[inline]
    pub fn should_close(&self) -> bool {
        self.should_close
    }

    /// Returns true if a key is pressed
    #[inline]
    pub fn key_pressed(&self, key: Key) -> bool {
        self.keys.get(key as usize).copied().unwrap_or(false)
    }

    /// Returns the mouse position in pixels
    #[inline]
    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    /// Returns true if a mouse button is pressed
    #[inline]
    pub fn mouse_button(&self, button: MouseButton) -> bool {
        self.mouse_buttons
            .get(button as usize)
            .copied()
            .unwrap_or(false)
    }
}

/// Keyboard keys
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Key {
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
    Num0 = 26,
    Num1 = 27,
    Num2 = 28,
    Num3 = 29,
    Num4 = 30,
    Num5 = 31,
    Num6 = 32,
    Num7 = 33,
    Num8 = 34,
    Num9 = 35,
    Escape = 36,
    Space = 37,
    Enter = 38,
    Tab = 39,
    Backspace = 40,
    Left = 41,
    Right = 42,
    Up = 43,
    Down = 44,
    LeftShift = 45,
    RightShift = 46,
    LeftControl = 47,
    RightControl = 48,
    LeftAlt = 49,
    RightAlt = 50,
}

/// Mouse buttons
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MouseButton {
    Left = 0,
    Right = 1,
    Middle = 2,
    Button4 = 3,
    Button5 = 4,
}
