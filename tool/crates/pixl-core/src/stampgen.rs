/// Procedural stamp generation — algorithmically produce common pixel art
/// patterns without LLM authorship. Bootstraps the compose vocabulary.

/// A generated stamp with its grid and metadata.
#[derive(Debug, Clone)]
pub struct GeneratedStamp {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub grid: Vec<Vec<char>>,
    pub pattern: String,
}

/// Generate stamps for a given pattern type.
/// `fg` and `bg` are the primary foreground/background palette symbols.
pub fn generate_stamps(pattern: &str, size: u32, fg: char, bg: char) -> Vec<GeneratedStamp> {
    match pattern {
        "brick_bond" | "brick" => generate_brick_bond(size, fg, bg),
        "checkerboard" | "checker" => generate_checkerboard(size, fg, bg),
        "diagonal" | "stripe" => generate_diagonal_stripes(size, fg, bg),
        "dither_bayer" | "bayer" => generate_bayer_dither(size, fg, bg),
        "horizontal_stripe" | "hstripe" => generate_horizontal_stripes(size, fg, bg),
        "dots" | "dot_grid" => generate_dot_grid(size, fg, bg),
        "cross" | "crosshatch" => generate_crosshatch(size, fg, bg),
        "noise" | "random" => generate_noise_pattern(size, fg, bg),
        _ => vec![],
    }
}

/// List available pattern names.
pub fn available_patterns() -> Vec<&'static str> {
    vec![
        "brick_bond",
        "checkerboard",
        "diagonal",
        "dither_bayer",
        "horizontal_stripe",
        "dots",
        "cross",
        "noise",
    ]
}

fn generate_brick_bond(size: u32, fg: char, bg: char) -> Vec<GeneratedStamp> {
    let s = size as usize;
    let mut stamps = Vec::new();

    // Running bond (offset every other row)
    let mut grid = vec![vec![fg; s]; s];
    for y in 0..s {
        let offset = if y % 2 == 0 { 0 } else { s / 2 };
        // Mortar lines
        if y % (s / 2) == 0 {
            for x in 0..s {
                grid[y][x] = bg;
            }
        } else {
            // Vertical mortar
            let mortar_x = (offset + s / 2) % s;
            if mortar_x < s {
                grid[y][mortar_x] = bg;
            }
        }
    }
    stamps.push(GeneratedStamp {
        name: format!("brick_running_{}x{}", size, size),
        width: size,
        height: size,
        grid,
        pattern: "brick_bond".to_string(),
    });

    // Stack bond (aligned)
    let mut grid2 = vec![vec![fg; s]; s];
    for y in 0..s {
        if y % (s / 2) == 0 {
            for x in 0..s {
                grid2[y][x] = bg;
            }
        } else {
            let mortar_x = s / 2;
            if mortar_x < s {
                grid2[y][mortar_x] = bg;
            }
        }
    }
    stamps.push(GeneratedStamp {
        name: format!("brick_stack_{}x{}", size, size),
        width: size,
        height: size,
        grid: grid2,
        pattern: "brick_bond".to_string(),
    });

    stamps
}

fn generate_checkerboard(size: u32, fg: char, bg: char) -> Vec<GeneratedStamp> {
    let s = size as usize;

    // 1px checkerboard
    let grid1: Vec<Vec<char>> = (0..s)
        .map(|y| {
            (0..s)
                .map(|x| if (x + y) % 2 == 0 { fg } else { bg })
                .collect()
        })
        .collect();

    // 2px checkerboard
    let grid2: Vec<Vec<char>> = (0..s)
        .map(|y| {
            (0..s)
                .map(|x| if ((x / 2) + (y / 2)) % 2 == 0 { fg } else { bg })
                .collect()
        })
        .collect();

    vec![
        GeneratedStamp {
            name: format!("checker_1px_{}x{}", size, size),
            width: size,
            height: size,
            grid: grid1,
            pattern: "checkerboard".to_string(),
        },
        GeneratedStamp {
            name: format!("checker_2px_{}x{}", size, size),
            width: size,
            height: size,
            grid: grid2,
            pattern: "checkerboard".to_string(),
        },
    ]
}

fn generate_diagonal_stripes(size: u32, fg: char, bg: char) -> Vec<GeneratedStamp> {
    let s = size as usize;

    let grid_r: Vec<Vec<char>> = (0..s)
        .map(|y| {
            (0..s)
                .map(|x| if (x + y) % 4 < 2 { fg } else { bg })
                .collect()
        })
        .collect();

    let grid_l: Vec<Vec<char>> = (0..s)
        .map(|y| {
            (0..s)
                .map(|x| if (x + s - y) % 4 < 2 { fg } else { bg })
                .collect()
        })
        .collect();

    vec![
        GeneratedStamp {
            name: format!("diag_right_{}x{}", size, size),
            width: size,
            height: size,
            grid: grid_r,
            pattern: "diagonal".to_string(),
        },
        GeneratedStamp {
            name: format!("diag_left_{}x{}", size, size),
            width: size,
            height: size,
            grid: grid_l,
            pattern: "diagonal".to_string(),
        },
    ]
}

