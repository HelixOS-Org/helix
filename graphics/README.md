# Graphics Subsystem - Helix OS

Ce dossier contient la stack graphique de Helix OS, **séparée du driver GPU (MAGMA)**.

## Architecture

```
graphics/
├── lumina-core/      # API graphique bas-niveau (buffers, pipelines, shaders)
├── lumina-fx/        # Effets et systèmes de rendu haut-niveau
├── lumina-math/      # Mathématiques graphiques (matrices, vecteurs, quaternions)
└── lumina-shader/    # Compilation et réflexion de shaders
```

## Séparation des responsabilités

| Composant | Responsabilité | Dépendances |
|-----------|----------------|-------------|
| **MAGMA** (drivers/gpu/) | Communication hardware GPU, GSP | Kernel |
| **lumina-core** | Abstractions GPU, command buffers | MAGMA |
| **lumina-fx** | Sky, Water, Terrain, VFX | lumina-core |
| **lumina-shader** | SPIR-V, compilation, réflexion | lumina-core |

## Stack complète

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
│         (Driver GPU - communication GSP)            │
├─────────────────────────────────────────────────────┤
│                  GPU Hardware                        │
└─────────────────────────────────────────────────────┘
```
