//! GPU Fluid Simulation System for Lumina
//!
//! This module provides comprehensive GPU-accelerated fluid simulation including
//! grid-based solvers, SPH particles, and multi-phase fluids.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Fluid System Handles
// ============================================================================

/// GPU fluid system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuFluidSystemHandle(pub u64);

impl GpuFluidSystemHandle {
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

impl Default for GpuFluidSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Fluid volume handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FluidVolumeHandle(pub u64);

impl FluidVolumeHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Is null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for FluidVolumeHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Fluid emitter handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FluidEmitterHandle(pub u64);

impl FluidEmitterHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for FluidEmitterHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Fluid collider handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FluidColliderHandle(pub u64);

impl FluidColliderHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for FluidColliderHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Fluid System Creation
// ============================================================================

/// GPU fluid system create info
#[derive(Clone, Debug)]
pub struct GpuFluidSystemCreateInfo {
    /// Name
    pub name: String,
    /// Simulation type
    pub sim_type: FluidSimType,
    /// Max particles (for SPH)
    pub max_particles: u32,
    /// Grid resolution (for grid-based)
    pub grid_resolution: [u32; 3],
    /// Features
    pub features: FluidFeatures,
    /// Solver settings
    pub solver: FluidSolverSettings,
}

impl GpuFluidSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            sim_type: FluidSimType::Hybrid,
            max_particles: 100_000,
            grid_resolution: [64, 64, 64],
            features: FluidFeatures::all(),
            solver: FluidSolverSettings::default(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With simulation type
    pub fn with_sim_type(mut self, sim_type: FluidSimType) -> Self {
        self.sim_type = sim_type;
        self
    }

    /// With max particles
    pub fn with_max_particles(mut self, count: u32) -> Self {
        self.max_particles = count;
        self
    }

    /// With grid resolution
    pub fn with_resolution(mut self, res: [u32; 3]) -> Self {
        self.grid_resolution = res;
        self
    }

    /// With features
    pub fn with_features(mut self, features: FluidFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With solver
    pub fn with_solver(mut self, solver: FluidSolverSettings) -> Self {
        self.solver = solver;
        self
    }

    /// Standard preset
    pub fn standard() -> Self {
        Self::new()
    }

    /// High quality preset
    pub fn high_quality() -> Self {
        Self::new()
            .with_max_particles(500_000)
            .with_resolution([128, 128, 128])
            .with_solver(FluidSolverSettings::high_quality())
    }

    /// Mobile preset
    pub fn mobile() -> Self {
        Self::new()
            .with_max_particles(10_000)
            .with_resolution([32, 32, 32])
            .with_features(FluidFeatures::BASIC)
            .with_solver(FluidSolverSettings::mobile())
    }
}

impl Default for GpuFluidSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Fluid simulation type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FluidSimType {
    /// Grid-based Eulerian
    Grid   = 0,
    /// SPH particles
    Sph    = 1,
    /// FLIP/PIC hybrid
    #[default]
    Hybrid = 2,
    /// Position-based fluids
    Pbf    = 3,
}

bitflags::bitflags! {
    /// Fluid features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct FluidFeatures: u32 {
        /// None
        const NONE = 0;
        /// Advection
        const ADVECTION = 1 << 0;
        /// Pressure solve
        const PRESSURE = 1 << 1;
        /// Viscosity
        const VISCOSITY = 1 << 2;
        /// Surface tension
        const SURFACE_TENSION = 1 << 3;
        /// Vorticity confinement
        const VORTICITY = 1 << 4;
        /// Multi-phase
        const MULTI_PHASE = 1 << 5;
        /// Foam/spray
        const FOAM = 1 << 6;
        /// Buoyancy
        const BUOYANCY = 1 << 7;
        /// Basic
        const BASIC = Self::ADVECTION.bits() | Self::PRESSURE.bits();
        /// All
        const ALL = 0xFF;
    }
}

impl Default for FluidFeatures {
    fn default() -> Self {
        Self::all()
    }
}

// ============================================================================
// Solver Settings
// ============================================================================

/// Fluid solver settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FluidSolverSettings {
    /// Pressure iterations
    pub pressure_iterations: u32,
    /// Viscosity iterations
    pub viscosity_iterations: u32,
    /// Time step
    pub time_step: f32,
    /// Substeps
    pub substeps: u32,
    /// CFL number
    pub cfl: f32,
    /// Jacobi omega (relaxation)
    pub jacobi_omega: f32,
    /// Solver type
    pub solver_type: FluidSolverType,
}

