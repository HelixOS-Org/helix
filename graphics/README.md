# Graphics Subsystem - Helix OS

This directory contains the Helix OS graphics stack, **separate from the GPU driver (MAGMA)**.

## Architecture

```
graphics/
├── lumina-core/      # Low-level graphics API (buffers, pipelines, shaders)
├── lumina-fx/        # High-level effects and rendering systems
├── lumina-math/      # Graphics mathematics (matrices, vectors, quaternions)
└── lumina-shader/    # Shader compilation and reflection
```

## Separation of Responsibilities

| Component | Responsibility | Dependencies |
|-----------|----------------|--------------|
| **MAGMA** (drivers/gpu/) | GPU hardware communication, GSP | Kernel |
| **lumina-core** | GPU abstractions, command buffers | MAGMA |
| **lumina-fx** | Sky, Water, Terrain, VFX | lumina-core |
| **lumina-shader** | SPIR-V, compilation, reflection | lumina-core |

## Full Stack

```
┌─────────────────────────────────────────────────────┐
│                    Application                       │
├─────────────────────────────────────────────────────┤
│                    lumina-fx                         │
│         (Sky, Water, Terrain, Particles, VFX)       │
├─────────────────────────────────────────────────────┤
│                   lumina-core                        │
│    (Buffers, Pipelines, Descriptors, Commands)      │
├─────────────────────────────────────────────────────┤
│                     MAGMA                            │
│         (GPU Driver - GSP communication)            │
├─────────────────────────────────────────────────────┤
│                  GPU Hardware                        │
└─────────────────────────────────────────────────────┘
```
