use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "pixl", version, about = "PIXL — LLM-native pixel art toolchain")]
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
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { file, check_edges } => {
            cmd_validate(&file, check_edges);
        }
        Commands::Check { file, fix } => {
            cmd_validate(&file, false);
            if fix {
                eprintln!("--fix: auto-fix not yet implemented (coming soon)");
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
    }
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

fn load_pax(file: &std::path::Path) -> (pixl_core::types::PaxFile, std::collections::HashMap<String, pixl_core::types::Palette>) {
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

    let size_str = tile_raw.size.as_deref().unwrap_or("16x16");
    let (w, h) = match pixl_core::types::parse_size(size_str) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    let grid_str = match &tile_raw.grid {
        Some(g) => g,
        None => {
            eprintln!("error: tile '{}' has no grid data (RLE/compose not yet supported in CLI render)", tile_name);
            process::exit(1);
        }
    };

    // Parse grid with symmetry
    let (grid_w, grid_h) = match tile_raw.symmetry.as_deref() {
        Some("horizontal") => (w / 2, h),
        Some("vertical") => (w, h / 2),
        Some("quad") => (w / 2, h / 2),
        _ => (w, h),
    };

    let grid = match pixl_core::grid::parse_grid(grid_str, grid_w, grid_h, palette) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    // Expand symmetry
    let sym = match tile_raw.symmetry.as_deref() {
        Some("horizontal") => pixl_core::types::Symmetry::Horizontal,
        Some("vertical") => pixl_core::types::Symmetry::Vertical,
        Some("quad") => pixl_core::types::Symmetry::Quad,
        _ => pixl_core::types::Symmetry::None,
    };

    let full_grid = match pixl_core::symmetry::expand_symmetry(&grid, w, h, sym) {
        Ok(g) => g,
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

    println!("rendered '{}' ({}x{} @{}x) -> {}", tile_name, w, h, scale, out.display());
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

    // Collect tiles with grid data (skip templates)
    let mut atlas_tiles = Vec::new();
    for (name, tile_raw) in &pax_file.tile {
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
        let grid = match pixl_core::grid::parse_grid(tile_raw.grid.as_ref().unwrap(), w, h, palette) {
            Ok(g) => g,
            Err(_) => continue,
        };
        atlas_tiles.push(pixl_render::atlas::AtlasTile {
            name: name.clone(),
            grid,
            width: w,
            height: h,
        });
    }

    if atlas_tiles.is_empty() {
        eprintln!("error: no tiles with grid data found");
        process::exit(1);
    }

    // Use first tile's palette for rendering
    let first_palette_name = &pax_file.tile.values().next().unwrap().palette;
    let palette = &palettes[first_palette_name];

    let out_name = out.file_name().unwrap_or_default().to_string_lossy().to_string();

    match pixl_render::atlas::pack_atlas(&atlas_tiles, palette, columns, padding, scale, &out_name) {
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

    println!("preview '{}' ({}x{} @16x{}) -> {}",
        tile_name, w, h,
        if show_grid { " +grid" } else { "" },
        out.display()
    );
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
