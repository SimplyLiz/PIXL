# Skeletal Animation Research — V2 Feature

## Key Sources

**Smack Studio** — generates depth maps for body parts, compute shader redraws
sprites to match bone configurations. No 3D model needed. UE4 integration.
Source: Smack Studio documentation.

**RotSprite** — rotation algorithm for pixel art that produces far fewer
artifacts than nearest-neighbor. Scale 8x, modified Scale2x, rotate, scale
back to 1x. No new colors introduced.

**Coutinho & Chaimowicz 2024** — pose generation as missing data imputation.
Generate sprite in target pose given all available sprites in other poses.

## Architecture for PAX

Three-layer system:
1. Body part sprites + depth hints (authored, small, LLM-reliable)
2. Skeleton with RotSprite rotation (bone hierarchy, joint constraints)
3. Keyframe poses + interpolation (lerp transforms, snap to integer pixels)

## Authoring Reduction

- 6-8 body part sprites (each within LLM accuracy zone)
- 3-4 skeletal keyframe poses (coordinate values, no spatial reasoning)
- System generates: 4-directional movement, walk cycles, attack combos
- Human touch: squash/stretch on ~10 key frames (~10-15 hours)

## Status: V2 feature, format specified in PAX 2.0 spec Section 19.1