impl FluidSolverSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            pressure_iterations: 50,
            viscosity_iterations: 10,
            time_step: 0.016,
            substeps: 4,
            cfl: 1.0,
            jacobi_omega: 1.0,
            solver_type: FluidSolverType::Jacobi,
        }
    }

    /// With pressure iterations
    pub const fn with_pressure_iters(mut self, iters: u32) -> Self {
        self.pressure_iterations = iters;
        self
    }

    /// With substeps
    pub const fn with_substeps(mut self, substeps: u32) -> Self {
        self.substeps = substeps;
        self
    }

    /// With CFL
    pub const fn with_cfl(mut self, cfl: f32) -> Self {
        self.cfl = cfl;
        self
    }

    /// High quality preset
    pub const fn high_quality() -> Self {
        Self {
            pressure_iterations: 100,
            viscosity_iterations: 20,
            time_step: 0.008,
            substeps: 8,
            cfl: 0.5,
            jacobi_omega: 1.0,
            solver_type: FluidSolverType::Multigrid,
        }
    }

    /// Mobile preset
    pub const fn mobile() -> Self {
        Self {
            pressure_iterations: 20,
            viscosity_iterations: 5,
            time_step: 0.033,
            substeps: 2,
            cfl: 2.0,
            jacobi_omega: 1.2,
            solver_type: FluidSolverType::Jacobi,
        }
    }
}

impl Default for FluidSolverSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Fluid solver type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FluidSolverType {
    /// Jacobi iteration
    #[default]
    Jacobi            = 0,
    /// Gauss-Seidel
    GaussSeidel       = 1,
    /// Multigrid
    Multigrid         = 2,
    /// Conjugate gradient
    ConjugateGradient = 3,
}

// ============================================================================
// Fluid Properties
// ============================================================================

/// Fluid properties
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FluidProperties {
    /// Density (kg/m³)
    pub density: f32,
    /// Viscosity (Pa·s)
    pub viscosity: f32,
    /// Surface tension coefficient
    pub surface_tension: f32,
    /// Stiffness (for SPH)
    pub stiffness: f32,
    /// Rest density (for SPH)
    pub rest_density: f32,
    /// Particle radius
    pub particle_radius: f32,
    /// Gravity
    pub gravity: [f32; 3],
    /// Temperature (K)
    pub temperature: f32,
}

impl FluidProperties {
    /// Creates new properties
    pub const fn new() -> Self {
        Self {
            density: 1000.0,
            viscosity: 0.001,
            surface_tension: 0.072,
            stiffness: 50.0,
            rest_density: 1000.0,
            particle_radius: 0.02,
            gravity: [0.0, -9.81, 0.0],
            temperature: 293.0, // 20°C
        }
    }

    /// With density
    pub const fn with_density(mut self, density: f32) -> Self {
        self.density = density;
        self.rest_density = density;
        self
    }

    /// With viscosity
    pub const fn with_viscosity(mut self, viscosity: f32) -> Self {
        self.viscosity = viscosity;
        self
    }

    /// With surface tension
    pub const fn with_surface_tension(mut self, tension: f32) -> Self {
        self.surface_tension = tension;
        self
    }

    /// With gravity
    pub const fn with_gravity(mut self, gravity: [f32; 3]) -> Self {
        self.gravity = gravity;
        self
    }

    /// Water preset
    pub const fn water() -> Self {
        Self {
            density: 1000.0,
            viscosity: 0.001,
            surface_tension: 0.072,
            stiffness: 50.0,
            rest_density: 1000.0,
            particle_radius: 0.02,
            gravity: [0.0, -9.81, 0.0],
            temperature: 293.0,
        }
    }

    /// Honey preset
    pub const fn honey() -> Self {
        Self {
            density: 1400.0,
            viscosity: 5.0,
            surface_tension: 0.08,
            stiffness: 30.0,
            rest_density: 1400.0,
            particle_radius: 0.025,
            gravity: [0.0, -9.81, 0.0],
            temperature: 293.0,
        }
    }

    /// Oil preset
    pub const fn oil() -> Self {
        Self {
            density: 900.0,
            viscosity: 0.1,
            surface_tension: 0.03,
            stiffness: 40.0,
            rest_density: 900.0,
            particle_radius: 0.02,
            gravity: [0.0, -9.81, 0.0],
            temperature: 293.0,
        }
    }