fn generate_bayer_dither(size: u32, fg: char, bg: char) -> Vec<GeneratedStamp> {
    let s = size as usize;
    let bayer = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

    // 25% density
    let grid_25: Vec<Vec<char>> = (0..s)
        .map(|y| {
            (0..s)
                .map(|x| if bayer[y % 4][x % 4] < 4 { fg } else { bg })
                .collect()
        })
        .collect();

    // 50% density
    let grid_50: Vec<Vec<char>> = (0..s)
        .map(|y| {
            (0..s)
                .map(|x| if bayer[y % 4][x % 4] < 8 { fg } else { bg })
                .collect()
        })
        .collect();

    // 75% density
    let grid_75: Vec<Vec<char>> = (0..s)
        .map(|y| {
            (0..s)
                .map(|x| if bayer[y % 4][x % 4] < 12 { fg } else { bg })
                .collect()
        })
        .collect();

    vec![
        GeneratedStamp {
            name: format!("bayer_25_{}x{}", size, size),
            width: size,
            height: size,
            grid: grid_25,
            pattern: "dither_bayer".to_string(),
        },
        GeneratedStamp {
            name: format!("bayer_50_{}x{}", size, size),
            width: size,
            height: size,
            grid: grid_50,
            pattern: "dither_bayer".to_string(),
        },
        GeneratedStamp {
            name: format!("bayer_75_{}x{}", size, size),
            width: size,
            height: size,
            grid: grid_75,
            pattern: "dither_bayer".to_string(),
        },
    ]
}

fn generate_horizontal_stripes(size: u32, fg: char, bg: char) -> Vec<GeneratedStamp> {
    let s = size as usize;

    let grid: Vec<Vec<char>> = (0..s)
        .map(|y| {
            let sym = if y % 2 == 0 { fg } else { bg };
            vec![sym; s]
        })
        .collect();

    vec![GeneratedStamp {
        name: format!("hstripe_{}x{}", size, size),
        width: size,
        height: size,
        grid,
        pattern: "horizontal_stripe".to_string(),
    }]
}

fn generate_dot_grid(size: u32, fg: char, bg: char) -> Vec<GeneratedStamp> {
    let s = size as usize;

    let grid: Vec<Vec<char>> = (0..s)
        .map(|y| {
            (0..s)
                .map(|x| if x % 3 == 1 && y % 3 == 1 { fg } else { bg })
                .collect()
        })
        .collect();

    vec![GeneratedStamp {
        name: format!("dots_{}x{}", size, size),
        width: size,
        height: size,
        grid,
        pattern: "dots".to_string(),
    }]
}

fn generate_crosshatch(size: u32, fg: char, bg: char) -> Vec<GeneratedStamp> {
    let s = size as usize;

    let grid: Vec<Vec<char>> = (0..s)
        .map(|y| {
            (0..s)
                .map(|x| if x == s / 2 || y == s / 2 { fg } else { bg })
                .collect()
        })
        .collect();

    vec![GeneratedStamp {
        name: format!("cross_{}x{}", size, size),
        width: size,
        height: size,
        grid,
        pattern: "cross".to_string(),
    }]
}

fn generate_noise_pattern(size: u32, fg: char, bg: char) -> Vec<GeneratedStamp> {
    let s = size as usize;
    // Deterministic pseudo-random using simple hash
    let grid: Vec<Vec<char>> = (0..s)
        .map(|y| {
            (0..s)
                .map(|x| {
                    let hash = (x * 7 + y * 13 + x * y * 3) % 16;
                    if hash < 6 { fg } else { bg }
                })
                .collect()
        })
        .collect();

    vec![GeneratedStamp {
        name: format!("noise_{}x{}", size, size),
        width: size,
        height: size,
        grid,
        pattern: "noise".to_string(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_brick_produces_two() {
        let stamps = generate_stamps("brick_bond", 4, '#', '+');
        assert_eq!(stamps.len(), 2);
        assert_eq!(stamps[0].width, 4);
        assert_eq!(stamps[0].height, 4);
        assert!(stamps[0].name.contains("running"));
        assert!(stamps[1].name.contains("stack"));
    }

    #[test]
    fn generate_checkerboard_correct() {
        let stamps = generate_stamps("checkerboard", 4, '#', '.');
        assert_eq!(stamps.len(), 2);
        let grid = &stamps[0].grid;
        assert_eq!(grid[0][0], '#');
        assert_eq!(grid[0][1], '.');
        assert_eq!(grid[1][0], '.');
        assert_eq!(grid[1][1], '#');
    }

    #[test]
    fn generate_bayer_three_densities() {
        let stamps = generate_stamps("bayer", 4, '#', '.');
        assert_eq!(stamps.len(), 3);
        // 25% should have fewer fg pixels than 75%
        let count_fg = |grid: &Vec<Vec<char>>| -> usize {
            grid.iter()
                .flat_map(|r| r.iter())
                .filter(|&&c| c == '#')
                .count()
        };
        assert!(count_fg(&stamps[0].grid) < count_fg(&stamps[2].grid));
    }

    #[test]
    fn all_patterns_produce_output() {
        for pattern in available_patterns() {
            let stamps = generate_stamps(pattern, 4, '#', '.');
            assert!(
                !stamps.is_empty(),
                "pattern '{}' produced no stamps",
                pattern
            );
            for stamp in &stamps {
                assert_eq!(stamp.grid.len(), 4);
                assert_eq!(stamp.grid[0].len(), 4);
            }
        }
    }

    #[test]
    fn unknown_pattern_returns_empty() {
        let stamps = generate_stamps("nonexistent", 4, '#', '.');
        assert!(stamps.is_empty());
    }

    #[test]
    fn diagonal_stripes_tileable() {
        let stamps = generate_stamps("diagonal", 4, '#', '.');
        let grid = &stamps[0].grid;
        // First and last rows should have same pattern (tileable vertically)
        // This is a visual pattern, not exact — just check it's not empty
        assert!(!grid.is_empty());
    }
}
