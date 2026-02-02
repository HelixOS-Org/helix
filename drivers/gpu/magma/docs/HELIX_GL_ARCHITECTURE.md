# Helix-GL: OpenGL Translation Layer Architecture

**Version:** 0.1.0
**Status:** Design Phase
**Target:** OpenGL 3.3+ Core Profile → Vulkan 1.3

---

## Executive Summary

Helix-GL is a **Rust-native OpenGL translation layer** that converts OpenGL API calls into Vulkan commands executed by the Magma driver. This approach mirrors projects like **Zink** (Mesa) and **ANGLE** (Google), but with Rust's safety guarantees and zero-copy memory sharing with Magma.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              DATA FLOW                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌───────────┐ │
│  │ Legacy App   │    │  Helix-GL    │    │    Magma     │    │   GPU     │ │
│  │  (OpenGL)    │───▶│ Translation  │───▶│   Vulkan     │───▶│  (GSP)    │ │
│  │              │    │    Layer     │    │   Driver     │    │           │ │
│  └──────────────┘    └──────────────┘    └──────────────┘    └───────────┘ │
│                                                                              │
│       glDrawArrays()    VkCmdDraw()      Push Buffer        Hardware        │
│       glBindTexture()   VkCmdBind...()   MMIO Commands      Execution       │
│       glUseProgram()    Pipeline Bind    GSP RPC                            │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Table of Contents