    /// Lava preset
    pub const fn lava() -> Self {
        Self {
            density: 2500.0,
            viscosity: 100.0,
            surface_tension: 0.4,
            stiffness: 20.0,
            rest_density: 2500.0,
            particle_radius: 0.05,
            gravity: [0.0, -9.81, 0.0],
            temperature: 1273.0, // 1000°C
        }
    }

    /// Gas/smoke preset
    pub const fn gas() -> Self {
        Self {
            density: 1.2,
            viscosity: 0.00001,
            surface_tension: 0.0,
            stiffness: 10.0,
            rest_density: 1.2,
            particle_radius: 0.05,
            gravity: [0.0, 0.1, 0.0], // Buoyant
            temperature: 293.0,
        }
    }
}

impl Default for FluidProperties {
    fn default() -> Self {
        Self::water()
    }
}

// ============================================================================
// Fluid Volume
// ============================================================================

/// Fluid volume create info
#[derive(Clone, Debug)]
pub struct FluidVolumeCreateInfo {
    /// Name
    pub name: String,
    /// Bounds
    pub bounds: FluidBounds,
    /// Properties
    pub properties: FluidProperties,
    /// Initial fill
    pub initial_fill: FluidFill,
    /// Rendering settings
    pub rendering: FluidRenderSettings,
}

impl FluidVolumeCreateInfo {
    /// Creates new volume
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            bounds: FluidBounds::default(),
            properties: FluidProperties::default(),
            initial_fill: FluidFill::None,
            rendering: FluidRenderSettings::default(),
        }
    }

    /// With bounds
    pub fn with_bounds(mut self, bounds: FluidBounds) -> Self {
        self.bounds = bounds;
        self
    }

    /// With properties
    pub fn with_properties(mut self, properties: FluidProperties) -> Self {
        self.properties = properties;
        self
    }

    /// With initial fill
    pub fn with_fill(mut self, fill: FluidFill) -> Self {
        self.initial_fill = fill;
        self
    }

    /// With rendering
    pub fn with_rendering(mut self, rendering: FluidRenderSettings) -> Self {
        self.rendering = rendering;
        self
    }

    /// Water pool preset
    pub fn water_pool(name: impl Into<String>, size: [f32; 3]) -> Self {
        let half = [size[0] * 0.5, size[1] * 0.5, size[2] * 0.5];
        Self::new(name)
            .with_bounds(FluidBounds::new([-half[0], 0.0, -half[2]], [
                half[0], size[1], half[2],
            ]))
            .with_properties(FluidProperties::water())
            .with_fill(FluidFill::Box {
                min: [-half[0] * 0.9, 0.0, -half[2] * 0.9],
                max: [half[0] * 0.9, size[1] * 0.5, half[2] * 0.9],
            })
    }

    /// Lava pool preset
    pub fn lava_pool(name: impl Into<String>) -> Self {
        Self::new(name)
            .with_properties(FluidProperties::lava())
            .with_rendering(FluidRenderSettings::lava())
    }
}

impl Default for FluidVolumeCreateInfo {
    fn default() -> Self {
        Self::new("FluidVolume")
    }
}

/// Fluid bounds
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FluidBounds {
    /// Min corner
    pub min: [f32; 3],
    /// Max corner
    pub max: [f32; 3],
    /// Open boundaries (bitmask)
    pub open_boundaries: u32,
}

impl FluidBounds {
    /// Creates new bounds
    pub const fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self {
            min,
            max,
            open_boundaries: 0,
        }
    }

    /// Unit bounds
    pub const fn unit() -> Self {
        Self {
            min: [0.0, 0.0, 0.0],
            max: [1.0, 1.0, 1.0],
            open_boundaries: 0,
        }
    }

    /// With open top
    pub const fn with_open_top(mut self) -> Self {
        self.open_boundaries |= 0x20; // +Y
        self
    }

    /// Size
    pub const fn size(&self) -> [f32; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }

    /// Volume
    pub fn volume(&self) -> f32 {
        let size = self.size();
        size[0] * size[1] * size[2]
    }
}

impl Default for FluidBounds {
    fn default() -> Self {
        Self::unit()
    }
}

/// Fluid fill
#[derive(Clone, Copy, Debug)]
pub enum FluidFill {
    /// No initial fill
    None,
    /// Fill box region
    Box { min: [f32; 3], max: [f32; 3] },
    /// Fill sphere region
    Sphere { center: [f32; 3], radius: f32 },
    /// Fill entire volume
    Full,
    /// Fill percentage
    Percentage(f32),
}

impl Default for FluidFill {
    fn default() -> Self {
        Self::None
    }
}

