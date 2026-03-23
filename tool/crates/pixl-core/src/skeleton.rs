/// Skeletal animation system.
///
/// Author body parts as small sprites, define a skeleton with bones,
/// set keyframe poses, and the system composites rotated body parts
/// into complete animation frames.
///
/// Uses RotSprite algorithm: scale 8x, rotate, scale back — no new colors.

use crate::types::{Palette, Rgba};
use std::collections::HashMap;

// ── Types ───────────────────────────────────────────────

/// A body part — a small sprite attached to a bone.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BodyPart {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub grid: Vec<Vec<char>>,
    pub pivot: (u32, u32),         // attachment point within the part
    pub depth: i32,                // z-order for compositing
    pub symmetric: bool,           // mirror for opposite side
}

/// A bone in the skeleton hierarchy.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Bone {
    pub name: String,
    pub parent: Option<String>,
    pub offset: (i32, i32),        // offset from parent bone
    pub part: String,              // which body part to render
    pub rotation_range: Option<(f32, f32)>, // min/max degrees
}

/// A skeleton definition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Skeleton {
    pub name: String,
    pub canvas: (u32, u32),        // output sprite size
    pub bones: Vec<Bone>,
}

/// A keyframe pose — rotation angles for each bone.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Keyframe {
    pub frame: u32,
    pub duration_ms: Option<u32>,
    pub bone_rotations: HashMap<String, f32>,  // bone_name -> degrees
    pub bone_offsets: HashMap<String, (i32, i32)>, // bone_name -> (dx, dy)
    pub mirror: Option<String>,    // "horizontal" to mirror the frame
}

/// A skeletal animation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkeletalAnimation {
    pub name: String,
    pub fps: u32,
    pub looping: bool,
    pub keyframes: Vec<Keyframe>,
}

// ── RotSprite ───────────────────────────────────────────

/// Rotate a pixel grid by `degrees` using the RotSprite algorithm.
/// Scale up 8x, rotate with nearest-neighbor, scale back down.
/// Preserves the indexed color palette — no new colors introduced.
pub fn rotsprite_rotate(
    grid: &[Vec<char>],
    degrees: f32,
    void_sym: char,
) -> Vec<Vec<char>> {
    if degrees.abs() < 0.5 {
        return grid.to_vec();
    }

    let h = grid.len();
    let w = if h > 0 { grid[0].len() } else { return vec![] };

    // Scale up 8x using nearest-neighbor
    let scale = 8usize;
    let big_w = w * scale;
    let big_h = h * scale;
    let mut big = vec![vec![void_sym; big_w]; big_h];
    for y in 0..h {
        for x in 0..w {
            for sy in 0..scale {
                for sx in 0..scale {
                    big[y * scale + sy][x * scale + sx] = grid[y][x];
                }
            }
        }
    }

    // Rotate the upscaled grid
    let rad = degrees * std::f32::consts::PI / 180.0;
    let cos_a = rad.cos();
    let sin_a = rad.sin();
    let cx = big_w as f32 / 2.0;
    let cy = big_h as f32 / 2.0;

    let mut rotated = vec![vec![void_sym; big_w]; big_h];
    for y in 0..big_h {
        for x in 0..big_w {
            // Inverse rotation to find source pixel
            let fx = x as f32 - cx;
            let fy = y as f32 - cy;
            let src_x = (fx * cos_a + fy * sin_a + cx).round() as i32;
            let src_y = (-fx * sin_a + fy * cos_a + cy).round() as i32;

            if src_x >= 0 && src_y >= 0 && (src_x as usize) < big_w && (src_y as usize) < big_h {
                rotated[y][x] = big[src_y as usize][src_x as usize];
            }
        }
    }

    // Scale back down by sampling center of each 8x8 block
    let mut result = vec![vec![void_sym; w]; h];
    for y in 0..h {
        for x in 0..w {
            let sample_x = x * scale + scale / 2;
            let sample_y = y * scale + scale / 2;
            if sample_x < big_w && sample_y < big_h {
                result[y][x] = rotated[sample_y][sample_x];
            }
        }
    }

    result
}

// ── Skeleton Compositing ────────────────────────────────

