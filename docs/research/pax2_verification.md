# PAX 2.0 Technical Claims — Verification Results

Date: 2026-03-22

## 1. FNV-1a Hashing

- `std::hash::DefaultHasher` uses **SipHash 1-3**, NOT FNV-1a
- FNV-1a requires the `fnv` crate (v1.0.7, 504M+ downloads)
- Use `fnv` for edge hashing — fast, deterministic, no crypto dep
- Sources: [Rust docs](https://doc.rust-lang.org/std/hash/struct.DefaultHasher.html), [fnv on crates.io](https://crates.io/crates/fnv)

## 2. fixedbitset Version

- 0.5.x series is real. Latest: 0.5.7
- Compiled successfully in our workspace
- Source: [crates.io](https://crates.io/crates/fixedbitset)

## 3. SpritePalettizer / Palette LUT Shader

- Real, well-documented technique
- SpritePalettizer by NeZvers: [itch.io](https://nezvers.itch.io/spritepalettizer)
- Used in Godot, GameMaker, Unity
- Grayscale index texture + palette LUT row texture
- Source: [Envato tutorial](https://gamedevelopment.tutsplus.com/tutorials/how-to-use-a-shader-to-dynamically-swap-a-sprites-colors--cms-25129)

## 4. Dual-Grid Autotiling

- Conceptualized by Oskar Stalberg
- 5 tile *types*, 15 actual tiles (6 if symmetric)
- TileMapDual for Godot: [GitHub](https://github.com/pablogila/TileMapDual)
- Source: [Boris the Brave](https://www.boristhebrave.com/2021/09/12/beyond-basic-autotiling/)

## 5. Blob Tileset Reference

- Canonical URL: `cr31.co.uk` (NOT `.com`)
- Page: https://www.cr31.co.uk/stagecast/wang/blob.html
- Mirror: https://www.boristhebrave.com/permanent/24/06/cr31/stagecast/wang/blob.html
- Classification: https://www.boristhebrave.com/2021/11/14/classification-of-tilesets/

## 6. TOML Array-of-Tables Nesting

- `[[spriteset.hero.sprite]]` is valid TOML v1.0.0
- Inline frame arrays inside `[[sprite]]` blocks are valid
- Source: [TOML spec](https://toml.io/en/v1.0.0)

## 7. TexturePacker JSON Hash `animationTags`

- **WRONG** — TexturePacker does NOT have `animationTags`
- That's Aseprite's `frameTags` in its JSON export format
- TexturePacker meta section: app, version, image, format, size, scale, smartupdate
- PAX should export animation tags in Aseprite-compatible format separately

## 8. rectangle-pack Crate

- Exists: [crates.io](https://crates.io/crates/rectangle-pack)
- Latest: 0.4.2 (2021-05-03)
- Not actively maintained but functional