// ============================================================================
// Fluid Rendering
// ============================================================================

/// Fluid render settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FluidRenderSettings {
    /// Render mode
    pub mode: FluidRenderMode,
    /// Color
    pub color: [f32; 4],
    /// Refraction
    pub refraction: f32,
    /// Reflection
    pub reflection: f32,
    /// Absorption
    pub absorption: [f32; 3],
    /// Scattering
    pub scattering: f32,
    /// Foam color
    pub foam_color: [f32; 4],
    /// Foam threshold
    pub foam_threshold: f32,
    /// Smoothing radius
    pub smoothing_radius: f32,
}

impl FluidRenderSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            mode: FluidRenderMode::ScreenSpace,
            color: [0.0, 0.3, 0.8, 0.9],
            refraction: 1.33,
            reflection: 0.3,
            absorption: [0.05, 0.03, 0.01],
            scattering: 0.02,
            foam_color: [1.0, 1.0, 1.0, 0.8],
            foam_threshold: 0.5,
            smoothing_radius: 0.1,
        }
    }

    /// With color
    pub const fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// With render mode
    pub const fn with_mode(mut self, mode: FluidRenderMode) -> Self {
        self.mode = mode;
        self
    }

    /// Water preset
    pub const fn water() -> Self {
        Self::new()
    }

    /// Lava preset
    pub const fn lava() -> Self {
        Self {
            mode: FluidRenderMode::Particles,
            color: [1.0, 0.3, 0.0, 1.0],
            refraction: 1.5,
            reflection: 0.1,
            absorption: [0.0, 0.1, 0.3],
            scattering: 0.5,
            foam_color: [1.0, 0.8, 0.0, 1.0],
            foam_threshold: 0.3,
            smoothing_radius: 0.15,
        }
    }

    /// Oil preset
    pub const fn oil() -> Self {
        Self {
            mode: FluidRenderMode::ScreenSpace,
            color: [0.1, 0.1, 0.05, 0.95],
            refraction: 1.47,
            reflection: 0.5,
            absorption: [0.4, 0.4, 0.3],
            scattering: 0.01,
            foam_color: [0.3, 0.3, 0.2, 0.5],
            foam_threshold: 0.7,
            smoothing_radius: 0.08,
        }
    }
}

impl Default for FluidRenderSettings {
    fn default() -> Self {
        Self::water()
    }
}

/// Fluid render mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FluidRenderMode {
    /// Screen-space rendering
    #[default]
    ScreenSpace = 0,
    /// Particle splatting
    Particles   = 1,
    /// Marching cubes mesh
    Mesh        = 2,
    /// Ray marched
    RayMarched  = 3,
}

// ============================================================================
// Fluid Emitter
// ============================================================================

/// Fluid emitter create info
#[derive(Clone, Debug)]
pub struct FluidEmitterCreateInfo {
    /// Name
    pub name: String,
    /// Emitter shape
    pub shape: EmitterShape,
    /// Position
    pub position: [f32; 3],
    /// Direction
    pub direction: [f32; 3],
    /// Emission rate (particles/second)
    pub rate: f32,
    /// Initial velocity
    pub velocity: f32,
    /// Velocity variation
    pub velocity_variation: f32,
    /// Temperature (for buoyancy)
    pub temperature: f32,
}

impl FluidEmitterCreateInfo {
    /// Creates new emitter
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            shape: EmitterShape::Point,
            position: [0.0; 3],
            direction: [0.0, -1.0, 0.0],
            rate: 1000.0,
            velocity: 5.0,
            velocity_variation: 0.2,
            temperature: 293.0,
        }
    }

    /// At position
    pub fn at(mut self, x: f32, y: f32, z: f32) -> Self {
        self.position = [x, y, z];
        self
    }

    /// With direction
    pub fn with_direction(mut self, dir: [f32; 3]) -> Self {
        self.direction = dir;
        self
    }

    /// With rate
    pub fn with_rate(mut self, rate: f32) -> Self {
        self.rate = rate;
        self
    }

    /// With velocity
    pub fn with_velocity(mut self, velocity: f32) -> Self {
        self.velocity = velocity;
        self
    }

    /// With shape
    pub fn with_shape(mut self, shape: EmitterShape) -> Self {
        self.shape = shape;
        self
    }

    /// Faucet preset
    pub fn faucet(name: impl Into<String>, position: [f32; 3]) -> Self {
        Self::new(name)
            .at(position[0], position[1], position[2])
            .with_direction([0.0, -1.0, 0.0])
            .with_rate(500.0)
            .with_velocity(3.0)
            .with_shape(EmitterShape::Disk { radius: 0.05 })
    }

    /// Hose preset
    pub fn hose(name: impl Into<String>, position: [f32; 3], direction: [f32; 3]) -> Self {
        Self::new(name)
            .at(position[0], position[1], position[2])
            .with_direction(direction)
            .with_rate(2000.0)
            .with_velocity(10.0)
            .with_shape(EmitterShape::Disk { radius: 0.02 })
    }
}