1. [Design Philosophy](#design-philosophy)
2. [Architecture Overview](#architecture-overview)
3. [OpenGL State Machine in Rust](#opengl-state-machine-in-rust)
4. [Shader Translation Pipeline](#shader-translation-pipeline)
5. [Memory Model & Zero-Copy](#memory-model--zero-copy)
6. [Core Components](#core-components)
7. [Implementation Roadmap](#implementation-roadmap)
8. [API Mapping Reference](#api-mapping-reference)

---

## Design Philosophy

### Why Translation, Not Native Implementation?

| Approach | Pros | Cons |
|----------|------|------|
| **Native OpenGL Driver** | Maximum performance | Massive codebase, maintenance nightmare |
| **Translation Layer** | Leverage Vulkan work, smaller surface | Slight overhead, state tracking complexity |

We choose **translation** because:
1. **Magma already implements Vulkan** — we reuse 100% of that infrastructure
2. **OpenGL's state machine is complex** — better to map it than reimplement GPU interaction
3. **Rust makes state tracking safe** — compile-time guarantees on state validity

### Core Principles

```
┌─────────────────────────────────────────────────────────────────┐
│                    HELIX-GL DESIGN PRINCIPLES                   │
├─────────────────────────────────────────────────────────────────┤
│  1. ZERO-COPY        Shared memory between GL and Vulkan        │
│  2. LAZY EVALUATION  Defer Vulkan calls until draw time         │
│  3. STATE BATCHING   Minimize pipeline/descriptor rebuilds      │
│  4. TYPE SAFETY      Rust types enforce valid GL state          │
│  5. MODERN FIRST     OpenGL 3.3+ Core Profile only              │
└─────────────────────────────────────────────────────────────────┘
```

---

## Architecture Overview

### Layer Stack

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         APPLICATION LAYER                                │
│                                                                          │
│   ┌────────────────────────────────────────────────────────────────┐    │
│   │                    Legacy OpenGL Application                    │    │
│   │         glClear() | glDrawArrays() | glUseProgram()            │    │
│   └────────────────────────────────────────────────────────────────┘    │
│                                    │                                     │
│                                    ▼                                     │
├─────────────────────────────────────────────────────────────────────────┤
│                         HELIX-GL LAYER                                   │
│                                                                          │
│   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │
│   │ GL Dispatch │  │ State       │  │ Shader      │  │ Resource    │   │
│   │   Table     │  │ Tracker     │  │ Compiler    │  │ Manager     │   │
│   └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘   │
│          │                │                │                │           │
│          └────────────────┴────────────────┴────────────────┘           │
│                                    │                                     │
│                          ┌─────────▼─────────┐                          │
│                          │  Command Builder  │                          │
│                          │  (Vulkan Cmds)    │                          │
│                          └─────────┬─────────┘                          │
│                                    │                                     │
├────────────────────────────────────┼────────────────────────────────────┤
│                         MAGMA VULKAN DRIVER                              │
│                                    │                                     │
│   ┌─────────────┐  ┌─────────────┐ │ ┌─────────────┐  ┌─────────────┐   │
│   │ magma-vulkan│  │ magma-cmd   │◀┘ │ magma-mem   │  │ magma-rpc   │   │
│   └─────────────┘  └─────────────┘   └─────────────┘  └─────────────┘   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility |
|-----------|----------------|
| **GL Dispatch Table** | Maps `glXxx()` calls to internal handlers |
| **State Tracker** | Maintains OpenGL state machine in Rust structs |
| **Shader Compiler** | GLSL → SPIR-V translation via `naga` or `glslang` |
| **Resource Manager** | Maps GL names (GLuint) to Vulkan handles |
| **Command Builder** | Converts tracked state into VkCmd* calls |

---

## OpenGL State Machine in Rust

OpenGL's global state machine is the primary challenge. We model it with **typed Rust structs** that enforce valid state transitions at compile time.

### State Hierarchy

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          GlContext (Root)                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐       │
│  │   DrawState      │  │   TextureState   │  │   ShaderState    │       │
│  │  ────────────    │  │  ────────────    │  │  ────────────    │       │
│  │  bound_vao       │  │  texture_units[] │  │  current_program │       │
│  │  bound_program   │  │  active_unit     │  │  programs[]      │       │
│  │  viewport        │  │  samplers[]      │  │  shaders[]       │       │
│  │  scissor         │  │                  │  │                  │       │
│  │  blend_state     │  │                  │  │                  │       │
│  │  depth_state     │  │                  │  │                  │       │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘       │
│                                                                          │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐       │
│  │   BufferState    │  │  FramebufferState│  │   SyncState      │       │
│  │  ────────────    │  │  ────────────    │  │  ────────────    │       │
│  │  buffers[]       │  │  draw_fbo        │  │  fences[]        │       │
│  │  bound_array     │  │  read_fbo        │  │  queries[]       │       │
│  │  bound_element   │  │  default_fbo     │  │                  │       │
│  │  bound_uniform   │  │                  │  │                  │       │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘       │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Dirty Flag Pattern

State changes are tracked but Vulkan commands are **deferred until draw time**:

```
┌──────────────────────────────────────────────────────────────────────┐
│                      DIRTY FLAG WORKFLOW                              │
├──────────────────────────────────────────────────────────────────────┤
│                                                                       │
│  glUseProgram(5)                                                      │
│       │                                                               │
│       ▼                                                               │
│  ┌─────────────────────────────────────────┐                         │
│  │  state.shader.current_program = 5       │                         │
│  │  state.dirty_flags |= DIRTY_PIPELINE    │  ◀── Just mark dirty   │
│  └─────────────────────────────────────────┘                         │
│                                                                       │
│  glBindTexture(GL_TEXTURE_2D, 10)                                     │
│       │                                                               │
│       ▼                                                               │
│  ┌─────────────────────────────────────────┐                         │
│  │  state.texture.units[active] = 10       │                         │
│  │  state.dirty_flags |= DIRTY_DESCRIPTORS │                         │
│  └─────────────────────────────────────────┘                         │
│                                                                       │
│  glDrawArrays(GL_TRIANGLES, 0, 3)                                     │
│       │                                                               │
│       ▼                                                               │
│  ┌─────────────────────────────────────────┐                         │
│  │  if dirty_flags & DIRTY_PIPELINE:       │                         │
│  │      rebuild_pipeline()  ◀── Actual Vulkan work here              │
│  │  if dirty_flags & DIRTY_DESCRIPTORS:    │                         │
│  │      update_descriptor_sets()           │                         │
│  │  vkCmdDraw(...)                         │                         │
│  │  dirty_flags = 0                        │                         │
│  └─────────────────────────────────────────┘                         │
│                                                                       │
└──────────────────────────────────────────────────────────────────────┘
```

---

## Shader Translation Pipeline

### GLSL to SPIR-V Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     SHADER TRANSLATION PIPELINE                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌────────────┐    ┌────────────┐    ┌────────────┐    ┌────────────┐  │
│  │   GLSL     │    │   Parse    │    │  Validate  │    │  SPIR-V    │  │
│  │   Source   │───▶│   (Naga)   │───▶│  & Lower   │───▶│   Binary   │  │
│  │            │    │            │    │            │    │            │  │
│  └────────────┘    └────────────┘    └────────────┘    └────────────┘  │
│                                                                          │
│  #version 330        naga::front     naga::valid      naga::back        │
│  in vec3 pos;        ::glsl          ::Validator      ::spv             │
│  void main() {                                                           │
│    gl_Position =                                                         │
│      mvp * vec4(                                                         │
│        pos, 1.0);                                                        │
│  }                                                                       │
│                                                                          │
├─────────────────────────────────────────────────────────────────────────┤
│                        CACHING STRATEGY                                  │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  ShaderCache                                                     │    │
│  │  ───────────                                                     │    │
│  │  Key: blake3(glsl_source + version + stage)                      │    │
│  │  Value: Compiled SPIR-V + Reflection Data                        │    │
│  │                                                                   │    │
│  │  In-memory LRU cache + optional disk persistence                 │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Uniform Mapping Challenge

OpenGL uniforms are set individually; Vulkan uses descriptor sets:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    UNIFORM → DESCRIPTOR MAPPING                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  OpenGL Side:                      Vulkan Side:                          │
│  ─────────────                     ────────────                          │
│  glUniform1f(loc0, time)           ┌─────────────────────────┐          │
│  glUniform3f(loc1, x, y, z)        │  Uniform Buffer Object  │          │
│  glUniformMatrix4fv(loc2, mvp)     │  ─────────────────────  │          │
│                                    │  offset 0:  time (f32)  │          │
│       │                            │  offset 4:  padding     │          │
│       │                            │  offset 16: xyz (vec3)  │          │
│       ▼                            │  offset 32: mvp (mat4)  │          │
│  ┌──────────────────────┐          └─────────────────────────┘          │
│  │  Uniform Collector   │                    │                           │
│  │  ──────────────────  │                    ▼                           │
│  │  Batch all uniforms  │───────────▶  vkCmdBindDescriptorSets()        │
│  │  into single UBO     │                                                │
│  │  update per-draw     │                                                │
│  └──────────────────────┘                                                │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Memory Model & Zero-Copy

### Buffer Sharing Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                       ZERO-COPY BUFFER SHARING                           │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Application calls glBufferData():                                       │
│                                                                          │
│  ┌──────────────┐                                                        │
│  │ glBufferData │                                                        │
│  │ (target,     │                                                        │
│  │  size,       │                                                        │
│  │  data,       │                                                        │
│  │  usage)      │                                                        │
│  └──────┬───────┘                                                        │
│         │                                                                │
│         ▼                                                                │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    Helix-GL Buffer Manager                       │    │
│  │                                                                   │    │
│  │  1. Allocate via magma-mem (returns VkBuffer + VkDeviceMemory)   │    │
│  │  2. Store handle in GL name table                                │    │
│  │  3. NO COPY — same memory used by both GL and Vulkan             │    │
│  │                                                                   │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│         │                                                                │
│         ▼                                                                │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                       Magma VRAM                                  │    │
│  │                                                                   │    │
│  │  ┌─────────────────────────────────────────────────────────┐     │    │
│  │  │  VkBuffer (GL_ARRAY_BUFFER = VK_BUFFER_USAGE_VERTEX)    │     │    │
│  │  │  ─────────────────────────────────────────────────────  │     │    │
│  │  │  Backed by same VkDeviceMemory allocation               │     │    │
│  │  │  Used directly by vkCmdBindVertexBuffers()              │     │    │
│  │  └─────────────────────────────────────────────────────────┘     │    │
│  │                                                                   │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Memory Usage Mapping

| GL Usage Hint | Vulkan Memory Type | Strategy |
|---------------|-------------------|----------|
| `GL_STATIC_DRAW` | DEVICE_LOCAL | Upload once, no CPU access |
| `GL_DYNAMIC_DRAW` | DEVICE_LOCAL + HOST_VISIBLE | Staging + copy |
| `GL_STREAM_DRAW` | HOST_VISIBLE + HOST_COHERENT | Direct write each frame |

---

## Core Components

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        HELIX-GL COMPONENTS                               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                         magma-gl crate                           │    │
│  ├─────────────────────────────────────────────────────────────────┤    │
│  │                                                                   │    │
│  │  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐     │    │
│  │  │  context  │  │   state   │  │  shader   │  │  dispatch │     │    │
│  │  │  ───────  │  │  ───────  │  │  ───────  │  │  ───────  │     │    │
│  │  │ GlContext │  │ GlState   │  │ GlShader  │  │ gl_*()    │     │    │
│  │  │ GlConfig  │  │ DirtyFlags│  │ GlProgram │  │ entry pts │     │    │
│  │  │ EglSurface│  │ DrawState │  │ Compiler  │  │           │     │    │
│  │  └───────────┘  └───────────┘  └───────────┘  └───────────┘     │    │
│  │                                                                   │    │
│  │  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐     │    │
│  │  │  buffer   │  │  texture  │  │ framebuf  │  │  pipeline │     │    │
│  │  │  ───────  │  │  ───────  │  │  ───────  │  │  ───────  │     │    │
│  │  │ GlBuffer  │  │ GlTexture │  │ GlFbo     │  │ GlPipeline│     │    │
│  │  │ VaoState  │  │ Sampler   │  │ Renderbuf │  │ Cache     │     │    │
│  │  └───────────┘  └───────────┘  └───────────┘  └───────────┘     │    │
│  │                                                                   │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                    │                                     │
│                    Uses magma-vulkan API                                 │
│                                    │                                     │
│                                    ▼                                     │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                      magma-vulkan crate                          │    │
│  │         VkInstance, VkDevice, VkCommandBuffer, etc.              │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Roadmap

### Phase Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     IMPLEMENTATION PHASES                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Phase 1          Phase 2          Phase 3          Phase 4              │
│  Context &        Vertex           Draw Call        Textures             │
│  Surface          Buffers                                                │
│  ────────         ────────         ────────         ────────             │
│  Week 1-2         Week 3-4         Week 5-6         Week 7-8             │
│                                                                          │
│  ┌─────────┐      ┌─────────┐      ┌─────────┐      ┌─────────┐         │
│  │ EGL-like│      │ glGen   │      │ glDraw  │      │ glTex   │         │
│  │ surface │─────▶│ Buffers │─────▶│ Arrays  │─────▶│ Image2D │         │
│  │ context │      │ glBuffer│      │ Pipeline│      │ Sampler │         │
│  │         │      │ Data    │      │ Shaders │      │         │         │
│  └─────────┘      └─────────┘      └─────────┘      └─────────┘         │
│       │                │                │                │               │
│       ▼                ▼                ▼                ▼               │
│  VkInstance        VkBuffer         VkPipeline      VkImage             │
│  VkSurface         Zero-copy        VkShader        VkSampler           │
│  VkSwapchain       mapping          vkCmdDraw       Descriptors         │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Phase 1: Context Creation & Surface Binding

**Goal:** Create a GL context backed by Vulkan, bind to a window surface.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        PHASE 1: CONTEXT                                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Application:                                                            │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  // EGL-style context creation                                    │   │
│  │  let display = egl::get_display(native_window);                   │   │
│  │  let config = egl::choose_config(display, &attribs);              │   │
│  │  let surface = egl::create_window_surface(display, config, win);  │   │
│  │  let context = egl::create_context(display, config);              │   │
│  │  egl::make_current(display, surface, context);                    │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                    │                                     │
│                                    ▼                                     │
│  Helix-GL Internal:                                                      │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  1. Create VkInstance (via magma-vulkan)                          │   │
│  │  2. Create VkSurfaceKHR from native window handle                 │   │
│  │  3. Create VkDevice with graphics queue                           │   │
│  │  4. Create VkSwapchainKHR                                         │   │
│  │  5. Initialize GlContext with all state zeroed                    │   │
│  │  6. Set thread-local current context                              │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  Deliverables:                                                           │
│  ✓ egl::* API functions                                                 │
│  ✓ GlContext struct                                                     │
│  ✓ glGetString(GL_VERSION) returns "3.3.0 Helix-GL"                    │
│  ✓ glClear(GL_COLOR_BUFFER_BIT) produces colored screen                │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Phase 2: Vertex Buffer Mapping

**Goal:** GL buffers backed by Vulkan buffers with zero-copy.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        PHASE 2: BUFFERS                                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Application:                                                            │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  GLuint vbo;                                                      │   │
│  │  glGenBuffers(1, &vbo);                                           │   │
│  │  glBindBuffer(GL_ARRAY_BUFFER, vbo);                              │   │
│  │  glBufferData(GL_ARRAY_BUFFER, size, vertices, GL_STATIC_DRAW);   │   │
│  │                                                                    │   │
│  │  GLuint vao;                                                      │   │
│  │  glGenVertexArrays(1, &vao);                                      │   │
│  │  glBindVertexArray(vao);                                          │   │
│  │  glVertexAttribPointer(0, 3, GL_FLOAT, GL_FALSE, 0, 0);          │   │
│  │  glEnableVertexAttribArray(0);                                    │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                    │                                     │
│                                    ▼                                     │
│  Helix-GL Internal:                                                      │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  glGenBuffers:                                                    │   │
│  │    → Allocate GLuint name, no Vulkan object yet (lazy)            │   │
│  │                                                                    │   │
│  │  glBufferData:                                                    │   │
│  │    → Create VkBuffer via magma-mem                                │   │
│  │    → Upload data (staging or direct based on usage)               │   │
│  │    → Store in name→handle map                                     │   │
│  │                                                                    │   │
│  │  glVertexAttribPointer:                                           │   │
│  │    → Record in VAO state (binding, format, offset)                │   │
│  │    → Will become VkVertexInputAttributeDescription at draw        │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  Deliverables:                                                           │
│  ✓ glGenBuffers, glBindBuffer, glBufferData                             │
│  ✓ glGenVertexArrays, glBindVertexArray                                 │
│  ✓ glVertexAttribPointer, glEnableVertexAttribArray                     │
│  ✓ Zero-copy path for GL_STATIC_DRAW                                    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Phase 3: The Draw Call (Triangle)

**Goal:** Render a colored triangle using translated GL calls.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        PHASE 3: DRAW CALL                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Application:                                                            │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  // Shaders                                                       │   │
│  │  GLuint vs = glCreateShader(GL_VERTEX_SHADER);                    │   │
│  │  glShaderSource(vs, 1, &vsrc, NULL);                              │   │
│  │  glCompileShader(vs);                                             │   │
│  │                                                                    │   │
│  │  GLuint program = glCreateProgram();                              │   │
│  │  glAttachShader(program, vs);                                     │   │
│  │  glAttachShader(program, fs);                                     │   │
│  │  glLinkProgram(program);                                          │   │
│  │  glUseProgram(program);                                           │   │
│  │                                                                    │   │
│  │  // Draw                                                          │   │
│  │  glDrawArrays(GL_TRIANGLES, 0, 3);                                │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                    │                                     │
│                                    ▼                                     │
│  Helix-GL Internal:                                                      │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  glCompileShader:                                                 │   │
│  │    → Parse GLSL with naga::front::glsl                            │   │
│  │    → Validate with naga::valid::Validator                         │   │
│  │    → Emit SPIR-V with naga::back::spv                             │   │
│  │    → Cache result by source hash                                  │   │
│  │                                                                    │   │
│  │  glLinkProgram:                                                   │   │
│  │    → Create VkShaderModules from SPIR-V                           │   │
│  │    → Build reflection data (uniforms, attributes)                 │   │
│  │    → Create VkPipelineLayout                                      │   │
│  │                                                                    │   │
│  │  glDrawArrays:                                                    │   │
│  │    → Flush dirty state                                            │   │
│  │    → Build VkGraphicsPipeline (cached by state hash)              │   │
│  │    → vkCmdBindPipeline()                                          │   │
│  │    → vkCmdBindVertexBuffers()                                     │   │
│  │    → vkCmdDraw(3, 1, 0, 0)                                        │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  Deliverables:                                                           │
│  ✓ glCreateShader, glShaderSource, glCompileShader                      │
│  ✓ glCreateProgram, glAttachShader, glLinkProgram, glUseProgram         │
│  ✓ glDrawArrays, glDrawElements                                         │
│  ✓ GLSL → SPIR-V translation via naga                                   │
│  ✓ Pipeline state hashing & caching                                     │
│                                                                          │
│  SUCCESS METRIC: Colored triangle on screen                              │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Phase 4: Texture Sampling

**Goal:** Support textured rendering with samplers.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        PHASE 4: TEXTURES                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Application:                                                            │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  GLuint tex;                                                      │   │
│  │  glGenTextures(1, &tex);                                          │   │
│  │  glBindTexture(GL_TEXTURE_2D, tex);                               │   │
│  │  glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, w, h, 0,                 │   │
│  │               GL_RGBA, GL_UNSIGNED_BYTE, pixels);                 │   │
│  │  glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);│   │
│  │  glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);│   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                    │                                     │
│                                    ▼                                     │
│  Helix-GL Internal:                                                      │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  glTexImage2D:                                                    │   │
│  │    → Create VkImage (VK_IMAGE_USAGE_SAMPLED_BIT)                  │   │
│  │    → Create VkImageView                                           │   │
│  │    → Upload pixels via staging buffer                             │   │
│  │    → Transition layout: UNDEFINED → SHADER_READ_ONLY_OPTIMAL      │   │
│  │                                                                    │   │
│  │  glTexParameteri:                                                 │   │
│  │    → Create/update VkSampler with filter/wrap modes               │   │
│  │                                                                    │   │
│  │  At draw time:                                                    │   │
│  │    → Bind combined image sampler to descriptor set                │   │
│  │    → vkCmdBindDescriptorSets()                                    │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  Deliverables:                                                           │
│  ✓ glGenTextures, glBindTexture, glTexImage2D                           │
│  ✓ glTexParameteri (filter, wrap modes)                                 │
│  ✓ glActiveTexture, glUniform1i for sampler binding                     │
│  ✓ Descriptor set management for textures                               │
│                                                                          │
│  SUCCESS METRIC: Textured quad on screen                                 │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## API Mapping Reference

### Draw Calls

| OpenGL | Vulkan Equivalent |
|--------|-------------------|
| `glDrawArrays(mode, first, count)` | `vkCmdDraw(count, 1, first, 0)` |
| `glDrawElements(mode, count, type, indices)` | `vkCmdDrawIndexed(count, 1, 0, 0, 0)` |
| `glDrawArraysInstanced(...)` | `vkCmdDraw(..., instanceCount, ...)` |
| `glMultiDrawArrays(...)` | `vkCmdDrawIndirect(...)` |

### State Mapping

| OpenGL State | Vulkan Equivalent |
|--------------|-------------------|
| Blend enable/func | `VkPipelineColorBlendStateCreateInfo` |
| Depth test/func | `VkPipelineDepthStencilStateCreateInfo` |
| Cull face/front | `VkPipelineRasterizationStateCreateInfo` |
| Viewport/scissor | Dynamic state or `VkPipelineViewportStateCreateInfo` |
| Polygon mode | `VkPipelineRasterizationStateCreateInfo` |

### Format Mapping

| OpenGL Format | Vulkan Format |
|---------------|---------------|
| `GL_RGBA8` | `VK_FORMAT_R8G8B8A8_UNORM` |
| `GL_RGB8` | `VK_FORMAT_R8G8B8_UNORM` |
| `GL_DEPTH24_STENCIL8` | `VK_FORMAT_D24_UNORM_S8_UINT` |
| `GL_R32F` | `VK_FORMAT_R32_SFLOAT` |

---

## Success Criteria

| Phase | Metric | Verification |
|-------|--------|--------------|
| Phase 1 | Context creates, `glClear` works | Solid color screen |
| Phase 2 | Buffers allocate, VAO records state | No crashes, memory allocated |
| Phase 3 | Triangle renders | Visual confirmation |
| Phase 4 | Texture samples | Textured quad visible |

---

## Future Extensions

After core functionality:

1. **Framebuffer Objects** — FBO → VkFramebuffer mapping
2. **Uniform Buffer Objects** — UBO → VkBuffer with dynamic offset
3. **Transform Feedback** — Complex, may require compute shaders
4. **Geometry Shaders** — Direct GLSL→SPIR-V for geometry stage
5. **Compute Shaders** — `glDispatchCompute` → `vkCmdDispatch`

---

## Appendix: Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| GLSL Parser | `naga` | Pure Rust, MIT licensed, well-maintained |
| SPIR-V Backend | `naga::back::spv` | Integrated with parser |
| Shader Cache | `blake3` hash | Fast, collision-resistant |
| Memory Allocator | `magma-mem` | Already implemented, zero-copy |
| Thread Safety | `parking_lot` | Fast mutexes for context locks |