/// Composite a skeleton at a given pose into a pixel grid.
pub fn composite_pose(
    skeleton: &Skeleton,
    parts: &HashMap<String, BodyPart>,
    keyframe: &Keyframe,
    void_sym: char,
) -> Vec<Vec<char>> {
    let (cw, ch) = skeleton.canvas;
    let mut canvas = vec![vec![void_sym; cw as usize]; ch as usize];

    // Build bone hierarchy
    let bone_map: HashMap<&str, &Bone> = skeleton.bones.iter()
        .map(|b| (b.name.as_str(), b))
        .collect();

    // Compute world positions for each bone (parent-relative chain)
    let mut world_positions: HashMap<&str, (i32, i32)> = HashMap::new();
    let mut world_rotations: HashMap<&str, f32> = HashMap::new();

    // Sort bones by dependency (parents before children)
    let sorted = topological_sort(&skeleton.bones);

    for bone_name in &sorted {
        let bone = bone_map[bone_name.as_str()];
        let parent_pos = bone.parent.as_ref()
            .and_then(|p| world_positions.get(p.as_str()))
            .copied()
            .unwrap_or((cw as i32 / 2, ch as i32 / 2)); // root at center

        let parent_rot = bone.parent.as_ref()
            .and_then(|p| world_rotations.get(p.as_str()))
            .copied()
            .unwrap_or(0.0);

        let extra_offset = keyframe.bone_offsets
            .get(bone_name.as_str())
            .copied()
            .unwrap_or((0, 0));

        let local_rot = keyframe.bone_rotations
            .get(bone_name.as_str())
            .copied()
            .unwrap_or(0.0);

        let world_rot = parent_rot + local_rot;

        // Rotate offset by parent rotation
        let rad = parent_rot * std::f32::consts::PI / 180.0;
        let ox = bone.offset.0 as f32;
        let oy = bone.offset.1 as f32;
        let rx = (ox * rad.cos() - oy * rad.sin()).round() as i32;
        let ry = (ox * rad.sin() + oy * rad.cos()).round() as i32;

        let wx = parent_pos.0 + rx + extra_offset.0;
        let wy = parent_pos.1 + ry + extra_offset.1;

        world_positions.insert(bone_name.as_str(), (wx, wy));
        world_rotations.insert(bone_name.as_str(), world_rot);
    }

    // Render body parts sorted by depth
    let mut render_order: Vec<(&str, i32)> = skeleton.bones.iter()
        .filter_map(|b| {
            parts.get(&b.part).map(|p| (b.name.as_str(), p.depth))
        })
        .collect();
    render_order.sort_by_key(|(_, d)| *d);

    for (bone_name, _) in &render_order {
        let bone = bone_map[*bone_name];
        let Some(part) = parts.get(&bone.part) else { continue };
        let Some(&(wx, wy)) = world_positions.get(*bone_name) else { continue };
        let rotation = world_rotations.get(*bone_name).copied().unwrap_or(0.0);

        // Rotate the body part grid
        let rotated = rotsprite_rotate(&part.grid, rotation, void_sym);

        // Blit onto canvas
        let rh = rotated.len() as i32;
        let rw = if rh > 0 { rotated[0].len() as i32 } else { 0 };
        let px = wx - part.pivot.0 as i32;
        let py = wy - part.pivot.1 as i32;

        for ry in 0..rh {
            for rx in 0..rw {
                let cx = px + rx;
                let cy = py + ry;
                if cx >= 0 && cy >= 0 && (cx as usize) < cw as usize && (cy as usize) < ch as usize {
                    let sym = rotated[ry as usize][rx as usize];
                    if sym != void_sym {
                        canvas[cy as usize][cx as usize] = sym;
                    }
                }
            }
        }
    }

    // Apply mirror if requested
    if keyframe.mirror.as_deref() == Some("horizontal") {
        for row in &mut canvas {
            row.reverse();
        }
    }

    canvas
}

/// Interpolate between two keyframes.
pub fn interpolate_keyframes(a: &Keyframe, b: &Keyframe, t: f32) -> Keyframe {
    let mut rotations = HashMap::new();
    let mut offsets = HashMap::new();

    // Collect all bone names from both keyframes
    let all_bones: std::collections::HashSet<&str> = a.bone_rotations.keys()
        .chain(b.bone_rotations.keys())
        .map(|s| s.as_str())
        .collect();

    for bone in all_bones {
        let ra = a.bone_rotations.get(bone).copied().unwrap_or(0.0);
        let rb = b.bone_rotations.get(bone).copied().unwrap_or(0.0);
        rotations.insert(bone.to_string(), lerp(ra, rb, t));

        let oa = a.bone_offsets.get(bone).copied().unwrap_or((0, 0));
        let ob = b.bone_offsets.get(bone).copied().unwrap_or((0, 0));
        offsets.insert(bone.to_string(), (
            lerp(oa.0 as f32, ob.0 as f32, t).round() as i32,
            lerp(oa.1 as f32, ob.1 as f32, t).round() as i32,
        ));
    }

    Keyframe {
        frame: lerp(a.frame as f32, b.frame as f32, t).round() as u32,
        duration_ms: a.duration_ms,
        bone_rotations: rotations,
        bone_offsets: offsets,
        mirror: None,
    }
}

