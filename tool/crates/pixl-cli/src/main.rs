use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    name = "pixl",
    version,
    about = "PIXL — LLM-native pixel art toolchain"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a .pax file
    Validate {
        /// Path to the .pax file
        file: PathBuf,

        /// Check edge compatibility between tiles
        #[arg(long)]
        check_edges: bool,
    },

    /// Validate and auto-fix edge classes from grid content
    Check {
        /// Path to the .pax file
        file: PathBuf,

        /// Auto-generate missing edge classes from grid content
        #[arg(long)]
        fix: bool,
    },

    /// Render a tile to PNG
    Render {
        /// Path to the .pax file
        file: PathBuf,

        /// Tile name to render
        #[arg(long)]
        tile: String,

        /// Scale factor (default: 1)
        #[arg(long, default_value = "1")]
        scale: u32,

        /// Output PNG path
        #[arg(long, short)]
        out: PathBuf,
    },

    /// Pack tiles into a sprite atlas
    Atlas {
        /// Path to the .pax file
        file: PathBuf,

        /// Output atlas PNG path
        #[arg(long, short)]
        out: PathBuf,

        /// Output JSON metadata path
        #[arg(long)]
        map: Option<PathBuf>,

        /// Columns in atlas grid
        #[arg(long, default_value = "8")]
        columns: u32,

        /// Padding between tiles (pixels)
        #[arg(long, default_value = "1")]
        padding: u32,

        /// Scale factor
        #[arg(long, default_value = "1")]
        scale: u32,
    },

    /// Render a 16x zoom preview of a tile
    Preview {
        /// Path to the .pax file
        file: PathBuf,

        /// Tile name
        #[arg(long)]
        tile: String,

        /// Output PNG path
        #[arg(long, short)]
        out: PathBuf,

        /// Show pixel grid lines
        #[arg(long)]
        grid: bool,
    },

    /// Show anatomy blueprint for a canvas size
    Blueprint {
        /// Canvas size (e.g., "32x48")
        size: String,

        /// Anatomy model
        #[arg(long, default_value = "humanoid_chibi")]
        model: String,
    },

    /// Generate a map from a narrative description (or predicate rules)
    Narrate {
        /// Path to the .pax file with tileset
        file: PathBuf,

        /// Map width in tiles
        #[arg(long, default_value = "12")]
        width: usize,

        /// Map height in tiles
        #[arg(long, default_value = "8")]
        height: usize,

        /// RNG seed
        #[arg(long, default_value = "42")]
        seed: u64,

        /// Predicate rules (repeatable). Format: "border:wall", "region:boss:obstacle:3x3:southeast", "path:0,3:11,3"
        #[arg(long, short)]
        rule: Vec<String>,

        /// Output PNG path
        #[arg(long, short)]
        out: PathBuf,
    },

    /// Generate tile variants from a base tile
    Vary {
        /// Path to the .pax file
        file: PathBuf,

        /// Base tile name
        #[arg(long)]
        tile: String,

        /// Number of variants to generate
        #[arg(long, default_value = "4")]
        count: usize,

        /// RNG seed
        #[arg(long, default_value = "42")]
        seed: u64,

        /// Output directory for variant PNGs
        #[arg(long, short)]
        out: Option<PathBuf>,
    },

    /// Generate procedural stamps for a pattern
    GenerateStamps {
        /// Pattern: brick_bond, checkerboard, diagonal, dither_bayer, horizontal_stripe, dots, cross, noise
        pattern: String,

        /// Stamp size (e.g., "4x4", "8x8")
        #[arg(long, default_value = "4")]
        size: u32,

        /// Foreground symbol
        #[arg(long, default_value = "#")]
        fg: char,

        /// Background symbol
        #[arg(long, default_value = "+")]
        bg: char,
    },

    /// Extract style latent from reference tiles
    Style {
        /// Path to the .pax file
        file: PathBuf,

        /// Tile names to use as reference (comma-separated). Omit for all tiles.
        #[arg(long)]
        tiles: Option<String>,
    },

    /// Import a reference image into PAX format (diffusion bridge)
    Import {
        /// Reference image path (PNG, JPG, etc.)
        image: PathBuf,

        /// Target size (e.g., "16x16", "32x48")
        #[arg(long, default_value = "16x16")]
        size: String,

        /// Palette from a .pax file
        #[arg(long)]
        pax: PathBuf,

        /// Palette name within the .pax file
        #[arg(long)]
        palette: String,

        /// Apply Bayer dithering
        #[arg(long)]
        dither: bool,

        /// Output .pax grid to stdout (or --out for PNG preview)
        #[arg(long, short)]
        out: Option<PathBuf>,
    },

    /// Create or manage a .pixlproject file
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },

    /// Create a new .pax file from a built-in theme
    New {
        /// Theme: dark_fantasy, light_fantasy, sci_fi, nature, gameboy, nes
        theme: String,

        /// Output .pax file path
        #[arg(long, short)]
        out: PathBuf,
    },

    /// Export to game engine format
    Export {
        /// Path to the .pax file
        file: PathBuf,

        /// Export format: texturepacker, tiled, godot, unity, gbstudio
        #[arg(long, default_value = "tiled")]
        format: String,

        /// Output directory
        #[arg(long, short)]
        out: PathBuf,
    },

    /// Import a directory of PNG tiles into PAX corpus format
    Corpus {
        /// Directory containing PNG tile images
        dir: PathBuf,

        /// Palette from a .pax file to quantize into
        #[arg(long)]
        pax: PathBuf,

        /// Palette name within the .pax file
        #[arg(long)]
        palette: String,

        /// Target tile size (e.g., "16x16")
        #[arg(long, default_value = "16x16")]
        size: String,

        /// Output .pax file with stamps
        #[arg(long, short)]
        out: PathBuf,

        /// Also output training pairs JSON for LoRA fine-tuning
        #[arg(long)]
        training: Option<PathBuf>,
    },

    /// Start the MCP server (stdio transport)
    Mcp {
        /// Optional: pre-load a .pax file
        #[arg(long)]
        file: Option<PathBuf>,
    },

    /// Start the HTTP API server (for PIXL Studio)
    Serve {

        /// Port to listen on
        #[arg(long, default_value = "3742")]
        port: u16,

        /// Optional: pre-load a .pax file
        #[arg(long)]
        file: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum ProjectAction {
    /// Initialize a new project
    Init {
        /// Project name
        name: String,

        /// Theme to use
        #[arg(long)]
        theme: Option<String>,

        /// Output .pixlproject file
        #[arg(long, short, default_value = "project.pixlproject")]
        out: PathBuf,
    },

    /// Add a world to the project
    AddWorld {
        /// Path to .pixlproject file
        project: PathBuf,

        /// World name
        name: String,

        /// Path to .pax file
        pax: String,
    },

    /// Show project status
    Status {
        /// Path to .pixlproject file
        project: PathBuf,
    },

    /// Extract and save style latent to the project
    LearnStyle {
        /// Path to .pixlproject file
        project: PathBuf,

        /// World to extract style from
        world: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Project { action } => {
            cmd_project(action);
        }
        Commands::Validate { file, check_edges } => {
            cmd_validate(&file, check_edges);
        }
        Commands::Check { file, fix } => {
            if fix {
                cmd_check_fix(&file);
            } else {
                cmd_validate(&file, true);
            }
        }
        Commands::Render {
            file,
            tile,
            scale,
            out,
        } => {
            cmd_render(&file, &tile, scale, &out);
        }
        Commands::Atlas {
            file,
            out,
            map,
            columns,
            padding,
            scale,
        } => {
            cmd_atlas(&file, &out, map.as_deref(), columns, padding, scale);
        }
        Commands::Preview {
            file,
            tile,
            out,
            grid,
        } => {
            cmd_preview(&file, &tile, &out, grid);
        }
        Commands::Blueprint { size, model } => {
            cmd_blueprint(&size, &model);
        }
        Commands::Narrate {
            file,
            width,
            height,
            seed,
            rule,
            out,
        } => {
            cmd_narrate(&file, width, height, seed, &rule, &out);
        }
        Commands::Vary {
            file,
            tile,
            count,
            seed,
            out,
        } => {
            cmd_vary(&file, &tile, count, seed, out.as_deref());
        }
        Commands::GenerateStamps {
            pattern,
            size,
            fg,
            bg,
        } => {
            cmd_generate_stamps(&pattern, size, fg, bg);
        }
        Commands::Style { file, tiles } => {
            cmd_style(&file, tiles.as_deref());
        }
        Commands::Import {
            image,
            size,
            pax,
            palette,
            dither,
            out,
        } => {
            cmd_import(&image, &size, &pax, &palette, dither, out.as_deref());
        }
        Commands::New { theme, out } => {
            cmd_new(&theme, &out);
        }
        Commands::Export { file, format, out } => {
            cmd_export(&file, &format, &out);
        }
        Commands::Corpus {
            dir,
            pax,
            palette,
            size,
            out,
            training,
        } => {
            cmd_corpus(&dir, &pax, &palette, &size, &out, training.as_deref());
        }
        Commands::Mcp { file } => {
            cmd_mcp(file.as_deref());
        }
        Commands::Serve { port, file } => {
            cmd_serve(port, file.as_deref());
        }
    }
}

#[tokio::main]
async fn cmd_mcp_async(file: Option<&std::path::Path>) {
    let result = if let Some(path) = file {
        pixl_mcp::server::run_stdio_with_file(&path.to_string_lossy()).await
    } else {
        pixl_mcp::server::run_stdio().await
    };

    if let Err(e) = result {
        eprintln!("MCP server error: {}", e);
        process::exit(1);
    }
}

fn cmd_mcp(file: Option<&std::path::Path>) {
    cmd_mcp_async(file);
}

fn cmd_serve(port: u16, file: Option<&std::path::Path>) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let state = if let Some(path) = file {
            let source = std::fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                process::exit(1);
            });
            pixl_mcp::state::McpState::from_source(&source).unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                process::exit(1);
            })
        } else {
            pixl_mcp::state::McpState::new()
        };

        if let Err(e) = pixl_mcp::http::run_http(state, port).await {
            eprintln!("server error: {}", e);
            process::exit(1);
        }
    });
}

