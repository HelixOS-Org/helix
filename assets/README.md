# Helix OS - Logo Assets

## Fichiers disponibles

### Logos principaux
- **`logo.svg`** - Logo principal avec dégradé bleu/violet
- **`logo-alt.svg`** - Version alternative avec spirale géométrique
- **`logo-mono.svg`** - Version monochrome (noir pur)

### Icons
- **`icon.svg`** - Icône carrée 64x64 optimisée pour app icons
- **`favicon.svg`** - À utiliser comme favicon

### Banners
- **`logo-banner.svg`** - Logo horizontal avec texte pour README

## Utilisation

### Dans le README
```markdown
<div align="center">
  <img src="assets/logo.svg" width="200" alt="Helix OS Logo">
</div>
```

### Comme favicon
```html
<link rel="icon" type="image/svg+xml" href="assets/icon.svg">
```

### En PNG (conversion)
Pour convertir en PNG avec transparence :
```bash
# Avec Inkscape
inkscape logo.svg -w 512 -h 512 -o logo-512.png

# Avec ImageMagick
convert -background none logo.svg -resize 512x512 logo-512.png

# Avec rsvg-convert
rsvg-convert -w 512 -h 512 logo.svg -o logo-512.png
```

## Variantes de couleurs

### Couleurs du dégradé
- Bleu primaire: `#4A90E2`
- Violet secondaire: `#7B68EE`

### Versions monochrome
- Noir: `#000000`
- Blanc: `#FFFFFF` (pour fonds sombres)

## Formats recommandés par usage

| Usage | Format | Taille |
|-------|--------|--------|
| README.md | SVG | 200x200 |
| Favicon | SVG ou ICO | 32x32 |
| App Icon | PNG | 512x512 |
| Social Media | PNG | 1200x630 |
| Print | SVG ou PDF | Vectoriel |

## Design Concept

Le logo représente une **double hélice** stylisée symbolisant :
- 🧬 **ADN/Code génétique** - Architecture modulaire
- 🔄 **Spirale** - Évolution continue, cycles
- 🔗 **Connexions** - Interopérabilité, communication
- ⚡ **Circuit** - Technologie, innovation
- 🎯 **Minimalisme** - Clarté, efficacité

Forme ultra-épurée pour une reconnaissance instantanée à toute échelle.