/// Generate all frames for a skeletal animation.
pub fn generate_animation_frames(
    skeleton: &Skeleton,
    parts: &HashMap<String, BodyPart>,
    animation: &SkeletalAnimation,
    void_sym: char,
) -> Vec<(Vec<Vec<char>>, u32)> {
    let mut frames = Vec::new();

    if animation.keyframes.is_empty() {
        return frames;
    }

    if animation.keyframes.len() == 1 {
        let grid = composite_pose(skeleton, parts, &animation.keyframes[0], void_sym);
        let dur = animation.keyframes[0].duration_ms.unwrap_or(1000 / animation.fps.max(1));
        frames.push((grid, dur));
        return frames;
    }

    // Generate frames between keyframes
    let frame_duration = 1000 / animation.fps.max(1);

    for i in 0..animation.keyframes.len() {
        let kf = &animation.keyframes[i];
        let next = if i + 1 < animation.keyframes.len() {
            &animation.keyframes[i + 1]
        } else if animation.looping {
            &animation.keyframes[0]
        } else {
            // Last frame — just render it
            let grid = composite_pose(skeleton, parts, kf, void_sym);
            let dur = kf.duration_ms.unwrap_or(frame_duration);
            frames.push((grid, dur));
            break;
        };

        // How many intermediate frames between this keyframe and next
        let frame_span = if next.frame > kf.frame {
            next.frame - kf.frame
        } else {
            1
        };

        for f in 0..frame_span {
            let t = f as f32 / frame_span as f32;
            let interpolated = interpolate_keyframes(kf, next, t);
            let grid = composite_pose(skeleton, parts, &interpolated, void_sym);
            let dur = kf.duration_ms.unwrap_or(frame_duration);
            frames.push((grid, dur));
        }
    }

    frames
}