fn cmd_validate(file: &PathBuf, check_edges: bool) {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", file.display(), e);
            process::exit(1);
        }
    };

    let pax_file = match pixl_core::parser::parse_pax(&source) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("parse error: {}", e);
            process::exit(1);
        }
    };

    let result = pixl_core::validate::validate(&pax_file, check_edges);

    // Print stats
    println!(
        "pixl validate: {} palettes, {} themes, {} stamps, {} tiles, {} sprites",
        result.stats.palettes,
        result.stats.themes,
        result.stats.stamps,
        result.stats.tiles,
        result.stats.sprites,
    );

    // Print warnings
    for w in &result.warnings {
        println!("  warning: {}", w);
    }

    // Print errors
    for e in &result.errors {
        eprintln!("  error: {}", e);
    }

    if result.errors.is_empty() {
        println!("ok: no errors found.");
    } else {
        eprintln!(
            "{} error(s), {} warning(s)",
            result.errors.len(),
            result.warnings.len()
        );
        process::exit(1);
    }
}

/// Check if a user-declared edge class is compatible with an auto-classified one.
/// "solid" matches "solid_#", "floor" matches "solid_+", "open" matches "open", etc.
fn edge_class_compatible(declared: &str, auto: &str) -> bool {
    if declared == auto {
        return true;
    }
    // User-friendly alias: "solid" matches "solid_#", "solid_+"
    if auto.starts_with(declared) && auto.as_bytes().get(declared.len()) == Some(&b'_') {
        return true;
    }
    // "floor" is commonly used for "solid_+" (walkable surface)
    if declared == "floor" && auto.starts_with("solid_") {
        return true;
    }
    // "water" commonly used for "solid_~"
    if declared == "water" && auto.starts_with("solid_") {
        return true;
    }
    false
}

