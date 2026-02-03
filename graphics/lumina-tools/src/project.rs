//! Project Management
//!
//! Project scaffolding and configuration.

use alloc::string::String;
use alloc::vec::Vec;

/// Project configuration
#[derive(Debug, Clone)]
pub struct ProjectConfig {
    /// Project name
    pub name: String,
    /// Version
    pub version: String,
    /// Authors
    pub authors: Vec<String>,
    /// Description
    pub description: Option<String>,
    /// Entry point
    pub entry: String,
    /// Shader directories
    pub shader_dirs: Vec<String>,
    /// Asset directories
    pub asset_dirs: Vec<String>,
    /// Build configuration
    pub build: BuildConfig,
    /// Target GPUs
    pub targets: Vec<TargetConfig>,
}

/// Build configuration
#[derive(Debug, Clone)]
pub struct BuildConfig {
    /// Output directory
    pub output_dir: String,
    /// Optimization level
    pub opt_level: u8,
    /// Debug info
    pub debug_info: bool,
    /// Validation layers
    pub validation: bool,
}

/// Target configuration
#[derive(Debug, Clone)]
pub struct TargetConfig {
    /// Target name
    pub name: String,
    /// API (vulkan, metal, dx12)
    pub api: String,
    /// Shader format
    pub shader_format: String,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: String::from("lumina-project"),
            version: String::from("0.1.0"),
            authors: Vec::new(),
            description: None,
            entry: String::from("src/main.rs"),
            shader_dirs: vec![String::from("shaders")],
            asset_dirs: vec![String::from("assets")],
            build: BuildConfig::default(),
            targets: vec![TargetConfig::default()],
        }
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            output_dir: String::from("build"),
            opt_level: 2,
            debug_info: true,
            validation: true,
        }
    }
}

impl Default for TargetConfig {
    fn default() -> Self {
        Self {
            name: String::from("default"),
            api: String::from("vulkan"),
            shader_format: String::from("spirv"),
        }
    }
}

/// Project template generator
pub struct ProjectGenerator;

impl ProjectGenerator {
    /// Generate minimal project
    pub fn generate_minimal(name: &str) -> ProjectFiles {
        ProjectFiles {
            config: ProjectConfig {
                name: name.into(),
                ..Default::default()
            },
            files: vec![
                (String::from("src/main.rs"), MINIMAL_MAIN.into()),
                (String::from("shaders/main.vert"), MINIMAL_VERT.into()),
                (String::from("shaders/main.frag"), MINIMAL_FRAG.into()),
            ],
        }
    }

    /// Generate triangle example
    pub fn generate_triangle(name: &str) -> ProjectFiles {
        ProjectFiles {
            config: ProjectConfig {
                name: name.into(),
                description: Some("Triangle example".into()),
                ..Default::default()
            },
            files: vec![
                (String::from("src/main.rs"), TRIANGLE_MAIN.into()),
                (String::from("shaders/triangle.vert"), TRIANGLE_VERT.into()),
                (String::from("shaders/triangle.frag"), TRIANGLE_FRAG.into()),
            ],
        }
    }

    /// Generate compute example
    pub fn generate_compute(name: &str) -> ProjectFiles {
        ProjectFiles {
            config: ProjectConfig {
                name: name.into(),
                description: Some("Compute example".into()),
                ..Default::default()
            },
            files: vec![
                (String::from("src/main.rs"), COMPUTE_MAIN.into()),
                (String::from("shaders/compute.comp"), COMPUTE_SHADER.into()),
            ],
        }
    }
}

/// Generated project files
#[derive(Debug, Clone)]
pub struct ProjectFiles {
    /// Configuration
    pub config: ProjectConfig,
    /// Files (path, content)
    pub files: Vec<(String, String)>,
}

// Template strings
const MINIMAL_MAIN: &str = r#"//! Minimal LUMINA Application

fn main() {
    println!("LUMINA Application");
}
"#;

const MINIMAL_VERT: &str = r#"#version 450

layout(location = 0) in vec3 position;

void main() {
    gl_Position = vec4(position, 1.0);
}
"#;

const MINIMAL_FRAG: &str = r#"#version 450

layout(location = 0) out vec4 color;

void main() {
    color = vec4(1.0, 1.0, 1.0, 1.0);
}
"#;

const TRIANGLE_MAIN: &str = r#"//! Triangle Example

fn main() {
    println!("Triangle Example");
}
"#;

const TRIANGLE_VERT: &str = r#"#version 450

layout(location = 0) out vec3 fragColor;

vec2 positions[3] = vec2[](
    vec2(0.0, -0.5),
    vec2(0.5, 0.5),
    vec2(-0.5, 0.5)
);

vec3 colors[3] = vec3[](
    vec3(1.0, 0.0, 0.0),
    vec3(0.0, 1.0, 0.0),
    vec3(0.0, 0.0, 1.0)
);

void main() {
    gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
    fragColor = colors[gl_VertexIndex];
}
"#;

const TRIANGLE_FRAG: &str = r#"#version 450

layout(location = 0) in vec3 fragColor;
layout(location = 0) out vec4 outColor;

void main() {
    outColor = vec4(fragColor, 1.0);
}
"#;

const COMPUTE_MAIN: &str = r#"//! Compute Example

fn main() {
    println!("Compute Example");
}
"#;

const COMPUTE_SHADER: &str = r#"#version 450

layout(local_size_x = 64) in;

layout(set = 0, binding = 0) buffer Data {
    float data[];
};

void main() {
    uint idx = gl_GlobalInvocationID.x;
    data[idx] = data[idx] * 2.0;
}
"#;
