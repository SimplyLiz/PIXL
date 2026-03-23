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

    /// Start the MCP server (stdio transport)
    Mcp {
        /// Optional: pre-load a .pax file
        #[arg(long)]
        file: Option<PathBuf>,
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
        Commands::Mcp { file } => {
            cmd_mcp(file.as_deref());
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
            eprintln!(
                "error: tile '{}' has no grid data (RLE/compose not yet supported in CLI render)",
                tile_name
            );
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

    println!(
        "rendered '{}' ({}x{} @{}x) -> {}",
        tile_name,
        w,
        h,
        scale,
        out.display()
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
        let grid = match pixl_core::grid::parse_grid(tile_raw.grid.as_ref().unwrap(), w, h, palette)
        {
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