fn cmd_project(action: ProjectAction) {
    match action {
        ProjectAction::Init { name, theme, out } => {
            let proj = pixl_core::project::PixlProject::new(&name, theme.as_deref());
            if let Err(e) = proj.save(&out) {
                eprintln!("error: {}", e);
                process::exit(1);
            }
            println!("created project '{}' -> {}", name, out.display());
        }
        ProjectAction::AddWorld { project, name, pax } => {
            let mut proj = match pixl_core::project::PixlProject::load(&project) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("error: {}", e);
                    process::exit(1);
                }
            };
            proj.add_world(&name, &pax);
            if let Err(e) = proj.save(&project) {
                eprintln!("error: {}", e);
                process::exit(1);
            }
            println!("added world '{}' -> {}", name, pax);
        }
        ProjectAction::Status { project } => {
            let proj = match pixl_core::project::PixlProject::load(&project) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("error: {}", e);
                    process::exit(1);
                }
            };
            println!("{}", proj.summary());
            if let Some(ref latent) = proj.style_latent {
                println!();
                println!("{}", latent.describe());
            }
        }
        ProjectAction::LearnStyle { project, world } => {
            let mut proj = match pixl_core::project::PixlProject::load(&project) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("error: {}", e);
                    process::exit(1);
                }
            };

            let pax_path = match proj.worlds.get(&world) {
                Some(p) => p.clone(),
                None => {
                    eprintln!("error: world '{}' not found in project", world);
                    process::exit(1);
                }
            };

            // Resolve relative to project file directory
            let proj_dir = project.parent().unwrap_or(std::path::Path::new("."));
            let full_path = proj_dir.join(&pax_path);

            let (pax_file, palettes) = load_pax(&full_path);
            let palette_name = pax_file
                .tile
                .values()
                .next()
                .map(|t| t.palette.as_str())
                .unwrap_or("");
            let palette = match palettes.get(palette_name) {
                Some(p) => p,
                None => {
                    eprintln!("error: no palette found");
                    process::exit(1);
                }
            };

            let mut grids: Vec<Vec<Vec<char>>> = Vec::new();
            for (_, tile_raw) in &pax_file.tile {
                if tile_raw.template.is_some() || tile_raw.grid.is_none() {
                    continue;
                }
                let size_str = tile_raw.size.as_deref().unwrap_or("16x16");
                let (w, h) = match pixl_core::types::parse_size(size_str) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                if let Some(ref grid_str) = tile_raw.grid {
                    if let Ok(grid) = pixl_core::grid::parse_grid(grid_str, w, h, palette) {
                        grids.push(grid);
                    }
                }
            }

            let grid_refs: Vec<&Vec<Vec<char>>> = grids.iter().collect();
            let latent = pixl_core::style::StyleLatent::extract(&grid_refs, palette, '.');

            println!("{}", latent.describe());
            proj.style_latent = Some(latent);
            proj.progress.tiles_authored = grids.len() as u32;

            if let Err(e) = proj.save(&project) {
                eprintln!("error: {}", e);
                process::exit(1);
            }
            println!("style latent saved to {}", project.display());
        }
    }
}

fn cmd_corpus(
    dir: &PathBuf,
    pax_file: &PathBuf,
    palette_name: &str,
    size_str: &str,
    out: &PathBuf,
    training_path: Option<&std::path::Path>,
) {
    let (_, palettes) = load_pax(pax_file);
    let palette = match palettes.get(palette_name) {
        Some(p) => p,
        None => {
            eprintln!("error: palette '{}' not found", palette_name);
            process::exit(1);
        }
    };

    let (tw, th) = match pixl_core::types::parse_size(size_str) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    // Find all PNG files in the directory
    let png_files: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .map(|ext| ext == "png" || ext == "PNG")
                    .unwrap_or(false)
            })
            .collect(),
        Err(e) => {
            eprintln!("error reading directory: {}", e);
            process::exit(1);
        }
    };

    if png_files.is_empty() {
        eprintln!("error: no PNG files found in {}", dir.display());
        process::exit(1);
    }

    println!("found {} PNG files in {}", png_files.len(), dir.display());

    let mut entries = Vec::new();
    let mut failed = Vec::new();

    for png_path in &png_files {
        let file_stem = png_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Sanitize name for TOML key
        let name = file_stem
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
            .collect::<String>();

        match image::open(png_path) {
            Ok(img) => {
                let resized = img.resize_exact(tw, th, image::imageops::FilterType::Lanczos3);
                let rgba = resized.to_rgba8();

                let pixels: Vec<(u8, u8, u8, u8)> = rgba
                    .pixels()
                    .map(|p| (p.0[0], p.0[1], p.0[2], p.0[3]))
                    .collect();

                let (grid, accuracy) =
                    pixl_core::corpus::quantize_pixels(&pixels, tw, th, palette, '.');

                // Try to infer affordance from filename
                let affordance = pixl_core::corpus::map_affordance(&file_stem);

                entries.push(pixl_core::corpus::CorpusEntry {
                    name: name.clone(),
                    source_file: png_path.to_string_lossy().to_string(),
                    width: tw,
                    height: th,
                    grid,
                    palette_name: palette_name.to_string(),
                    affordance,
                    tags: vec![],
                    color_accuracy: accuracy,
                });

                println!(
                    "  {} -> {} ({:.1}% accuracy)",
                    file_stem,
                    name,
                    accuracy * 100.0
                );
            }
            Err(e) => {
                failed.push((file_stem.clone(), format!("{}", e)));
                eprintln!("  SKIP {}: {}", file_stem, e);
            }
        }
    }

    let batch = pixl_core::corpus::CorpusBatch {
        entries,
        failed,
        palette: palette.clone(),
        palette_name: palette_name.to_string(),
    };

    // Write .pax stamps
    let pax_output = pixl_core::corpus::generate_pax_stamps(&batch);
    if let Err(e) = std::fs::write(out, &pax_output) {
        eprintln!("error writing {}: {}", out.display(), e);
        process::exit(1);
    }
    println!(
        "\nwrote {} stamps to {} ({} failed)",
        batch.entries.len(),
        out.display(),
        batch.failed.len()
    );

    // Write training pairs if requested
    if let Some(tp) = training_path {
        let pairs = pixl_core::corpus::generate_training_pairs(&batch);
        let json = serde_json::to_string_pretty(&pairs).unwrap();
        if let Err(e) = std::fs::write(tp, &json) {
            eprintln!("error writing training data: {}", e);
        } else {
            println!("wrote {} training pairs to {}", pairs.len(), tp.display());
        }
    }
}

fn cmd_new(theme: &str, out: &PathBuf) {
    let themes = [
        ("dark_fantasy", include_str!("../../../themes/dark_fantasy.pax")),
        ("light_fantasy", include_str!("../../../themes/light_fantasy.pax")),
        ("sci_fi", include_str!("../../../themes/sci_fi.pax")),
        ("nature", include_str!("../../../themes/nature.pax")),
        ("gameboy", include_str!("../../../themes/gameboy.pax")),
        ("nes", include_str!("../../../themes/nes.pax")),
    ];

    let content = match themes.iter().find(|(name, _)| *name == theme) {
        Some((_, content)) => content,
        None => {
            let available: Vec<&str> = themes.iter().map(|(n, _)| *n).collect();
            eprintln!(
                "error: unknown theme '{}'. Available: {}",
                theme,
                available.join(", ")
            );
            process::exit(1);
        }
    };

    // Create parent directories if they don't exist
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("error: cannot create directory {}: {}", parent.display(), e);
                process::exit(1);
            }
        }
    }

    if let Err(e) = std::fs::write(out, content) {
        eprintln!("error: {}", e);
        process::exit(1);
    }

    println!("created {} from theme '{}'", out.display(), theme);
}