// ── Helpers ─────────────────────────────────────────────

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn topological_sort(bones: &[Bone]) -> Vec<String> {
    let mut sorted = Vec::new();
    let mut visited = std::collections::HashSet::new();
    let bone_map: HashMap<&str, &Bone> = bones.iter().map(|b| (b.name.as_str(), b)).collect();

    fn visit(
        name: &str,
        bone_map: &HashMap<&str, &Bone>,
        visited: &mut std::collections::HashSet<String>,
        sorted: &mut Vec<String>,
    ) {
        if visited.contains(name) { return; }
        visited.insert(name.to_string());
        if let Some(bone) = bone_map.get(name) {
            if let Some(ref parent) = bone.parent {
                visit(parent, bone_map, visited, sorted);
            }
        }
        sorted.push(name.to_string());
    }

    for bone in bones {
        visit(&bone.name, &bone_map, &mut visited, &mut sorted);
    }
    sorted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head_part() -> BodyPart {
        BodyPart {
            name: "head".to_string(),
            width: 4,
            height: 4,
            grid: vec![
                vec!['.', '#', '#', '.'],
                vec!['#', '+', '+', '#'],
                vec!['#', '+', '+', '#'],
                vec!['.', '#', '#', '.'],
            ],
            pivot: (2, 3),
            depth: 0,
            symmetric: false,
        }
    }

    fn torso_part() -> BodyPart {
        BodyPart {
            name: "torso".to_string(),
            width: 4,
            height: 6,
            grid: vec![
                vec!['.', '+', '+', '.'],
                vec!['+', '+', '+', '+'],
                vec!['+', '+', '+', '+'],
                vec!['+', '+', '+', '+'],
                vec!['+', '+', '+', '+'],
                vec!['.', '+', '+', '.'],
            ],
            pivot: (2, 0),
            depth: 1,
            symmetric: false,
        }
    }

    fn test_skeleton() -> Skeleton {
        Skeleton {
            name: "hero".to_string(),
            canvas: (16, 24),
            bones: vec![
                Bone {
                    name: "root".to_string(),
                    parent: None,
                    offset: (0, 0),
                    part: "torso".to_string(),
                    rotation_range: None,
                },
                Bone {
                    name: "head".to_string(),
                    parent: Some("root".to_string()),
                    offset: (0, -6),
                    part: "head".to_string(),
                    rotation_range: Some((-15.0, 15.0)),
                },
            ],
        }
    }

    #[test]
    fn rotsprite_identity() {
        let grid = vec![
            vec!['#', '+'],
            vec!['+', '#'],
        ];
        let result = rotsprite_rotate(&grid, 0.0, '.');
        assert_eq!(result, grid);
    }

    #[test]
    fn rotsprite_preserves_size() {
        let grid = vec![
            vec!['#', '#', '#', '#'],
            vec!['#', '+', '+', '#'],
            vec!['#', '+', '+', '#'],
            vec!['#', '#', '#', '#'],
        ];
        let rotated = rotsprite_rotate(&grid, 45.0, '.');
        assert_eq!(rotated.len(), 4);
        assert_eq!(rotated[0].len(), 4);
    }

    #[test]
    fn rotsprite_no_new_colors() {
        let grid = vec![
            vec!['#', '+', '.'],
            vec!['+', '#', '+'],
            vec!['.', '+', '#'],
        ];
        let rotated = rotsprite_rotate(&grid, 30.0, '.');
        let valid = ['#', '+', '.'];
        for row in &rotated {
            for &ch in row {
                assert!(valid.contains(&ch), "unexpected char: {}", ch);
            }
        }
    }

    #[test]
    fn composite_rest_pose() {
        let skeleton = test_skeleton();
        let mut parts = HashMap::new();
        parts.insert("head".to_string(), head_part());
        parts.insert("torso".to_string(), torso_part());

        let kf = Keyframe {
            frame: 0,
            duration_ms: None,
            bone_rotations: HashMap::new(),
            bone_offsets: HashMap::new(),
            mirror: None,
        };

        let canvas = composite_pose(&skeleton, &parts, &kf, '.');
        assert_eq!(canvas.len(), 24);
        assert_eq!(canvas[0].len(), 16);

        // Should have some non-void pixels
        let non_void: usize = canvas.iter()
            .flat_map(|r| r.iter())
            .filter(|&&c| c != '.')
            .count();
        assert!(non_void > 0, "composited frame should have pixels");
    }

    #[test]
    fn interpolate_midpoint() {
        let a = Keyframe {
            frame: 0,
            duration_ms: None,
            bone_rotations: [("arm".to_string(), -30.0)].into(),
            bone_offsets: HashMap::new(),
            mirror: None,
        };
        let b = Keyframe {
            frame: 4,
            duration_ms: None,
            bone_rotations: [("arm".to_string(), 30.0)].into(),
            bone_offsets: HashMap::new(),
            mirror: None,
        };

        let mid = interpolate_keyframes(&a, &b, 0.5);
        let arm_rot = mid.bone_rotations["arm"];
        assert!((arm_rot - 0.0).abs() < 0.01, "midpoint should be ~0, got {}", arm_rot);
    }

    #[test]
    fn generate_frames_produces_output() {
        let skeleton = test_skeleton();
        let mut parts = HashMap::new();
        parts.insert("head".to_string(), head_part());
        parts.insert("torso".to_string(), torso_part());

        let anim = SkeletalAnimation {
            name: "idle".to_string(),
            fps: 4,
            looping: true,
            keyframes: vec![
                Keyframe {
                    frame: 0, duration_ms: None,
                    bone_rotations: [("head".to_string(), 0.0)].into(),
                    bone_offsets: HashMap::new(), mirror: None,
                },
                Keyframe {
                    frame: 2, duration_ms: None,
                    bone_rotations: [("head".to_string(), 5.0)].into(),
                    bone_offsets: [("root".to_string(), (0, -1))].into(), mirror: None,
                },
            ],
        };

        let frames = generate_animation_frames(&skeleton, &parts, &anim, '.');
        assert!(!frames.is_empty(), "should produce at least 1 frame");
        // Each frame should be the right canvas size
        for (grid, dur) in &frames {
            assert_eq!(grid.len(), 24);
            assert_eq!(grid[0].len(), 16);
            assert!(*dur > 0);
        }
    }

    #[test]
    fn topological_sort_order() {
        let bones = vec![
            Bone { name: "head".to_string(), parent: Some("root".to_string()), offset: (0, 0), part: "head".to_string(), rotation_range: None },
            Bone { name: "root".to_string(), parent: None, offset: (0, 0), part: "torso".to_string(), rotation_range: None },
            Bone { name: "arm".to_string(), parent: Some("root".to_string()), offset: (0, 0), part: "arm".to_string(), rotation_range: None },
        ];
        let sorted = topological_sort(&bones);
        let root_idx = sorted.iter().position(|s| s == "root").unwrap();
        let head_idx = sorted.iter().position(|s| s == "head").unwrap();
        let arm_idx = sorted.iter().position(|s| s == "arm").unwrap();
        assert!(root_idx < head_idx);
        assert!(root_idx < arm_idx);
    }
}
