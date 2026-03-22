# Zelda GBA Quality Analysis — Can PAX Do It?

## Verdict

Yes. Environment tilesets (90% of visual work) are comfortably in PAX's
wheelhouse. Character animation is the honest weak spot — plan 2–3 weeks of
human artist time on 20 canonical poses. Total asset production: 6–12 months
solo artist → 6–8 weeks with PAX + 1 artist QC.

## GBA Hardware vs PAX

| Spec | GBA Value | PAX Status |
|------|-----------|------------|
| Tile resolution | 8×8 hw (16×16 logical) | Native |
| Colors per sprite | 16 from a 16-color palette | Palette system + max_palette_size |
| On-screen palettes | 16 × 16 = 256 colors | Multi-palette via themes |
| Unique tiles (VRAM) | 1024 × 8×8 max | WFC atlas handles |
| BG layers | 4 (Mode 0) | Tilemap layers |
| Minish Cap tiles | ~800–1200 unique | Volume challenge |
| Link frames | ~300 across all anims | Human touch needed |

## Four Blockers and Their Fixes

1. **Palette discipline** (easy) — `max_palette_size = 16` in theme. Hard error.
2. **Light source** (medium) — `light_source = "top-left"` in theme. 4×4 reference quad in prompts.
3. **Character animation** (medium) — 20 canonical poses, rest via delta + mirror.
4. **World variation** (hard) — Theme inheritance: `extends` locks shared assets.