fn cmd_check_fix(file: &PathBuf) {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", file.display(), e);
            process::exit(1);
        }
    };

    let mut pax_file = match pixl_core::parser::parse_pax(&source) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("parse error: {}", e);
            process::exit(1);
        }
    };

    let palettes = match pixl_core::parser::resolve_all_palettes(&pax_file) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("palette error: {}", e);
            process::exit(1);
        }
    };

    let mut fixed = 0usize;
    let mut warned = 0usize;

    for (name, tile_raw) in pax_file.tile.iter_mut() {
        if tile_raw.template.is_some() || tile_raw.grid.is_none() {
            continue;
        }
        let size_str = tile_raw.size.as_deref().unwrap_or("16x16");
        let (w, h) = match pixl_core::types::parse_size(size_str) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let palette = match palettes.get(&tile_raw.palette) {
            Some(p) => p,
            None => continue,
        };
        let grid = match pixl_core::grid::parse_grid(tile_raw.grid.as_ref().unwrap(), w, h, palette)
        {
            Ok(g) => g,
            Err(_) => continue,
        };

        let auto = pixl_core::edges::auto_classify_edges(&grid);

        if tile_raw.edge_class.is_none() {
            // Fill missing
            tile_raw.edge_class = Some(pixl_core::types::EdgeClassRaw {
                n: auto.n.clone(),
                e: auto.e.clone(),
                s: auto.s.clone(),
                w: auto.w.clone(),
            });
            println!("  fixed: '{}' — edge_class generated", name);
            fixed += 1;
        } else {
            // Warn on mismatch (don't overwrite)
            let ec = tile_raw.edge_class.as_ref().unwrap();
            let mismatches: Vec<String> = [
                (&ec.n, &auto.n, "n"),
                (&ec.e, &auto.e, "e"),
                (&ec.s, &auto.s, "s"),
                (&ec.w, &auto.w, "w"),
            ]
            .iter()
            .filter(|(declared, computed, _)| !edge_class_compatible(declared, computed))
            .map(|(declared, computed, dir)| {
                format!("{}='{}' (auto='{}')", dir, declared, computed)
            })
            .collect();

            if !mismatches.is_empty() {
                println!(
                    "  warn: '{}' — edge mismatch: {}",
                    name,
                    mismatches.join(", ")
                );
                warned += 1;
            }
        }
    }

    if fixed > 0 {
        // Append edge_class sections to original source (preserves formatting)
        let mut appendix = String::new();
        for (name, tile_raw) in &pax_file.tile {
            if tile_raw.template.is_some() || tile_raw.grid.is_none() {
                continue;
            }
            // Only append for tiles we just fixed (no prior edge_class)
            if let Some(ref ec) = tile_raw.edge_class {
                // Check if this was in the original source (not our fix)
                let marker = format!("[tile.{}.edge_class]", name);
                if source.contains(&marker) {
                    continue;
                }
                appendix.push_str(&format!(
                    "\n[tile.{}.edge_class]\nn = \"{}\"\ne = \"{}\"\ns = \"{}\"\nw = \"{}\"\n",
                    name, ec.n, ec.e, ec.s, ec.w
                ));
            }
        }

        if !appendix.is_empty() {
            let new_source = format!("{}\n{}", source.trim_end(), appendix);
            if let Err(e) = std::fs::write(file, &new_source) {
                eprintln!("error writing {}: {}", file.display(), e);
                process::exit(1);
            }
            println!("wrote {} — {} edges fixed", file.display(), fixed);
        }
    }

    if warned > 0 {
        println!("{} edge mismatch warning(s) — not overwritten", warned);
    }
    if fixed == 0 && warned == 0 {
        println!("ok: all tiles have edge classes, no mismatches");
    }
}