impl Default for FluidEmitterCreateInfo {
    fn default() -> Self {
        Self::new("Emitter")
    }
}

/// Emitter shape
#[derive(Clone, Copy, Debug)]
pub enum EmitterShape {
    /// Point emitter
    Point,
    /// Disk emitter
    Disk { radius: f32 },
    /// Box emitter
    Box { half_extents: [f32; 3] },
    /// Sphere emitter
    Sphere { radius: f32 },
    /// Mesh surface emitter
    Mesh { mesh_id: u64 },
}

impl Default for EmitterShape {
    fn default() -> Self {
        Self::Point
    }
}

// ============================================================================
// Fluid Collider
// ============================================================================

/// Fluid collider create info
#[derive(Clone, Debug)]
pub struct FluidColliderCreateInfo {
    /// Name
    pub name: String,
    /// Collider type
    pub collider_type: FluidColliderType,
    /// Position
    pub position: [f32; 3],
    /// Rotation (quaternion)
    pub rotation: [f32; 4],
    /// Scale
    pub scale: [f32; 3],
    /// Friction
    pub friction: f32,
    /// Is static
    pub is_static: bool,
}

impl FluidColliderCreateInfo {
    /// Creates new collider
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            collider_type: FluidColliderType::Box {
                half_extents: [0.5; 3],
            },
            position: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            friction: 0.5,
            is_static: true,
        }
    }

    /// Box collider
    pub fn box_collider(name: impl Into<String>, half_extents: [f32; 3]) -> Self {
        Self::new(name).with_type(FluidColliderType::Box { half_extents })
    }

    /// Sphere collider
    pub fn sphere_collider(name: impl Into<String>, radius: f32) -> Self {
        Self::new(name).with_type(FluidColliderType::Sphere { radius })
    }

    /// With type
    pub fn with_type(mut self, collider_type: FluidColliderType) -> Self {
        self.collider_type = collider_type;
        self
    }

    /// At position
    pub fn at(mut self, x: f32, y: f32, z: f32) -> Self {
        self.position = [x, y, z];
        self
    }

    /// With friction
    pub fn with_friction(mut self, friction: f32) -> Self {
        self.friction = friction;
        self
    }

    /// Dynamic collider
    pub fn dynamic(mut self) -> Self {
        self.is_static = false;
        self
    }
}

impl Default for FluidColliderCreateInfo {
    fn default() -> Self {
        Self::new("Collider")
    }
}

/// Fluid collider type
#[derive(Clone, Copy, Debug)]
pub enum FluidColliderType {
    /// Box collider
    Box { half_extents: [f32; 3] },
    /// Sphere collider
    Sphere { radius: f32 },
    /// Capsule collider
    Capsule { radius: f32, height: f32 },
    /// Plane collider
    Plane { normal: [f32; 3] },
    /// SDF collider
    Sdf { volume_id: u64 },
    /// Mesh collider
    Mesh { mesh_id: u64 },
}

impl Default for FluidColliderType {
    fn default() -> Self {
        Self::Box {
            half_extents: [0.5; 3],
        }
    }
}

// ============================================================================
// SPH Specific
// ============================================================================

/// SPH kernel type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SphKernel {
    /// Poly6 kernel
    #[default]
    Poly6       = 0,
    /// Spiky kernel
    Spiky       = 1,
    /// Viscosity kernel
    Viscosity   = 2,
    /// Cubic spline
    CubicSpline = 3,
    /// Wendland
    Wendland    = 4,
}

/// SPH settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SphSettings {
    /// Smoothing length
    pub smoothing_length: f32,
    /// Kernel type
    pub kernel: SphKernel,
    /// Pressure coefficient
    pub pressure_coefficient: f32,
    /// Viscosity coefficient
    pub viscosity_coefficient: f32,
    /// Surface tension coefficient
    pub tension_coefficient: f32,
    /// Rest density
    pub rest_density: f32,
    /// Gas constant
    pub gas_constant: f32,
    /// Max neighbors
    pub max_neighbors: u32,
}

