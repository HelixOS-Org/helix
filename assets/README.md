# Helix OS — Assets

All visual assets for the Helix OS project. Every file is **SVG** (vector) — no pixelization at any scale.

---

## Files

### Core Identity

| File | Description | Dimensions |
|------|-------------|------------|
| `logo.svg` | Primary helix logo with blue → purple gradient | 200×200 |
| `icon.svg` | Square icon optimized for small sizes & favicons | 64×64 |
| `logo-banner.svg` | Horizontal layout: icon + "HELIX" text | 400×100 |
| `helix-banner-transparent.svg` | Banner with helix icon, "HELIX" text & "OS" badge — transparent background | 680×130 |
| `social-preview.svg` | Social/OpenGraph preview card | — |

### Subsystem Logos

| File | Description |
|------|-------------|
| `nexus-logo.svg` | NEXUS subsystem logo |
| `lumina-logo.svg` | Lumina subsystem logo |
| `lumina-icon.svg` | Lumina subsystem icon |

---

## Colors

| Role | Hex | Preview |
|------|-----|---------|
| Primary Blue | `#4A90E2` | 🔵 |
| Mid Purple | `#7B68EE` | 🟣 |
| Accent Purple | `#9B59B6` | 💜 |

All main assets use a horizontal gradient: `#4A90E2` → `#7B68EE` → `#9B59B6`.

---

## Usage

### In a README

```markdown
<div align="center">
  <img src="assets/logo.svg" width="200" alt="Helix OS">
</div>
```

### Banner (transparent, for dark backgrounds)

```markdown
<div align="center">
  <img src="assets/helix-banner-transparent.svg" width="560" alt="Helix OS">
</div>
```

### As favicon

```html
<link rel="icon" type="image/svg+xml" href="assets/icon.svg">
```

---

## PNG Conversion

All SVGs are vector — convert to PNG at any resolution without quality loss:

```bash
# rsvg-convert (recommended)
rsvg-convert -w 512 -h 512 logo.svg -o logo-512.png

# Inkscape
inkscape logo.svg -w 512 -h 512 -o logo-512.png

# ImageMagick
convert -background none logo.svg -resize 512x512 logo-512.png
```

---

## Recommended Sizes

| Usage | Format | Size |
|-------|--------|------|
| README header | SVG | original |
| Favicon | SVG / ICO | 32×32 |
| App / profile icon | PNG | 512×512 |
| Social preview | PNG | 1280×640 |
| Print | SVG / PDF | vector |

---

## Design

The logo is a stylized **double helix** (two reverse-S curves connected by horizontal rungs) representing:

- 🧬 **DNA** — modular, trait-based architecture
- 🔄 **Evolution** — self-healing, hot-swappable modules
- 🔗 **Connections** — interoperability between subsystems
- 🎯 **Minimalism** — clean, recognizable at any scale