fn cmd_export(file: &PathBuf, format: &str, out_dir: &PathBuf) {
    let (pax_file, palettes) = load_pax(file);

    // Create output directory
    if let Err(e) = std::fs::create_dir_all(out_dir) {
        eprintln!("error creating directory: {}", e);
        process::exit(1);
    }

    // Collect tile data
    let palette_name = pax_file
        .tile
        .values()
        .next()
        .map(|t| t.palette.as_str())
        .unwrap_or("");
    let palette = match palettes.get(palette_name) {
        Some(p) => p,
        None => {
            eprintln!("error: no palette found");
            process::exit(1);
        }
    };

    let mut tile_names: Vec<String> = Vec::new();
    let mut tile_grids: Vec<Vec<Vec<char>>> = Vec::new();
    let mut collision_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for (name, tile_raw) in &pax_file.tile {
        if tile_raw.template.is_some() || tile_raw.grid.is_none() {
            continue;
        }
        let size_str = tile_raw.size.as_deref().unwrap_or("16x16");
        let (w, h) = match pixl_core::types::parse_size(size_str) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if let Ok(grid) = pixl_core::grid::parse_grid(tile_raw.grid.as_ref().unwrap(), w, h, palette) {
            tile_names.push(name.clone());
            tile_grids.push(grid);
            if let Some(ref sem) = tile_raw.semantic {
                if let Some(ref c) = sem.collision {
                    collision_map.insert(name.clone(), c.clone());
                }
            }
        }
    }

    // Sort tiles alphabetically by name for deterministic GID assignment
    {
        let mut order: Vec<usize> = (0..tile_names.len()).collect();
        order.sort_by(|&a, &b| tile_names[a].cmp(&tile_names[b]));
        let sorted_names: Vec<String> = order.iter().map(|&i| tile_names[i].clone()).collect();
        let sorted_grids: Vec<Vec<Vec<char>>> = order.iter().map(|&i| tile_grids[i].clone()).collect();
        tile_names = sorted_names;
        tile_grids = sorted_grids;
        // collision_map is keyed by name so it doesn't need reordering
    }

    if tile_names.is_empty() {
        eprintln!("error: no tiles found");
        process::exit(1);
    }

    let tile_size = pax_file
        .tile
        .values()
        .next()
        .and_then(|t| t.size.as_deref())
        .and_then(|s| pixl_core::types::parse_size(s).ok())
        .unwrap_or((16, 16));

    match format {
        "texturepacker" | "tp" => {
            // Render atlas + TexturePacker JSON
            let atlas_tiles: Vec<pixl_render::atlas::AtlasTile> = tile_names
                .iter()
                .zip(tile_grids.iter())
                .map(|(name, grid)| pixl_render::atlas::AtlasTile {
                    name: name.clone(),
                    grid: grid.clone(),
                    width: tile_size.0,
                    height: tile_size.1,
                })
                .collect();

            let atlas_path = out_dir.join("atlas.png");
            let json_path = out_dir.join("atlas.json");

            match pixl_render::atlas::pack_atlas(&atlas_tiles, palette, 8, 1, 1, "atlas.png") {
                Ok((img, json)) => {
                    img.save(&atlas_path).unwrap();
                    let json_str = serde_json::to_string_pretty(&json).unwrap();
                    std::fs::write(&json_path, json_str).unwrap();
                    println!("texturepacker: {} -> {}", atlas_path.display(), json_path.display());
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    process::exit(1);
                }
            }
        }

        "tiled" | "tmj" => {
            // Atlas PNG + Tiled tileset JSON + empty map
            let atlas_tiles: Vec<pixl_render::atlas::AtlasTile> = tile_names
                .iter()
                .zip(tile_grids.iter())
                .map(|(name, grid)| pixl_render::atlas::AtlasTile {
                    name: name.clone(),
                    grid: grid.clone(),
                    width: tile_size.0,
                    height: tile_size.1,
                })
                .collect();

            let atlas_path = out_dir.join("tileset.png");
            let tsj_path = out_dir.join("tileset.tsj");

            match pixl_render::atlas::pack_atlas(&atlas_tiles, palette, 8, 1, 1, "tileset.png") {
                Ok((img, _)) => {
                    img.save(&atlas_path).unwrap();

                    let tileset = pixl_export::tiled::generate_tileset(
                        &pax_file.pax.name,
                        &tile_names,
                        tile_size.0,
                        tile_size.1,
                        "tileset.png",
                        img.width(),
                        img.height(),
                        8,
                        1, // spacing (between tiles)
                        1, // margin (from image edge)
                        &collision_map,
                    );
                    let tsj_str = serde_json::to_string_pretty(&tileset).unwrap();
                    std::fs::write(&tsj_path, tsj_str).unwrap();
                    println!("tiled: {} + {}", atlas_path.display(), tsj_path.display());
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    process::exit(1);
                }
            }
        }

        "godot" | "tres" => {
            let atlas_tiles: Vec<pixl_render::atlas::AtlasTile> = tile_names
                .iter()
                .zip(tile_grids.iter())
                .map(|(name, grid)| pixl_render::atlas::AtlasTile {
                    name: name.clone(),
                    grid: grid.clone(),
                    width: tile_size.0,
                    height: tile_size.1,
                })
                .collect();

            let atlas_path = out_dir.join("tileset.png");
            let tres_path = out_dir.join("tileset.tres");

            match pixl_render::atlas::pack_atlas(&atlas_tiles, palette, 8, 1, 1, "tileset.png") {
                Ok((img, _)) => {
                    img.save(&atlas_path).unwrap();
                    let tres = pixl_export::godot::generate_tres(
                        &pax_file.pax.name,
                        &tile_names,
                        tile_size.0,
                        tile_size.1,
                        "tileset.png",
                        &collision_map,
                    );
                    std::fs::write(&tres_path, tres).unwrap();
                    println!("godot: {} + {}", atlas_path.display(), tres_path.display());
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    process::exit(1);
                }
            }
        }

        _ => {
            eprintln!("error: unknown format '{}'. Supported: texturepacker, tiled, godot", format);
            process::exit(1);
        }
    }
}

fn load_pax(
    file: &std::path::Path,
) -> (
    pixl_core::types::PaxFile,
    std::collections::HashMap<String, pixl_core::types::Palette>,
) {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", file.display(), e);
            process::exit(1);
        }
    };
    let pax_file = match pixl_core::parser::parse_pax(&source) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("parse error: {}", e);
            process::exit(1);
        }
    };
    let palettes = match pixl_core::parser::resolve_all_palettes(&pax_file) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("palette error: {}", e);
            process::exit(1);
        }
    };
    (pax_file, palettes)
}

