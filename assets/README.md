# Helix OS - Logo Assets

## Available Files

### Main Logos
- **`logo.svg`** - Primary logo with blue/purple gradient
- **`logo-alt.svg`** - Alternative version with geometric spiral
- **`logo-mono.svg`** - Monochrome version (pure black)

### Icons
- **`icon.svg`** - 64x64 square icon optimized for app icons
- **`favicon.svg`** - To be used as favicon

### Banners
- **`logo-banner.svg`** - Horizontal logo with text for README

## Usage

### In the README
```markdown
<div align="center">
  <img src="assets/logo.svg" width="200" alt="Helix OS Logo">
</div>
```

### As favicon
```html
<link rel="icon" type="image/svg+xml" href="assets/icon.svg">
```

### PNG Conversion
To convert to PNG with transparency:
```bash
# With Inkscape
inkscape logo.svg -w 512 -h 512 -o logo-512.png

# With ImageMagick
convert -background none logo.svg -resize 512x512 logo-512.png

# With rsvg-convert
rsvg-convert -w 512 -h 512 logo.svg -o logo-512.png
```

## Color Variants

### Gradient Colors
- Primary Blue: `#4A90E2`
- Secondary Purple: `#7B68EE`

### Monochrome Versions
- Black: `#000000`
- White: `#FFFFFF` (for dark backgrounds)

## Recommended Formats by Usage

| Usage | Format | Size |
|-------|--------|------|
| README.md | SVG | 200x200 |
| Favicon | SVG or ICO | 32x32 |
| App Icon | PNG | 512x512 |
| Social Media | PNG | 1200x630 |
| Print | SVG or PDF | Vector |

## Design Concept

The logo represents a stylized **double helix** symbolizing:
- 🧬 **DNA/Genetic Code** - Modular architecture
- 🔄 **Spiral** - Continuous evolution, cycles
- 🔗 **Connections** - Interoperability, communication
- ⚡ **Circuit** - Technology, innovation
- 🎯 **Minimalism** - Clarity, efficiency

Ultra-clean form for instant recognition at any scale.