impl SphSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            smoothing_length: 0.1,
            kernel: SphKernel::Poly6,
            pressure_coefficient: 50.0,
            viscosity_coefficient: 0.01,
            tension_coefficient: 0.0728,
            rest_density: 1000.0,
            gas_constant: 2000.0,
            max_neighbors: 64,
        }
    }

    /// With smoothing length
    pub const fn with_smoothing(mut self, length: f32) -> Self {
        self.smoothing_length = length;
        self
    }

    /// With kernel
    pub const fn with_kernel(mut self, kernel: SphKernel) -> Self {
        self.kernel = kernel;
        self
    }
}

impl Default for SphSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// GPU fluid particle
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct GpuFluidParticle {
    /// Position
    pub position: [f32; 3],
    /// Density
    pub density: f32,
    /// Velocity
    pub velocity: [f32; 3],
    /// Pressure
    pub pressure: f32,
    /// Force
    pub force: [f32; 3],
    /// Mass
    pub mass: f32,
    /// Color/phase
    pub color: [f32; 4],
}

/// GPU fluid grid cell
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct GpuFluidCell {
    /// Velocity U component
    pub velocity_u: f32,
    /// Velocity V component
    pub velocity_v: f32,
    /// Velocity W component
    pub velocity_w: f32,
    /// Pressure
    pub pressure: f32,
    /// Density
    pub density: f32,
    /// Temperature
    pub temperature: f32,
    /// Divergence
    pub divergence: f32,
    /// Cell type (0=air, 1=fluid, 2=solid)
    pub cell_type: u32,
}

/// GPU fluid constants
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuFluidConstants {
    /// Grid dimensions
    pub grid_dims: [u32; 3],
    /// Particle count
    pub particle_count: u32,
    /// Grid cell size
    pub cell_size: f32,
    /// Time step
    pub dt: f32,
    /// Gravity
    pub gravity: [f32; 3],
    /// Viscosity
    pub viscosity: f32,
    /// Surface tension
    pub surface_tension: f32,
    /// Density
    pub density: f32,
    /// Stiffness
    pub stiffness: f32,
    /// Smoothing length (SPH)
    pub smoothing_length: f32,
    /// Time
    pub time: f32,
    /// Flags
    pub flags: u32,
    /// Pad
    pub _pad: [f32; 2],
}

impl Default for GpuFluidConstants {
    fn default() -> Self {
        Self {
            grid_dims: [64; 3],
            particle_count: 0,
            cell_size: 0.1,
            dt: 0.016,
            gravity: [0.0, -9.81, 0.0],
            viscosity: 0.001,
            surface_tension: 0.072,
            density: 1000.0,
            stiffness: 50.0,
            smoothing_length: 0.1,
            time: 0.0,
            flags: 0,
            _pad: [0.0; 2],
        }
    }
}

// ============================================================================
// Fluid Statistics
// ============================================================================

/// Fluid simulation statistics
#[derive(Clone, Debug, Default)]
pub struct GpuFluidStats {
    /// Active particles
    pub active_particles: u32,
    /// Max particles
    pub max_particles: u32,
    /// Grid cells
    pub grid_cells: u32,
    /// Fluid cells
    pub fluid_cells: u32,
    /// Simulation time (ms)
    pub sim_time_ms: f32,
    /// Pressure solve time (ms)
    pub pressure_time_ms: f32,
    /// Advection time (ms)
    pub advection_time_ms: f32,
    /// Render time (ms)
    pub render_time_ms: f32,
    /// Average velocity
    pub avg_velocity: f32,
    /// Max velocity
    pub max_velocity: f32,
    /// Total volume (m³)
    pub total_volume: f32,
}

impl GpuFluidStats {
    /// Particle fill ratio
    pub fn fill_ratio(&self) -> f32 {
        if self.max_particles > 0 {
            self.active_particles as f32 / self.max_particles as f32
        } else {
            0.0
        }
    }

    /// Fluid cell ratio
    pub fn fluid_cell_ratio(&self) -> f32 {
        if self.grid_cells > 0 {
            self.fluid_cells as f32 / self.grid_cells as f32
        } else {
            0.0
        }
    }

    /// Total time (ms)
    pub fn total_time_ms(&self) -> f32 {
        self.sim_time_ms + self.render_time_ms
    }

    /// Particles in millions
    pub fn particles_millions(&self) -> f32 {
        self.active_particles as f32 / 1_000_000.0
    }
}