fn cmd_render(file: &PathBuf, tile_name: &str, scale: u32, out: &PathBuf) {
    let (pax_file, palettes) = load_pax(file);

    let tile_raw = match pax_file.tile.get(tile_name) {
        Some(t) => t,
        None => {
            eprintln!("error: tile '{}' not found", tile_name);
            process::exit(1);
        }
    };

    let palette = match palettes.get(&tile_raw.palette) {
        Some(p) => p,
        None => {
            eprintln!("error: palette '{}' not found", tile_raw.palette);
            process::exit(1);
        }
    };

    // Use unified resolver — handles grid, RLE, compose, template, symmetry
    let (full_grid, w, h) = match pixl_core::resolve::resolve_tile_grid(
        tile_name,
        &pax_file.tile,
        &palettes,
        &std::collections::HashMap::new(), // stamps resolved separately if needed
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    let img = pixl_render::renderer::render_grid(&full_grid, palette, scale);

    if let Err(e) = img.save(out) {
        eprintln!("error: cannot write {}: {}", out.display(), e);
        process::exit(1);
    }

    println!(
        "rendered '{}' ({}x{} @{}x) -> {}",
        tile_name, w, h, scale, out.display()
    );
}

fn cmd_atlas(
    file: &PathBuf,
    out: &PathBuf,
    map_path: Option<&std::path::Path>,
    columns: u32,
    padding: u32,
    scale: u32,
) {
    let (pax_file, palettes) = load_pax(file);

    // Collect tiles using unified resolver (handles grid, RLE, compose, template, symmetry)
    let mut atlas_tiles = Vec::new();
    for name in pax_file.tile.keys() {
        let tile_raw = &pax_file.tile[name];
        if tile_raw.template.is_some() {
            continue; // template tiles use parent's grid
        }
        match pixl_core::resolve::resolve_tile_grid(
            name,
            &pax_file.tile,
            &palettes,
            &std::collections::HashMap::new(),
        ) {
            Ok((grid, w, h)) => {
                atlas_tiles.push(pixl_render::atlas::AtlasTile {
                    name: name.clone(),
                    grid,
                    width: w,
                    height: h,
                });
            }
            Err(_) => continue,
        }
    }

    if atlas_tiles.is_empty() {
        eprintln!("error: no resolvable tiles found");
        process::exit(1);
    }

    // Use first tile's palette for rendering
    let first_palette_name = &pax_file.tile.values().next().unwrap().palette;
    let palette = &palettes[first_palette_name];

    let out_name = out
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    match pixl_render::atlas::pack_atlas(&atlas_tiles, palette, columns, padding, scale, &out_name)
    {
        Ok((img, json)) => {
            if let Err(e) = img.save(out) {
                eprintln!("error: cannot write atlas: {}", e);
                process::exit(1);
            }
            println!("atlas: {} tiles -> {}", atlas_tiles.len(), out.display());

            if let Some(map_out) = map_path {
                let json_str = serde_json::to_string_pretty(&json).unwrap();
                if let Err(e) = std::fs::write(map_out, json_str) {
                    eprintln!("error: cannot write JSON: {}", e);
                    process::exit(1);
                }
                println!("metadata -> {}", map_out.display());
            }
        }
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    }
}

fn cmd_preview(file: &PathBuf, tile_name: &str, out: &PathBuf, show_grid: bool) {
    let (pax_file, palettes) = load_pax(file);

    let tile_raw = match pax_file.tile.get(tile_name) {
        Some(t) => t,
        None => {
            eprintln!("error: tile '{}' not found", tile_name);
            process::exit(1);
        }
    };

    let palette = match palettes.get(&tile_raw.palette) {
        Some(p) => p,
        None => {
            eprintln!("error: palette '{}' not found", tile_raw.palette);
            process::exit(1);
        }
    };

    let size_str = tile_raw.size.as_deref().unwrap_or("16x16");
    let (w, h) = pixl_core::types::parse_size(size_str).unwrap_or((16, 16));

    let grid_str = match &tile_raw.grid {
        Some(g) => g,
        None => {
            eprintln!("error: tile '{}' has no grid data", tile_name);
            process::exit(1);
        }
    };

    let grid = pixl_core::grid::parse_grid(grid_str, w, h, palette).unwrap_or_else(|e| {
        eprintln!("error: {}", e);
        process::exit(1);
    });

    let preview_scale = 16u32;
    let img = pixl_render::renderer::render_grid(&grid, palette, preview_scale);
    let preview = pixl_render::preview::render_preview(&img, w, h, preview_scale, show_grid);

    if let Err(e) = preview.save(out) {
        eprintln!("error: {}", e);
        process::exit(1);
    }

    println!(
        "preview '{}' ({}x{} @16x{}) -> {}",
        tile_name,
        w,
        h,
        if show_grid { " +grid" } else { "" },
        out.display()
    );
}

fn cmd_narrate(
    file: &PathBuf,
    width: usize,
    height: usize,
    seed: u64,
    rules: &[String],
    out: &PathBuf,
) {
    let (pax_file, palettes) = load_pax(file);

    // Get first palette
    let palette_name = pax_file
        .tile
        .values()
        .next()
        .map(|t| t.palette.as_str())
        .unwrap_or("");
    let palette = match palettes.get(palette_name) {
        Some(p) => p,
        None => {
            eprintln!("error: no palette found");
            process::exit(1);
        }
    };

    // Build tile edges and affordances from pax file
    let mut tile_edges = Vec::new();
    let mut tile_affordances = Vec::new();
    let mut tile_names_ordered = Vec::new();
    let mut tile_grids: Vec<Vec<Vec<char>>> = Vec::new();

    for (name, tile_raw) in &pax_file.tile {
        if tile_raw.template.is_some() || tile_raw.grid.is_none() {
            continue;
        }
        let ec = tile_raw.edge_class.as_ref();
        tile_edges.push(pixl_wfc::adjacency::TileEdges {
            name: name.clone(),
            n: ec.map(|e| e.n.clone()).unwrap_or_default(),
            e: ec.map(|e| e.e.clone()).unwrap_or_default(),
            s: ec.map(|e| e.s.clone()).unwrap_or_default(),
            w: ec.map(|e| e.w.clone()).unwrap_or_default(),
            weight: tile_raw.weight,
        });
        tile_affordances.push(pixl_wfc::semantic::TileAffordance {
            affordance: tile_raw
                .semantic
                .as_ref()
                .and_then(|s| s.affordance.clone()),
        });
        tile_names_ordered.push(name.clone());

        // Parse grid for rendering
        let size_str = tile_raw.size.as_deref().unwrap_or("16x16");
        let (w, h) = pixl_core::types::parse_size(size_str).unwrap_or((16, 16));
        if let Some(ref grid_str) = tile_raw.grid {
            if let Ok(grid) = pixl_core::grid::parse_grid(grid_str, w, h, palette) {
                tile_grids.push(grid);
            } else {
                tile_grids.push(vec![vec!['.'; w as usize]; h as usize]);
            }
        } else {
            tile_grids.push(vec![vec!['.'; w as usize]; h as usize]);
        }
    }

    // Sort tiles alphabetically by name for deterministic ordering
    {
        let mut order: Vec<usize> = (0..tile_names_ordered.len()).collect();
        order.sort_by(|&a, &b| tile_names_ordered[a].cmp(&tile_names_ordered[b]));
        let sorted_names: Vec<String> = order.iter().map(|&i| tile_names_ordered[i].clone()).collect();
        let sorted_edges: Vec<_> = order.iter().map(|&i| tile_edges[i].clone()).collect();
        let sorted_affordances: Vec<_> = order.iter().map(|&i| tile_affordances[i].clone()).collect();
        let sorted_grids: Vec<Vec<Vec<char>>> = order.iter().map(|&i| tile_grids[i].clone()).collect();
        tile_names_ordered = sorted_names;
        tile_edges = sorted_edges;
        tile_affordances = sorted_affordances;
        tile_grids = sorted_grids;
    }

    // Expand auto-rotated tiles into the pool
    let mut rotation_additions: Vec<(
        pixl_wfc::adjacency::TileEdges,
        pixl_wfc::semantic::TileAffordance,
        String,
        Vec<Vec<char>>,
    )> = Vec::new();

    for (name, tile_raw) in &pax_file.tile {
        let rotate = tile_raw.auto_rotate.as_deref().unwrap_or("none");
        if rotate == "none" {
            continue;
        }

        // Find this tile's index in the edges list
        let Some(idx) = tile_names_ordered.iter().position(|n| n == name) else {
            continue;
        };

        // Build a minimal Tile for generate_variants
        let source_tile = pixl_core::types::Tile {
            name: name.clone(),
            palette: tile_raw.palette.clone(),
            width: 0,
            height: 0,
            encoding: pixl_core::types::Encoding::Grid,
            symmetry: pixl_core::types::Symmetry::None,
            auto_rotate: match rotate {
                "4way" => pixl_core::types::AutoRotate::FourWay,
                "flip" => pixl_core::types::AutoRotate::Flip,
                "8way" => pixl_core::types::AutoRotate::EightWay,
                _ => pixl_core::types::AutoRotate::None,
            },
            edge_class: pixl_core::types::EdgeClass {
                n: tile_edges[idx].n.clone(),
                e: tile_edges[idx].e.clone(),
                s: tile_edges[idx].s.clone(),
                w: tile_edges[idx].w.clone(),
            },
            tags: vec![],
            weight: tile_raw.weight,
            palette_swaps: vec![],
            cycles: vec![],
            nine_slice: None,
            visual_height_extra: None,
            semantic: None,
            grid: tile_grids[idx].clone(),
        };

        let weight_mode = tile_raw.auto_rotate_weight.as_deref();
        for (suffix, rotated_grid, rotated_ec, variant_weight) in
            pixl_core::rotate::generate_variants(&source_tile, weight_mode)
        {
            let variant_name = format!("{}{}", name, suffix);
            rotation_additions.push((
                pixl_wfc::adjacency::TileEdges {
                    name: variant_name.clone(),
                    n: rotated_ec.n,
                    e: rotated_ec.e,
                    s: rotated_ec.s,
                    w: rotated_ec.w,
                    weight: variant_weight,
                },
                tile_affordances[idx].clone(),
                variant_name,
                rotated_grid,
            ));
        }
    }

    for (edges, affordance, name, grid) in rotation_additions {
        tile_edges.push(edges);
        tile_affordances.push(affordance);
        tile_names_ordered.push(name);
        tile_grids.push(grid);
    }

    if tile_edges.is_empty() {
        eprintln!("error: no tiles with edge classes found");
        process::exit(1);
    }

    // Build WFC rules
    let variant_groups = pax_file
        .wfc_rules
        .as_ref()
        .map(|r| r.variant_groups.clone())
        .unwrap_or_default();
    let adj_rules = pixl_wfc::adjacency::AdjacencyRules::build(&tile_edges, &variant_groups);

    // Parse semantic rules
    let forbids: Vec<pixl_wfc::semantic::SemanticRule> = pax_file
        .wfc_rules
        .as_ref()
        .map(|r| {
            r.forbids
                .iter()
                .filter_map(|s| pixl_wfc::semantic::parse_forbids(s))
                .collect()
        })
        .unwrap_or_default();

    let requires: Vec<pixl_wfc::semantic::SemanticRule> = pax_file
        .wfc_rules
        .as_ref()
        .map(|r| {
            r.requires
                .iter()
                .filter_map(|s| pixl_wfc::semantic::parse_requires(s))
                .collect()
        })
        .unwrap_or_default();

    let require_boost = pax_file
        .wfc_rules
        .as_ref()
        .map(|r| r.require_boost)
        .unwrap_or(3.0);

    // Parse predicates from rules
    let predicates: Vec<pixl_wfc::narrate::Predicate> = rules
        .iter()
        .filter_map(|r| pixl_wfc::narrate::parse_predicate(r))
        .collect();

    if predicates.is_empty() && rules.is_empty() {
        // Default: border with first obstacle tile
        eprintln!("hint: no rules provided. Use -r 'border:wall' -r 'region:room:walkable:4x4:center'");
    }

    let narrate_config = pixl_wfc::narrate::NarrateConfig {
        width,
        height,
        seed,
        max_retries: 5,
        predicates,
    };

    println!("narrate: {}x{} map, seed={}, {} rules", width, height, seed, rules.len());

    match pixl_wfc::narrate::narrate_map(
        &tile_edges,
        &tile_affordances,
        &adj_rules,
        &forbids,
        &requires,
        require_boost,
        &narrate_config,
    ) {
        Ok(result) => {
            println!(
                "ok: generated in {} retries, {} pins applied",
                result.retries, result.pins_applied
            );

            // Render the map
            let tile_size = pax_file
                .tile
                .values()
                .next()
                .and_then(|t| t.size.as_deref())
                .and_then(|s| pixl_core::types::parse_size(s).ok())
                .unwrap_or((16, 16));

            let scale = 2u32;
            let img_w = width as u32 * tile_size.0 * scale;
            let img_h = height as u32 * tile_size.1 * scale;

            let mut img = image::ImageBuffer::new(img_w, img_h);

            for (ty, row) in result.grid.iter().enumerate() {
                for (tx, &tile_idx) in row.iter().enumerate() {
                    if tile_idx < tile_grids.len() {
                        let tile_img =
                            pixl_render::renderer::render_grid(&tile_grids[tile_idx], palette, scale);
                        let ox = tx as u32 * tile_size.0 * scale;
                        let oy = ty as u32 * tile_size.1 * scale;
                        for py in 0..tile_img.height() {
                            for px in 0..tile_img.width() {
                                let ax = ox + px;
                                let ay = oy + py;
                                if ax < img_w && ay < img_h {
                                    img.put_pixel(ax, ay, *tile_img.get_pixel(px, py));
                                }
                            }
                        }
                    }
                }
            }

            if let Err(e) = img.save(out) {
                eprintln!("error: {}", e);
                process::exit(1);
            }
            println!("map -> {}", out.display());

            // Print tile name grid
            println!();
            for row in &result.grid {
                let names: Vec<&str> = row
                    .iter()
                    .map(|&idx| {
                        tile_names_ordered
                            .get(idx)
                            .map(|s| s.as_str())
                            .unwrap_or("?")
                    })
                    .collect();
                println!("  {}", names.join(" "));
            }
        }
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    }
}

fn cmd_vary(
    file: &PathBuf,
    tile_name: &str,
    count: usize,
    seed: u64,
    out_dir: Option<&std::path::Path>,
) {
    let (pax_file, palettes) = load_pax(file);

    let tile_raw = match pax_file.tile.get(tile_name) {
        Some(t) => t,
        None => {
            eprintln!("error: tile '{}' not found", tile_name);
            process::exit(1);
        }
    };

    let palette = match palettes.get(&tile_raw.palette) {
        Some(p) => p,
        None => {
            eprintln!("error: palette '{}' not found", tile_raw.palette);
            process::exit(1);
        }
    };

    // Resolve the base grid
    let (base_grid, w, h) = match pixl_core::resolve::resolve_tile_grid(
        tile_name,
        &pax_file.tile,
        &palettes,
        &std::collections::HashMap::new(),
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    let variants = pixl_core::vary::generate_variants(tile_name, &base_grid, palette, count, seed, '.');

    for v in &variants {
        println!("[tile.{}]", v.name);
        println!("palette = \"{}\"", tile_raw.palette);
        println!("size    = \"{}x{}\"", w, h);
        println!("# mutation: {}", v.mutation);
        println!("grid = '''");
        for row in &v.grid {
            println!("{}", row.iter().collect::<String>());
        }
        println!("'''");
        println!();
    }

    // Render PNGs if output dir specified
    if let Some(dir) = out_dir {
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("error: {}", e);
            process::exit(1);
        }
        for v in &variants {
            let img = pixl_render::renderer::render_grid(&v.grid, palette, 4);
            let path = dir.join(format!("{}.png", v.name));
            if let Err(e) = img.save(&path) {
                eprintln!("error: {}", e);
            } else {
                println!("  {} -> {}", v.name, path.display());
            }
        }
    }

    println!("# Generated {} variant(s) from '{}' (seed={})", variants.len(), tile_name, seed);
}

fn cmd_generate_stamps(pattern: &str, size: u32, fg: char, bg: char) {
    let stamps = pixl_core::stampgen::generate_stamps(pattern, size, fg, bg);
    if stamps.is_empty() {
        eprintln!(
            "error: unknown pattern '{}'. Available: {:?}",
            pattern,
            pixl_core::stampgen::available_patterns()
        );
        process::exit(1);
    }

    for stamp in &stamps {
        println!("[stamp.{}]", stamp.name);
        println!("palette = \"<your_palette>\"");
        println!("size    = \"{}x{}\"", stamp.width, stamp.height);
        println!("grid    = '''");
        for row in &stamp.grid {
            println!("{}", row.iter().collect::<String>());
        }
        println!("'''");
        println!();
    }

    println!("# Generated {} stamp(s) for pattern '{}'", stamps.len(), pattern);
}

fn cmd_style(file: &PathBuf, tiles_filter: Option<&str>) {
    let (pax_file, palettes) = load_pax(file);

    // Get first palette
    let palette_name = pax_file
        .tile
        .values()
        .next()
        .map(|t| t.palette.as_str())
        .unwrap_or("");
    let palette = match palettes.get(palette_name) {
        Some(p) => p,
        None => {
            eprintln!("error: no palette found");
            process::exit(1);
        }
    };

    // Collect grids from selected tiles
    let tile_names: Option<Vec<&str>> = tiles_filter.map(|s| s.split(',').map(|t| t.trim()).collect());

    let mut grids: Vec<Vec<Vec<char>>> = Vec::new();
    let mut used_names: Vec<String> = Vec::new();

    for (name, tile_raw) in &pax_file.tile {
        if tile_raw.template.is_some() || tile_raw.grid.is_none() {
            continue;
        }
        if let Some(ref filter) = tile_names {
            if !filter.contains(&name.as_str()) {
                continue;
            }
        }
        let size_str = tile_raw.size.as_deref().unwrap_or("16x16");
        let (w, h) = match pixl_core::types::parse_size(size_str) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if let Some(ref grid_str) = tile_raw.grid {
            if let Ok(grid) = pixl_core::grid::parse_grid(grid_str, w, h, palette) {
                grids.push(grid);
                used_names.push(name.clone());
            }
        }
    }

    if grids.is_empty() {
        eprintln!("error: no valid tiles found for style extraction");
        process::exit(1);
    }

    let grid_refs: Vec<&Vec<Vec<char>>> = grids.iter().collect();
    let latent = pixl_core::style::StyleLatent::extract(&grid_refs, palette, '.');

    println!("{}", latent.describe());
    println!();
    println!("Reference tiles: {}", used_names.join(", "));
    println!();

    // Output TOML for embedding in project file
    println!("# TOML for .pixlproject [style_latent] section:");
    println!("{}", toml::to_string_pretty(&latent).unwrap());
}

fn cmd_import(
    image_path: &PathBuf,
    size: &str,
    pax_path: &PathBuf,
    palette_name: &str,
    dither: bool,
    out: Option<&std::path::Path>,
) {
    let (w, h) = match pixl_core::types::parse_size(size) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    // Load reference image
    let img = match image::open(image_path) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("error: cannot open image {}: {}", image_path.display(), e);
            process::exit(1);
        }
    };

    // Load palette from pax file
    let (_, palettes) = load_pax(pax_path);
    let palette = match palettes.get(palette_name) {
        Some(p) => p,
        None => {
            eprintln!("error: palette '{}' not found", palette_name);
            process::exit(1);
        }
    };

    let result = pixl_render::import::import_reference(&img, w, h, palette, dither);

    println!("# Imported {}x{} from {}", w, h, image_path.display());
    println!("# Color accuracy: {:.1}%", result.color_accuracy * 100.0);
    println!("# Clipped colors: {}", result.clipped_colors);
    println!("# Dither: {}", if dither { "bayer" } else { "none" });
    println!();
    println!("{}", result.grid_string);

    // Optionally render preview
    if let Some(out_path) = out {
        let preview = pixl_render::renderer::render_grid(&result.grid, palette, 16);
        if let Err(e) = preview.save(out_path) {
            eprintln!("error: {}", e);
            process::exit(1);
        }
        println!();
        println!("preview -> {}", out_path.display());
    }
}

fn cmd_blueprint(size: &str, model: &str) {
    let (w, h) = match pixl_core::types::parse_size(size) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    match pixl_core::blueprint::render_guide(model, w, h) {
        Some(guide) => println!("{}", guide),
        None => {
            eprintln!(
                "error: unknown blueprint model '{}'. Available: {:?}",
                model,
                pixl_core::blueprint::available_models()
            );
            process::exit(1);
        }
    }
}
