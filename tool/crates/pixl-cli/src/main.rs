use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::process;

/// Output format for narrate command.
#[derive(Debug, Clone, ValueEnum, Default)]
enum OutputFormat {
    #[default]
    Text,
    Json,
}

/// Parse a weight override: "tile_name:value"
fn parse_weight_override(s: &str) -> Result<(String, f64), String> {
    let (name, val) = s
        .rsplit_once(':')
        .ok_or_else(|| format!("expected NAME:VALUE, got '{s}'"))?;
    let v: f64 = val
        .parse()
        .map_err(|e| format!("bad weight value '{val}': {e}"))?;
    if v < 0.0 {
        return Err(format!("weight must be non-negative, got {v}"));
    }
    Ok((name.to_string(), v))
}

/// Parse a pin: "x,y:tile_name"
fn parse_pin(s: &str) -> Result<(usize, usize, String), String> {
    let (coords, name) = s
        .split_once(':')
        .ok_or_else(|| format!("expected X,Y:TILE_NAME, got '{s}'"))?;
    let (xs, ys) = coords
        .split_once(',')
        .ok_or_else(|| format!("expected X,Y in coords, got '{coords}'"))?;
    let x: usize = xs.parse().map_err(|e| format!("bad x '{xs}': {e}"))?;
    let y: usize = ys.parse().map_err(|e| format!("bad y '{ys}': {e}"))?;
    Ok((x, y, name.to_string()))
}

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

        /// Analyze tileset completeness for WFC (missing transition tiles)
        #[arg(long)]
        completeness: bool,

        /// Check seam continuity across composite tile boundaries
        #[arg(long)]
        check_seams: bool,

        /// Run per-tile quality analysis and cross-tile style consistency checks
        #[arg(long)]
        quality: bool,
    },

    /// Validate and auto-fix edge classes from grid content
    Check {
        /// Path to the .pax file
        file: PathBuf,

        /// Auto-generate missing edge classes from grid content
        #[arg(long)]
        fix: bool,

        /// Check sub-completeness (guarantees contradiction-free WFC)
        #[arg(long)]
        subcomplete: bool,
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
        #[arg(long, short, alias = "output")]
        out: PathBuf,
    },

    /// Pack tiles into a sprite atlas
    Atlas {
        /// Path to the .pax file
        file: PathBuf,

        /// Output atlas PNG path
        #[arg(long, short, alias = "output")]
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
        #[arg(long, short, alias = "output")]
        out: PathBuf,

        /// Show pixel grid lines
        #[arg(long)]
        grid: bool,
    },

    /// Structural quality critique of a tile
    Critique {
        /// Path to the .pax file
        file: PathBuf,

        /// Tile name
        #[arg(long)]
        tile: String,
    },

    /// Upscale a tile grid (nearest-neighbor) for progressive resolution workflow
    Upscale {
        /// Path to the .pax file
        file: PathBuf,

        /// Tile name to upscale
        #[arg(long)]
        tile: String,

        /// Scale factor (2 = 8x8→16x16, 4 = 8x8→32x32)
        #[arg(long, default_value = "2")]
        factor: u32,

        /// Output PNG path (rendered preview)
        #[arg(long, short, alias = "output")]
        out: PathBuf,
    },

    /// Generate a sprite via DALL-E + palette quantization
    GenerateSprite {
        /// Path to the .pax file (provides palette)
        file: PathBuf,

        /// Text prompt describing the sprite
        #[arg(long)]
        prompt: String,

        /// Tile name to create
        #[arg(long)]
        name: String,

        /// Target size (e.g., "16x16", "32x32", or "auto" to detect from generated image)
        #[arg(long, default_value = "auto")]
        size: String,

        /// Enable Bayer dithering
        #[arg(long)]
        dither: bool,

        /// Max colors for auto-palette extraction
        #[arg(long, default_value = "32")]
        max_colors: u32,

        /// Output PNG path (quantized preview)
        #[arg(long, short, alias = "output")]
        out: PathBuf,
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
        #[arg(long, short, alias = "output")]
        out: PathBuf,

        /// Override tile weights (repeatable). Format: "tile_name:value"
        #[arg(long, short = 'w', value_parser = parse_weight_override)]
        weight: Vec<(String, f64)>,

        /// Pin specific cells (repeatable). Format: "x,y:tile_name"
        #[arg(long, value_parser = parse_pin)]
        pin: Vec<(usize, usize, String)>,

        /// Output format: text (default) or json
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
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
        #[arg(long, short, alias = "output")]
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
        #[arg(long, short, alias = "output")]
        out: Option<PathBuf>,
    },

    /// Create or manage a .pixlproject file
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },

    /// Create a new .pax file from a built-in theme
    New {
        /// Theme: dark_fantasy, light_fantasy, sci_fi, nature, gameboy, nes, snes, gba
        theme: String,

        /// Output .pax file path
        #[arg(long, short, alias = "output")]
        out: PathBuf,

        /// Generate tiles via AI instead of using static templates.
        /// Outputs knowledge-enriched prompts for each tile type.
        /// Pipe to an LLM: pixl new dark_fantasy --out my.pax --generate | llm
        #[arg(long)]
        generate: bool,
    },

    /// Export to game engine format
    Export {
        /// Path to the .pax file
        file: PathBuf,

        /// Export format: texturepacker, tiled, godot
        #[arg(long, default_value = "tiled")]
        format: String,

        /// Output directory
        #[arg(long, short, alias = "output")]
        out: PathBuf,
    },

    /// Render a sprite animation as GIF or PNG spritesheet
    RenderSprite {
        /// Path to the .pax file
        file: PathBuf,

        /// Spriteset name
        #[arg(long)]
        spriteset: String,

        /// Sprite name within the spriteset
        #[arg(long)]
        sprite: String,

        /// Scale factor
        #[arg(long, default_value = "4")]
        scale: u32,

        /// Output path (.gif or .png for spritesheet)
        #[arg(long, short, alias = "output")]
        out: PathBuf,
    },

    /// Render a composite sprite to PNG
    RenderComposite {
        /// Path to the .pax file
        file: PathBuf,

        /// Composite name
        #[arg(long)]
        composite: String,

        /// Variant name (optional)
        #[arg(long)]
        variant: Option<String>,

        /// Animation name (optional)
        #[arg(long)]
        anim: Option<String>,

        /// Frame index (1-based, optional)
        #[arg(long)]
        frame: Option<u32>,

        /// Scale factor
        #[arg(long, default_value = "4")]
        scale: u32,

        /// Output PNG path
        #[arg(long, short, alias = "output")]
        out: PathBuf,
    },

    /// Convert AI-generated images to true 1:1 pixel art
    Convert {
        /// Input image(s) — file or directory of images
        input: PathBuf,

        /// Output directory (default: ./pixl_convert)
        #[arg(long, short, alias = "output", default_value = "pixl_convert")]
        out: PathBuf,

        /// Single-resolution mode: target width (skips presets)
        #[arg(long)]
        width: Option<u32>,

        /// Single-resolution mode: max palette colors
        #[arg(long, default_value = "32")]
        colors: u32,

        /// Preview upscale factor (e.g., 4 for 4x nearest-neighbor)
        #[arg(long)]
        preview: Option<u32>,
    },

    /// Import an image as a PAX backdrop (tile-decomposed animated background)
    BackdropImport {
        /// Input image (pixelized or raw — will be quantized)
        image: PathBuf,

        /// Scene name
        #[arg(long, default_value = "scene")]
        name: String,

        /// Max palette colors
        #[arg(long, default_value = "32")]
        colors: u32,

        /// Tile size for decomposition
        #[arg(long, default_value = "16")]
        tile_size: u32,

        /// Output .pax file
        #[arg(long, short, alias = "output")]
        out: PathBuf,
    },

    /// Render a backdrop from a .pax file (static or animated GIF)
    BackdropRender {
        /// Path to .pax file
        file: PathBuf,

        /// Backdrop name
        #[arg(long)]
        name: String,

        /// Output path (PNG for static, GIF for animated)
        #[arg(long, short, alias = "output")]
        out: PathBuf,

        /// Number of animation frames (0 = static)
        #[arg(long, default_value = "0")]
        frames: u32,

        /// Frame duration in ms (for animated)
        #[arg(long, default_value = "120")]
        duration: u32,

        /// Scale factor
        #[arg(long, default_value = "1")]
        scale: u32,
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
        #[arg(long, short, alias = "output")]
        out: PathBuf,

        /// Also output training pairs JSON for LoRA fine-tuning
        #[arg(long)]
        training: Option<PathBuf>,
    },

    /// Scan reference images and extract quality patches for ML training
    Scan {
        /// Input image or directory of images (sprite sheets, tilesets, etc.)
        input: PathBuf,

        /// Output directory for patches and manifest
        #[arg(long, short, alias = "output", default_value = "pixl_scan")]
        out: PathBuf,

        /// Patch size in pixels (default: 16)
        #[arg(long, default_value = "16")]
        patch_size: u32,

        /// Sliding window stride (default: same as patch_size)
        #[arg(long)]
        stride: Option<u32>,

        /// Minimum unique colors to keep a patch (default: 2)
        #[arg(long, default_value = "2")]
        min_colors: usize,

        /// Maximum background pixel ratio (default: 0.85)
        #[arg(long, default_value = "0.85")]
        max_bg: f64,

        /// Native tile size for grid-based tilesets (e.g., 32 for DCSS tiles)
        #[arg(long)]
        tile_size: Option<u32>,

        /// Save extracted patches as individual PNGs
        #[arg(long, default_value = "true")]
        save_patches: bool,

        /// Generate a contact sheet preview of the first 64 patches
        #[arg(long)]
        preview: bool,
    },

    /// Prepare training data from scanned patches
    Prepare {
        /// Path to scan output directory (containing scan_manifest.json)
        scan_dir: PathBuf,

        /// Output directory for training data (JSONL files)
        #[arg(long, short, alias = "output", default_value = "pixl_training")]
        out: PathBuf,

        /// Style tag for labels (e.g. "my-game", "retro-rpg")
        #[arg(long, default_value = "custom")]
        style: String,

        /// Augmentation level: 4 = rotations only, 8 = + flips
        #[arg(long, default_value = "4")]
        aug: u8,

        /// Enable warm/cool/dark color shift augmentation
        #[arg(long)]
        color_aug: bool,

        /// Max samples per stratification bin
        #[arg(long, default_value = "150")]
        max_per_bin: usize,

        /// Max palette colors per category
        #[arg(long, default_value = "10")]
        max_colors: usize,
    },

    /// Train a LoRA adapter on prepared data
    Train {
        /// Path to training data directory (containing train.jsonl)
        data_dir: PathBuf,

        /// Output adapter directory
        #[arg(long, default_value = "pixl_adapter")]
        adapter: PathBuf,

        /// Base model ID
        #[arg(long, default_value = "mlx-community/Qwen2.5-3B-Instruct-4bit")]
        model: String,

        /// Number of training epochs
        #[arg(long, default_value = "3")]
        epochs: usize,

        /// Learning rate
        #[arg(long, default_value = "0.00002")]
        lr: f64,

        /// Number of LoRA layers
        #[arg(long, default_value = "16")]
        layers: usize,

        /// Resume training from existing adapter
        #[arg(long)]
        resume: bool,

        /// Merge multiple data directories (comma-separated names, e.g. "eotb_optimal,matched")
        #[arg(long)]
        sources: Option<String>,

        /// Exclude specific datasets from merge
        #[arg(long)]
        exclude: Option<String>,
    },

    /// Compare two adapters side by side — generate the same tile with each
    Compare {
        /// .pax file for palette
        file: PathBuf,

        /// Prompt for generation
        #[arg(long)]
        prompt: String,

        /// First adapter path
        #[arg(long)]
        adapter_a: PathBuf,

        /// Second adapter path
        #[arg(long)]
        adapter_b: PathBuf,

        /// Output comparison image
        #[arg(long, short, alias = "output", default_value = "comparison.png")]
        out: PathBuf,

        /// Base model ID
        #[arg(long, default_value = "mlx-community/Qwen2.5-3B-Instruct-4bit")]
        model: String,
    },

    /// Blend two style latents and show the result
    BlendStyle {
        /// First .pax file (style A)
        file_a: PathBuf,

        /// Second .pax file (style B)
        file_b: PathBuf,

        /// Blend factor: 0.0 = pure A, 1.0 = pure B (default: 0.5)
        #[arg(long, default_value = "0.5")]
        blend: f64,
    },

    /// List available LoRA adapters
    Adapters {
        /// Directory to scan for adapters (default: training/adapters/)
        #[arg(long)]
        dir: Option<PathBuf>,
    },

    /// List available training datasets
    Datasets {
        /// Directory to scan (default: training/)
        #[arg(long)]
        dir: Option<PathBuf>,
    },

    /// Retrain adapter from feedback — export accepted tiles, prepare, and train
    Retrain {
        /// Path to .pax file (source of feedback + tiles)
        file: PathBuf,

        /// Output adapter directory
        #[arg(long, default_value = "training/adapters/retrained")]
        adapter: PathBuf,

        /// Style tag
        #[arg(long, default_value = "retrained")]
        style: String,

        /// Training epochs
        #[arg(long, default_value = "5")]
        epochs: usize,
    },

    /// Generate a coherent set of tiles (e.g., wall variants at multiple distances)
    GenerateSet {
        /// .pax file for palette context
        file: PathBuf,

        /// Set type: walls, enemies, items, floors
        #[arg(long, default_value = "walls")]
        set_type: String,

        /// Theme name for palette selection
        #[arg(long, default_value = "dark_fantasy")]
        theme: String,

        /// Number of variants to generate
        #[arg(long, default_value = "3")]
        count: usize,

        /// Output directory for generated assets
        #[arg(long, short, alias = "output", default_value = "pixl_generated")]
        out: PathBuf,

        /// LoRA adapter path
        #[arg(long)]
        adapter: Option<PathBuf>,

        /// Base model ID
        #[arg(long, default_value = "mlx-community/Qwen2.5-3B-Instruct-4bit")]
        model: String,
    },

    /// Start the MCP server (stdio transport)
    Mcp {
        /// Optional: pre-load a .pax file
        #[arg(long)]
        file: Option<PathBuf>,

        /// Base model for local inference (e.g. "mlx-community/Qwen2.5-3B-Instruct-4bit")
        #[arg(long)]
        model: Option<String>,

        /// Path to LoRA adapter directory (safetensors format)
        #[arg(long)]
        adapter: Option<PathBuf>,

        /// Port for the mlx_lm inference sidecar (default: 8099)
        #[arg(long, default_value = "8099")]
        inference_port: u16,
    },

    /// Start the HTTP API server (for PIXL Studio)
    Serve {
        /// Port to listen on
        #[arg(long, default_value = "3742")]
        port: u16,

        /// Optional: pre-load a .pax file
        #[arg(long)]
        file: Option<PathBuf>,

        /// Base model for local inference (e.g. "mlx-community/Qwen2.5-3B-Instruct-4bit")
        #[arg(long)]
        model: Option<String>,

        /// Path to LoRA adapter directory (safetensors format)
        #[arg(long)]
        adapter: Option<PathBuf>,

        /// Port for the mlx_lm inference sidecar (default: 8099)
        #[arg(long, default_value = "8099")]
        inference_port: u16,
    },

    /// Convert a .pax file to PAX-L compact format (stdout)
    Compact {
        /// Path to the .pax file
        file: PathBuf,

        /// Disable auto-stamp extraction
        #[arg(long)]
        no_stamps: bool,

        /// Disable =N row references
        #[arg(long)]
        no_row_refs: bool,

        /// Disable @fill pattern detection
        #[arg(long)]
        no_fill: bool,
    },

    /// Convert PAX-L from stdin to .pax TOML format (stdout)
    Expand {
        /// Strict parsing mode (reject structural errors)
        #[arg(long)]
        strict: bool,
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
        #[arg(long, short, default_value = "project.pixlproject", alias = "output")]
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
        Commands::Validate {
            file,
            check_edges,
            completeness,
            check_seams,
            quality,
        } => {
            cmd_validate(&file, check_edges, check_seams, quality);
            if completeness {
                cmd_completeness(&file);
            }
        }
        Commands::Check {
            file,
            fix,
            subcomplete,
        } => {
            if fix {
                cmd_check_fix(&file);
            } else if subcomplete {
                cmd_subcomplete(&file);
            } else {
                cmd_validate(&file, true, false, false);
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
        Commands::Critique { file, tile } => {
            cmd_critique(&file, &tile);
        }
        Commands::Upscale {
            file,
            tile,
            factor,
            out,
        } => {
            cmd_upscale(&file, &tile, factor, &out);
        }
        Commands::GenerateSprite {
            file,
            prompt,
            name,
            size,
            dither,
            max_colors,
            out,
        } => {
            cmd_generate_sprite(&file, &prompt, &name, &size, dither, max_colors, &out);
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
            weight,
            pin,
            format,
        } => {
            cmd_narrate(
                &file, width, height, seed, &rule, &out, &weight, &pin, &format,
            );
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
        Commands::New {
            theme,
            out,
            generate,
        } => {
            if generate {
                cmd_new_generate(&theme, &out);
            } else {
                cmd_new(&theme, &out);
            }
        }
        Commands::Export { file, format, out } => {
            cmd_export(&file, &format, &out);
        }
        Commands::RenderSprite {
            file,
            spriteset,
            sprite,
            scale,
            out,
        } => {
            cmd_render_sprite(&file, &spriteset, &sprite, scale, &out);
        }
        Commands::RenderComposite {
            file,
            composite,
            variant,
            anim,
            frame,
            scale,
            out,
        } => {
            cmd_render_composite(
                &file,
                &composite,
                variant.as_deref(),
                anim.as_deref(),
                frame,
                scale,
                &out,
            );
        }
        Commands::Convert {
            input,
            out,
            width,
            colors,
            preview,
        } => {
            cmd_convert(&input, &out, width, colors, preview);
        }
        Commands::BackdropImport {
            image,
            name,
            colors,
            tile_size,
            out,
        } => {
            cmd_backdrop_import(&image, &name, colors, tile_size, &out);
        }
        Commands::BackdropRender {
            file,
            name,
            out,
            frames,
            duration,
            scale,
        } => {
            cmd_backdrop_render(&file, &name, &out, frames, duration, scale);
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
        Commands::Mcp {
            file,
            model,
            adapter,
            inference_port,
        } => {
            let inf = build_inference_config(model, adapter, inference_port);
            cmd_mcp(file.as_deref(), inf);
        }
        Commands::Serve {
            port,
            file,
            model,
            adapter,
            inference_port,
        } => {
            let inf = build_inference_config(model, adapter, inference_port);
            cmd_serve(port, file.as_deref(), inf);
        }
        Commands::Scan {
            input,
            out,
            patch_size,
            stride,
            min_colors,
            max_bg,
            tile_size,
            save_patches,
            preview,
        } => {
            cmd_scan(&input, &out, patch_size, stride, min_colors, max_bg, tile_size, save_patches, preview);
        }
        Commands::Prepare {
            scan_dir,
            out,
            style,
            aug,
            color_aug,
            max_per_bin,
            max_colors,
        } => {
            cmd_prepare(&scan_dir, &out, &style, aug, color_aug, max_per_bin, max_colors);
        }
        Commands::Train {
            data_dir,
            adapter,
            model,
            epochs,
            lr,
            layers,
            resume,
            sources,
            exclude,
        } => {
            cmd_train(&data_dir, &adapter, &model, epochs, lr, layers, resume, sources.as_deref(), exclude.as_deref());
        }
        Commands::Compare {
            file,
            prompt,
            adapter_a,
            adapter_b,
            out,
            model,
        } => {
            cmd_compare(&file, &prompt, &adapter_a, &adapter_b, &out, &model);
        }
        Commands::BlendStyle { file_a, file_b, blend } => {
            cmd_blend_style(&file_a, &file_b, blend);
        }
        Commands::Adapters { dir } => {
            cmd_adapters(dir.as_deref());
        }
        Commands::Datasets { dir } => {
            cmd_datasets(dir.as_deref());
        }
        Commands::Retrain {
            file,
            adapter,
            style,
            epochs,
        } => {
            cmd_retrain(&file, &adapter, &style, epochs);
        }
        Commands::GenerateSet {
            file,
            set_type,
            theme,
            count,
            out,
            adapter,
            model,
        } => {
            cmd_generate_set(&file, &set_type, &theme, count, &out, adapter.as_deref(), &model);
        }
        Commands::Compact {
            file,
            no_stamps: _,
            no_row_refs,
            no_fill,
        } => {
            cmd_compact(&file, no_row_refs, no_fill);
        }
        Commands::Expand { strict } => {
            cmd_expand(strict);
        }
    }
}

fn build_inference_config(
    model: Option<String>,
    adapter: Option<PathBuf>,
    inference_port: u16,
) -> Option<pixl_mcp::inference::InferenceConfig> {
    // Also check env vars as fallback
    let model = model.or_else(|| std::env::var("PIXL_MODEL").ok());
    let adapter = adapter.or_else(|| std::env::var("PIXL_ADAPTER").ok().map(PathBuf::from));

    model.map(|m| pixl_mcp::inference::InferenceConfig {
        model: m,
        adapter_path: adapter,
        port: inference_port,
        ..Default::default()
    })
}

fn cmd_subcomplete(file: &PathBuf) {
    let source = std::fs::read_to_string(file).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {}", file.display(), e);
        std::process::exit(1);
    });
    let pax = pixl_core::parser::parse_pax(&source).unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    });
    let report = pixl_core::completeness::check_subcomplete(&pax);
    println!("{}", report.summary);
    if !report.is_subcomplete {
        std::process::exit(1);
    }
}

fn cmd_compact(file: &PathBuf, no_row_refs: bool, no_fill: bool) {
    let source = std::fs::read_to_string(file).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {}", file.display(), e);
        std::process::exit(1);
    });
    let pax = pixl_core::parser::parse_pax(&source).unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    });
    let config = pixl_core::paxl::PaxlConfig {
        row_refs: !no_row_refs,
        fill_detect: !no_fill,
        ..Default::default()
    };
    match pixl_core::paxl::to_paxl(&pax, &config) {
        Ok(paxl) => print!("{}", paxl),
        Err(e) => {
            eprintln!("Serialization error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_expand(strict: bool) {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .unwrap_or_else(|e| {
            eprintln!("Error reading stdin: {}", e);
            std::process::exit(1);
        });
    match pixl_core::paxl::from_paxl(&input, strict) {
        Ok((pax_file, warnings)) => {
            for w in &warnings {
                eprintln!("warning: {}", w);
            }
            match toml::to_string_pretty(&pax_file) {
                Ok(toml_str) => print!("{}", toml_str),
                Err(e) => {
                    eprintln!("TOML serialization error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Parse error: {}", e);
            std::process::exit(1);
        }
    }
}

#[tokio::main]
async fn cmd_mcp_async(
    file: Option<&std::path::Path>,
    inference: Option<pixl_mcp::inference::InferenceConfig>,
) {
    let result = match (file, inference) {
        (_, Some(config)) => {
            pixl_mcp::server::run_stdio_with_inference(
                file.map(|p| p.to_string_lossy().as_ref().to_owned())
                    .as_deref(),
                config,
            )
            .await
        }
        (Some(path), None) => pixl_mcp::server::run_stdio_with_file(&path.to_string_lossy()).await,
        (None, None) => pixl_mcp::server::run_stdio().await,
    };

    if let Err(e) = result {
        eprintln!("MCP server error: {}", e);
        process::exit(1);
    }
}

fn cmd_mcp(
    file: Option<&std::path::Path>,
    inference: Option<pixl_mcp::inference::InferenceConfig>,
) {
    cmd_mcp_async(file, inference);
}

fn cmd_serve(
    port: u16,
    file: Option<&std::path::Path>,
    inference: Option<pixl_mcp::inference::InferenceConfig>,
) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let state = if let Some(path) = file {
            pixl_mcp::state::McpState::from_path(path).unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                process::exit(1);
            })
        } else {
            pixl_mcp::state::McpState::new()
        };

        if let Err(e) = pixl_mcp::http::run_http(state, port, inference).await {
            eprintln!("server error: {}", e);
            process::exit(1);
        }
    });
}

fn cmd_validate(file: &PathBuf, check_edges: bool, check_seams: bool, quality: bool) {
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

    let result = if quality {
        // Load knowledge base (best-effort — missing file is fine)
        let kb_path = std::path::Path::new("knowledge/pixelart-knowledge-base.json");
        let kb = if kb_path.exists() {
            pixl_core::knowledge::KnowledgeBase::load(kb_path)
        } else {
            None
        };
        pixl_core::validate::validate_quality(&pax_file, check_edges, kb.as_ref())
    } else {
        pixl_core::validate::validate(&pax_file, check_edges)
    };

    // Print stats
    println!(
        "pixl validate: {} palettes, {} themes, {} stamps, {} tiles, {} sprites, {} composites",
        result.stats.palettes,
        result.stats.themes,
        result.stats.stamps,
        result.stats.tiles,
        result.stats.sprites,
        result.stats.composites,
    );

    // Print warnings
    for w in &result.warnings {
        println!("  warning: {}", w);
    }

    // Print errors
    for e in &result.errors {
        eprintln!("  error: {}", e);
    }

    // Seam checking (requires resolved tiles)
    if check_seams && !pax_file.composite.is_empty() {
        let palettes = pixl_core::parser::resolve_all_palettes(&pax_file).unwrap_or_default();
        let empty_stamps = std::collections::HashMap::new();
        let tiles = resolve_all_tiles(&pax_file, &palettes, &empty_stamps);

        let seam_warnings = pixl_core::validate::check_seams(&pax_file, &tiles);
        for w in &seam_warnings {
            println!("  seam: {}", w);
        }
        if seam_warnings.is_empty() {
            println!("seams: all composite seams are continuous.");
        } else {
            println!("{} seam warning(s)", seam_warnings.len());
        }
    }

    // Quality report
    if let Some(ref qr) = result.quality {
        let issues_count: usize = qr
            .tile_reports
            .iter()
            .filter(|r| !r.structural.issues.is_empty())
            .count();
        println!();
        println!(
            "quality: {} tiles analyzed, {} with issues",
            qr.tiles_analyzed, issues_count,
        );

        for tr in &qr.tile_reports {
            if tr.structural.issues.is_empty() {
                continue;
            }

            let verdict = if pixl_core::structural::has_errors(&tr.structural) {
                "\x1b[31mREJECT\x1b[0m"
            } else if pixl_core::structural::has_warnings(&tr.structural) {
                "\x1b[33mREFINE\x1b[0m"
            } else {
                "\x1b[32mACCEPT\x1b[0m"
            };

            println!();
            println!("  {}: {}", tr.tile_name, verdict);

            for issue in &tr.structural.issues {
                let (icon, color) = match issue.severity {
                    pixl_core::structural::Severity::Error => ("✗", "\x1b[31m"),
                    pixl_core::structural::Severity::Warning => ("!", "\x1b[33m"),
                    pixl_core::structural::Severity::Info => ("·", "\x1b[36m"),
                };
                println!("    {}{} {}\x1b[0m", color, icon, issue.message);

                // Show KB advice for this issue
                for advice in tr.kb_advice.iter().filter(|a| a.issue_code == issue.code) {
                    println!(
                        "      \x1b[2m→ \"{}\"\x1b[0m",
                        truncate_advice(&advice.summary, 120),
                    );
                    println!("        \x1b[2m({})\x1b[0m", advice.source_title);
                }
            }
        }

        // Style consistency
        if let Some(ref sc) = qr.consistency {
            let consistency_pct =
                (1.0 - sc.light_direction_stddev.min(1.0)) * 100.0;
            println!();
            println!(
                "  style consistency: {:.0}% (light direction mean={:.2}, stddev={:.2})",
                consistency_pct, sc.mean_light_direction, sc.light_direction_stddev,
            );
            for outlier in &sc.outliers {
                println!(
                    "    \x1b[33m! {} light_direction={:.2} deviates from mean {:.2} (Δ{:.2})\x1b[0m",
                    outlier.tile_name,
                    outlier.light_direction,
                    sc.mean_light_direction,
                    outlier.deviation,
                );
            }
        }
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

/// Truncate advice text to a maximum length, adding ellipsis if needed.
fn truncate_advice(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

fn cmd_completeness(file: &PathBuf) {
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

    let report = pixl_core::completeness::analyze(&pax_file);

    println!("Tileset completeness: {:.0}%", report.score * 100.0);
    println!("Edge classes: {}", report.edge_classes.join(", "));
    println!("Connected pairs: {}", report.connected_pairs.len());

    if report.disconnected_pairs.is_empty() {
        println!("All edge classes are connected — WFC can reach every terrain type.");
    } else {
        println!("Disconnected pairs: {}", report.disconnected_pairs.len());
        for (a, b) in &report.disconnected_pairs {
            println!("  {} <-> {} (no transition tile)", a, b);
        }
        println!();
        println!("Recommended tiles to add:");
        for mt in &report.missing_tiles {
            println!(
                "  {} — edges: n={}, e={}, s={}, w={} (auto_rotate=4way)",
                mt.name, mt.edge_class.n, mt.edge_class.e, mt.edge_class.s, mt.edge_class.w,
            );
            println!("    {}", mt.reason);
        }
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
            .map(|c| {
                if c.is_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
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
        (
            "dark_fantasy",
            include_str!("../../../themes/dark_fantasy.pax"),
        ),
        (
            "light_fantasy",
            include_str!("../../../themes/light_fantasy.pax"),
        ),
        ("sci_fi", include_str!("../../../themes/sci_fi.pax")),
        ("nature", include_str!("../../../themes/nature.pax")),
        ("gameboy", include_str!("../../../themes/gameboy.pax")),
        ("nes", include_str!("../../../themes/nes.pax")),
        ("snes", include_str!("../../../themes/snes.pax")),
        ("gba", include_str!("../../../themes/gba.pax")),
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

fn cmd_new_generate(theme: &str, out: &PathBuf) {
    // Step 1: Write the static template (palette + theme + stamps, no tiles)
    cmd_new(theme, out);

    // Step 2: Load the template and knowledge base
    let source = std::fs::read_to_string(out).unwrap_or_default();
    let pax_file = match pixl_core::parser::parse_pax(&source) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error parsing template: {}", e);
            process::exit(1);
        }
    };

    // Find knowledge base
    let kb_path = std::path::Path::new("knowledge/pixelart-knowledge-base.json");
    let kb = if kb_path.exists() {
        pixl_core::knowledge::KnowledgeBase::load(kb_path)
    } else {
        None
    };

    // Step 3: Get palette info
    let palette_name = pax_file
        .theme
        .values()
        .next()
        .map(|t| t.palette.as_str())
        .unwrap_or("unknown");
    let palette_info = pax_file
        .palette
        .iter()
        .next()
        .map(|(_, pal_raw)| {
            pal_raw
                .iter()
                .map(|(sym, hex)| format!("'{}' = {}", sym, hex))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    let canvas = pax_file
        .theme
        .values()
        .next()
        .and_then(|t| t.canvas)
        .unwrap_or(16);

    let light_source = pax_file
        .theme
        .values()
        .next()
        .and_then(|t| t.light_source.as_deref())
        .unwrap_or("top-left");

    // Step 4: Generate prompts for each tile type
    let tile_specs = [
        (
            "wall_solid",
            "solid/solid/solid/solid",
            "A solid wall tile with brick or stone pattern. Use highlight on top-left edges, shadow on bottom-right. Full mortar lines between blocks.",
        ),
        (
            "floor_stone",
            "floor/floor/floor/floor",
            "A walkable floor tile. Use irregular stone sizes, scattered highlights, mortar gaps. Should look different from wall.",
        ),
        (
            "floor_variant",
            "floor/floor/floor/floor",
            "A second floor variant — cracked, mossy, or decorated. Should tile seamlessly with floor_stone.",
        ),
        (
            "wall_floor_n",
            "solid/solid/floor/solid",
            "A wall-to-floor transition tile. Top half wall pattern, bottom half floor pattern. Dither the boundary with 2-3 rows of blended pixels. auto_rotate=4way gives all 4 cardinal transitions.",
        ),
        (
            "wall_corner_ne",
            "solid/solid/floor/floor",
            "A corner tile where wall (top-right) meets floor (bottom-left). Diagonal boundary with dithered edge. auto_rotate=4way gives all 4 corners.",
        ),
        (
            "door_ns",
            "floor/solid/floor/solid",
            "A door/passage tile allowing movement through walls. Floor on north/south, solid wall on east/west. auto_rotate=4way gives both orientations.",
        ),
    ];

    println!();
    println!("# AI Generation Prompts for theme '{}'", theme);
    println!("# Palette: {}", palette_name);
    println!("# Canvas: {}x{}", canvas, canvas);
    println!("# Light source: {}", light_source);
    println!("# Pipe each prompt to an LLM to generate tile grids");
    println!();

    for (name, edges, description) in &tile_specs {
        // Search knowledge base for relevant techniques
        let knowledge = if let Some(ref kb) = kb {
            let query = format!("{} {} pixel art tile", description, theme);
            let results = kb.search(&query, 3);
            results
                .iter()
                .map(|r| format!("[{}] {}", r.source_title, r.content))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            String::new()
        };

        println!("--- {} (edges: {}) ---", name, edges);
        println!(
            "SYSTEM: You are a pixel art tile designer. Output ONLY a {}x{} character grid.",
            canvas, canvas
        );
        println!("Palette ({}): {}", palette_name, palette_info);
        println!(
            "Light source: {}. Use highlight symbols on lit edges, shadow on dark edges.",
            light_source
        );
        println!();
        if !knowledge.is_empty() {
            println!("KNOWLEDGE:");
            println!("{}", knowledge);
            println!();
        }
        println!("USER: {}", description);
        println!();
    }

    println!(
        "# To use: pipe each section to an LLM, extract the {}x{} grid,",
        canvas, canvas
    );
    println!(
        "# and add it to {} as [tile.NAME] with the specified edge_class.",
        out.display()
    );
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
            // Check for mismatch and auto-update stale edge classes
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
                // Update the stale edge class to match current grid
                tile_raw.edge_class = Some(pixl_core::types::EdgeClassRaw {
                    n: auto.n.clone(),
                    e: auto.e.clone(),
                    s: auto.s.clone(),
                    w: auto.w.clone(),
                });
                println!(
                    "  updated: '{}' — edge mismatch fixed: {}",
                    name,
                    mismatches.join(", ")
                );
                fixed += 1;
                warned += 1;
            }
        }
    }

    if fixed > 0 {
        // Remove stale edge_class sections from source, then append fresh ones
        let mut new_source = source.clone();
        let mut appendix = String::new();

        for (name, tile_raw) in &pax_file.tile {
            if tile_raw.template.is_some() || tile_raw.grid.is_none() {
                continue;
            }
            if let Some(ref ec) = tile_raw.edge_class {
                let marker = format!("[tile.{}.edge_class]", name);

                // Remove existing edge_class section if present
                if let Some(start) = new_source.find(&marker) {
                    // Find end of section: next [section] or end of file
                    let after_marker = start + marker.len();
                    let end = new_source[after_marker..]
                        .find("\n[")
                        .map(|pos| after_marker + pos)
                        .unwrap_or(new_source.len());
                    // Also trim any leading blank lines before the section
                    let trimmed_start = new_source[..start]
                        .rfind(|c: char| c != '\n' && c != '\r')
                        .map(|pos| pos + 1)
                        .unwrap_or(start);
                    new_source = format!("{}{}", &new_source[..trimmed_start], &new_source[end..]);
                }

                // Append fresh edge_class
                appendix.push_str(&format!(
                    "\n[tile.{}.edge_class]\nn = \"{}\"\ne = \"{}\"\ns = \"{}\"\nw = \"{}\"\n",
                    name, ec.n, ec.e, ec.s, ec.w
                ));
            }
        }

        if !appendix.is_empty() {
            let final_source = format!("{}\n{}", new_source.trim_end(), appendix);
            if let Err(e) = std::fs::write(file, &final_source) {
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
    let mut collision_map: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for (name, tile_raw) in &pax_file.tile {
        if tile_raw.template.is_some() || tile_raw.grid.is_none() {
            continue;
        }
        let size_str = tile_raw.size.as_deref().unwrap_or("16x16");
        let (w, h) = match pixl_core::types::parse_size(size_str) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if let Ok(grid) =
            pixl_core::grid::parse_grid(tile_raw.grid.as_ref().unwrap(), w, h, palette)
        {
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
        let sorted_grids: Vec<Vec<Vec<char>>> =
            order.iter().map(|&i| tile_grids[i].clone()).collect();
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
                    println!(
                        "texturepacker: {} -> {}",
                        atlas_path.display(),
                        json_path.display()
                    );
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

                    // Export tilemaps with multi-layer object splitting
                    let object_defs: std::collections::HashMap<String, &pixl_core::types::ObjectRaw> =
                        pax_file.object.iter().map(|(k, v)| (k.clone(), v)).collect();

                    for (tm_name, tilemap_raw) in &pax_file.tilemap {
                        let terrain_grid = match pixl_export::tiled::resolve_terrain_grid(
                            tilemap_raw,
                            &tile_names,
                        ) {
                            Some(g) => g,
                            None => continue,
                        };

                        let placements: Vec<(String, u32, u32)> = tilemap_raw
                            .objects
                            .iter()
                            .map(|p| (p.object.clone(), p.x, p.y))
                            .collect();

                        match pixl_export::tiled::generate_map_with_objects(
                            &terrain_grid,
                            &tile_names,
                            &placements,
                            &object_defs,
                            tile_size.0,
                            tile_size.1,
                            "tileset.tsj",
                        ) {
                            Ok(map) => {
                                let tmj_path = out_dir.join(format!("{}.tmj", tm_name));
                                let tmj_str = serde_json::to_string_pretty(&map).unwrap();
                                std::fs::write(&tmj_path, tmj_str).unwrap();
                                println!("tiled map: {}", tmj_path.display());
                            }
                            Err(e) => {
                                eprintln!("warning: tilemap '{}': {}", tm_name, e);
                            }
                        }
                    }
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
            eprintln!(
                "error: unknown format '{}'. Supported: texturepacker, tiled, godot",
                format
            );
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

/// Ensure parent directory exists for an output file path.
fn ensure_parent_dir(out: &PathBuf) {
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                eprintln!("error: cannot create directory {}: {}", parent.display(), e);
                process::exit(1);
            });
        }
    }
}

/// Resolve all tiles in a PaxFile to their full grids, returning a map of Tile structs.
fn resolve_all_tiles(
    pax_file: &pixl_core::types::PaxFile,
    palettes: &std::collections::HashMap<String, pixl_core::types::Palette>,
    stamps: &std::collections::HashMap<String, pixl_core::types::Stamp>,
) -> std::collections::HashMap<String, pixl_core::types::Tile> {
    let mut tiles = std::collections::HashMap::new();
    for (name, _tile_raw) in &pax_file.tile {
        if let Ok((grid, w, h)) = pixl_core::resolve::resolve_tile_grid(
            name,
            &pax_file.tile,
            palettes,
            stamps,
        ) {
            tiles.insert(
                name.clone(),
                pixl_core::types::Tile {
                    name: name.clone(),
                    palette: _tile_raw.palette.clone(),
                    width: w,
                    height: h,
                    encoding: pixl_core::types::Encoding::Grid,
                    symmetry: pixl_core::types::Symmetry::None,
                    auto_rotate: pixl_core::types::AutoRotate::None,
                    edge_class: pixl_core::types::EdgeClass {
                        n: String::new(),
                        e: String::new(),
                        s: String::new(),
                        w: String::new(),
                    },
                    tags: vec![],
                    target_layer: None,
                    weight: 1.0,
                    palette_swaps: vec![],
                    cycles: vec![],
                    nine_slice: None,
                    visual_height_extra: None,
                    semantic: None,
                    grid,
                },
            );
        }
    }
    tiles
}

fn cmd_render_composite(
    file: &PathBuf,
    composite_name: &str,
    variant: Option<&str>,
    anim_name: Option<&str>,
    frame: Option<u32>,
    scale: u32,
    out: &PathBuf,
) {
    let (pax_file, palettes) = load_pax(file);

    let raw = match pax_file.composite.get(composite_name) {
        Some(r) => r,
        None => {
            eprintln!("error: composite '{}' not found", composite_name);
            let names: Vec<&String> = pax_file.composite.keys().collect();
            if names.is_empty() {
                eprintln!("  (no composites defined in this file)");
            } else {
                eprintln!("  available: {}", names.iter().map(|n| n.as_str()).collect::<Vec<_>>().join(", "));
            }
            process::exit(1);
        }
    };

    let composite = match pixl_core::composite::resolve_composite(raw, composite_name) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error resolving composite '{}': {}", composite_name, e);
            process::exit(1);
        }
    };

    // Find palette from first non-void tile in the layout
    let palette_name = composite
        .slots
        .iter()
        .flat_map(|row| row.iter())
        .find(|s| s.name != "_")
        .and_then(|s| pax_file.tile.get(&s.name))
        .map(|t| t.palette.as_str())
        .unwrap_or_else(|| {
            eprintln!("error: no tiles found in composite layout");
            process::exit(1);
        });

    let palette = match palettes.get(palette_name) {
        Some(p) => p,
        None => {
            eprintln!("error: palette '{}' not found", palette_name);
            process::exit(1);
        }
    };

    let empty_stamps = std::collections::HashMap::new();
    let tiles = resolve_all_tiles(&pax_file, &palettes, &empty_stamps);

    let img = match pixl_render::renderer::render_composite(
        &composite,
        variant,
        anim_name,
        frame,
        &tiles,
        palette,
        scale,
    ) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("error composing '{}': {}", composite_name, e);
            process::exit(1);
        }
    };

    ensure_parent_dir(out);
    if let Err(e) = img.save(out) {
        eprintln!("error: cannot write {}: {}", out.display(), e);
        process::exit(1);
    }

    let label = match (variant, anim_name, frame) {
        (Some(v), _, _) => format!("{}:{}", composite_name, v),
        (_, Some(a), Some(f)) => format!("{}:{}:{}", composite_name, a, f),
        (_, Some(a), None) => format!("{}:{}", composite_name, a),
        _ => composite_name.to_string(),
    };
    println!(
        "rendered composite '{}' ({}x{} @{}x) -> {}",
        label,
        composite.width,
        composite.height,
        scale,
        out.display()
    );
}

fn cmd_generate_sprite(
    file: &PathBuf,
    prompt: &str,
    name: &str,
    size_str: &str,
    dither: bool,
    max_colors: u32,
    out: &PathBuf,
) {
    let (_pax_file, palettes) = load_pax(file);

    let palette_name = palettes.keys().next().unwrap_or_else(|| {
        eprintln!("error: no palettes found in {}", file.display());
        process::exit(1);
    });
    let session_palette = &palettes[palette_name];

    let (target_w, target_h) = if size_str == "auto" {
        (None, None)
    } else {
        match pixl_core::types::parse_size(size_str) {
            Ok((w, h)) => (Some(w), Some(h)),
            Err(e) => {
                eprintln!("error: {}", e);
                process::exit(1);
            }
        }
    };

    let config = match pixl_mcp::diffusion::DiffusionConfig::from_env() {
        Some(c) => c,
        None => {
            eprintln!("error: OPENAI_API_KEY not set");
            process::exit(1);
        }
    };

    let size_display = match (target_w, target_h) {
        (Some(w), Some(h)) => format!("{}x{}", w, h),
        _ => "auto".to_string(),
    };
    println!(
        "generating sprite '{}' via {} ({}, {} colors)...",
        name, config.model, size_display, max_colors
    );
    println!("prompt: \"{}\"", prompt);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        pixl_mcp::diffusion::generate_with_auto_palette(
            &config, prompt, target_w, target_h, max_colors, dither,
        )
        .await
    });

    let result = match result {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    let render_palette = result.extracted_palette.as_ref().unwrap_or(session_palette);

    println!(
        "generated {}x{} (detected {}px blocks → native {}x{}) → target {}x{} ({:.1}% accuracy, {} clipped)",
        result.generated_size.0,
        result.generated_size.1,
        result.detected_pixel_size,
        result.native_resolution.0,
        result.native_resolution.1,
        result.width,
        result.height,
        result.color_accuracy * 100.0,
        result.clipped_colors,
    );

    // Save reference image
    let ref_path = out.with_file_name(format!(
        "{}_reference.png",
        out.file_stem().unwrap_or_default().to_string_lossy()
    ));
    ensure_parent_dir(&ref_path);
    if let Err(e) = std::fs::write(&ref_path, &result.reference_png) {
        eprintln!("warning: cannot save reference: {}", e);
    } else {
        println!("reference -> {}", ref_path.display());
    }

    // Render quantized preview
    let img = pixl_render::renderer::render_grid(&result.grid, render_palette, 8);
    ensure_parent_dir(out);
    if let Err(e) = img.save(out) {
        eprintln!("error: cannot write {}: {}", out.display(), e);
        process::exit(1);
    }
    println!("quantized -> {}", out.display());

    // Print extracted palette TOML if auto_palette
    if let Some(ref toml) = result.palette_toml {
        println!("\nExtracted palette:");
        println!("{}", toml);
    }

    // Print the grid for copy-paste into .pax
    println!("PAX grid ({}x{}):", result.width, result.height);
    println!("'''");
    println!("{}", result.grid_string);
    println!("'''");

    // Structural critique
    let report = pixl_core::structural::analyze(&result.grid, render_palette, '.');
    let critique = pixl_core::structural::critique_text(&report, name);
    println!("\n{}", critique);
}

fn cmd_upscale(file: &PathBuf, tile_name: &str, factor: u32, out: &PathBuf) {
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

    let empty_stamps = std::collections::HashMap::new();
    let (grid, w, h) = match pixl_core::resolve::resolve_tile_grid(
        tile_name,
        &pax_file.tile,
        &palettes,
        &empty_stamps,
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    let (upscaled, grid_str, new_w, new_h) = pixl_core::upscale::upscale_tile_grid(&grid, factor);

    // Render
    let img = pixl_render::renderer::render_grid(&upscaled, palette, 4);
    ensure_parent_dir(out);
    if let Err(e) = img.save(out) {
        eprintln!("error: cannot write {}: {}", out.display(), e);
        process::exit(1);
    }

    println!(
        "upscaled '{}' {}x{} → {}x{} (factor {}) -> {}",
        tile_name, w, h, new_w, new_h, factor, out.display()
    );

    // Print the upscaled grid for copy-paste into a .pax file
    println!("\nUpscaled grid ({new_w}x{new_h}):");
    println!("'''");
    println!("{}", grid_str);
    println!("'''");

    // Run critique on upscaled result
    let report = pixl_core::structural::analyze(&upscaled, palette, '.');
    if !report.issues.is_empty() {
        println!();
        for issue in &report.issues {
            let prefix = match issue.severity {
                pixl_core::structural::Severity::Error => "ERROR",
                pixl_core::structural::Severity::Warning => "WARN",
                pixl_core::structural::Severity::Info => "INFO",
            };
            println!("  [{}] {}", prefix, issue.message);
        }
    }
}

fn cmd_critique(file: &PathBuf, tile_name: &str) {
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

    let empty_stamps = std::collections::HashMap::new();
    let (grid, w, h) = match pixl_core::resolve::resolve_tile_grid(
        tile_name,
        &pax_file.tile,
        &palettes,
        &empty_stamps,
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    let report = pixl_core::structural::analyze(&grid, palette, '.');

    println!("pixl critique: '{}' ({}x{})", tile_name, w, h);
    println!();

    // Metrics
    println!("  Outline coverage:    {:.1}%", report.outline_coverage * 100.0);
    println!("  Centering:           {:.1}%", report.centering_score * 100.0);
    println!("  Canvas utilization:  {:.1}%", report.canvas_utilization * 100.0);
    println!("  Mean contrast:       {:.4}", report.mean_adjacent_contrast);
    println!("  Pixel density:       {:.1}%", report.pixel_density * 100.0);
    println!("  Components:          {}", report.connected_components);
    println!(
        "  Bounding box:        ({},{}) to ({},{})",
        report.bounding_box.0, report.bounding_box.1,
        report.bounding_box.2, report.bounding_box.3,
    );
    println!();

    if report.issues.is_empty() {
        println!("  \x1b[32m✓ No structural issues found.\x1b[0m");
    } else {
        for issue in &report.issues {
            let (icon, color) = match issue.severity {
                pixl_core::structural::Severity::Error => ("✗", "\x1b[31m"),
                pixl_core::structural::Severity::Warning => ("!", "\x1b[33m"),
                pixl_core::structural::Severity::Info => ("·", "\x1b[36m"),
            };
            println!("  {}{} {}\x1b[0m", color, icon, issue.message);
        }
    }
    println!();

    if pixl_core::structural::has_errors(&report) {
        println!("  Verdict: \x1b[31mREJECT\x1b[0m — regenerate this tile.");
        process::exit(1);
    } else if pixl_core::structural::has_warnings(&report) {
        println!("  Verdict: \x1b[33mREFINE\x1b[0m — fix the issues above.");
    } else {
        println!("  Verdict: \x1b[32mACCEPT\x1b[0m");
    }
}

fn cmd_render(file: &PathBuf, tile_name: &str, scale: u32, out: &PathBuf) {
    let (pax_file, palettes) = load_pax(file);

    // Look up tile directly, or fall back to base name for rotated variants
    let tile_raw = match pax_file.tile.get(tile_name) {
        Some(t) => t,
        None => {
            // Try stripping rotation suffix to find base tile
            pixl_core::resolve::base_tile_name(tile_name)
                .and_then(|base| pax_file.tile.get(base))
                .unwrap_or_else(|| {
                    eprintln!("error: tile '{}' not found", tile_name);
                    process::exit(1);
                })
        }
    };

    let palette = match palettes.get(&tile_raw.palette) {
        Some(p) => p,
        None => {
            eprintln!("error: palette '{}' not found", tile_raw.palette);
            process::exit(1);
        }
    };

    // Use unified resolver — handles grid, RLE, compose, template, symmetry, rotation
    let (full_grid, w, h) = match pixl_core::resolve::resolve_tile_grid(
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

    let img = pixl_render::renderer::render_grid(&full_grid, palette, scale);

    ensure_parent_dir(out);
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

    // Compose composites (base + variants + animation frames) into AtlasTile entries
    let empty_stamps = std::collections::HashMap::new();
    let resolved_tiles = resolve_all_tiles(&pax_file, &palettes, &empty_stamps);
    let mut composite_atlas_tiles: Vec<pixl_render::atlas::AtlasTile> = Vec::new();

    for (cname, raw) in &pax_file.composite {
        if let Ok(comp) = pixl_core::composite::resolve_composite(raw, cname) {
            // Base layout
            if let Ok(grid) =
                pixl_core::composite::compose_grid(&comp, None, None, &resolved_tiles, '.')
            {
                composite_atlas_tiles.push(pixl_render::atlas::AtlasTile {
                    name: cname.clone(),
                    width: comp.width,
                    height: comp.height,
                    grid,
                });
            }

            // Variants
            for vname in comp.variants.keys() {
                if let Ok(grid) = pixl_core::composite::compose_grid(
                    &comp,
                    Some(vname),
                    None,
                    &resolved_tiles,
                    '.',
                ) {
                    composite_atlas_tiles.push(pixl_render::atlas::AtlasTile {
                        name: format!("{}:{}", cname, vname),
                        width: comp.width,
                        height: comp.height,
                        grid,
                    });
                }
            }

            // Animation frames
            for (aname, anim) in &comp.animations {
                for frame in &anim.frames {
                    if let Ok(grid) = pixl_core::composite::compose_anim_frame(
                        &comp,
                        aname,
                        frame.index,
                        None,
                        &resolved_tiles,
                        '.',
                    ) {
                        composite_atlas_tiles.push(pixl_render::atlas::AtlasTile {
                            name: format!("{}:{}:{}", cname, aname, frame.index),
                            width: comp.width,
                            height: comp.height,
                            grid,
                        });
                    }
                }
            }
        }
    }

    if atlas_tiles.is_empty() && composite_atlas_tiles.is_empty() {
        eprintln!("error: no resolvable tiles found");
        process::exit(1);
    }

    // Use first tile's palette for rendering
    let first_palette_name = pax_file
        .tile
        .values()
        .next()
        .map(|t| t.palette.as_str())
        .unwrap_or_else(|| {
            // Fall back to first composite's tile palette
            pax_file
                .composite
                .values()
                .next()
                .and_then(|c| {
                    let layout_lines: Vec<&str> = c.layout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
                    layout_lines.first().and_then(|line| {
                        line.split_whitespace().next().and_then(|token| {
                            let ref_name = pixl_core::types::TileRef::parse(token).name;
                            pax_file.tile.get(&ref_name).map(|t| t.palette.as_str())
                        })
                    })
                })
                .unwrap_or("")
        });
    let palette = &palettes[first_palette_name];

    let out_name = out
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Pack regular tiles
    if !atlas_tiles.is_empty() {
        match pixl_render::atlas::pack_atlas(
            &atlas_tiles,
            palette,
            columns,
            padding,
            scale,
            &out_name,
        ) {
            Ok((img, json)) => {
                ensure_parent_dir(out);
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

    // Pack composites in a separate atlas (they may have different dimensions)
    if !composite_atlas_tiles.is_empty() {
        let comp_out = out.with_file_name(format!(
            "{}_composites.png",
            out.file_stem().unwrap_or_default().to_string_lossy()
        ));
        let comp_out_name = comp_out
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        match pixl_render::atlas::pack_atlas(
            &composite_atlas_tiles,
            palette,
            columns.min(4), // fewer columns for larger sprites
            padding,
            scale,
            &comp_out_name,
        ) {
            Ok((img, json)) => {
                ensure_parent_dir(&comp_out);
                if let Err(e) = img.save(&comp_out) {
                    eprintln!("error: cannot write composite atlas: {}", e);
                    process::exit(1);
                }
                println!(
                    "composite atlas: {} entries -> {}",
                    composite_atlas_tiles.len(),
                    comp_out.display()
                );

                if let Some(map_out) = map_path {
                    let comp_map = map_out.with_file_name(format!(
                        "{}_composites.json",
                        map_out.file_stem().unwrap_or_default().to_string_lossy()
                    ));
                    let json_str = serde_json::to_string_pretty(&json).unwrap();
                    if let Err(e) = std::fs::write(&comp_map, json_str) {
                        eprintln!("error: cannot write JSON: {}", e);
                        process::exit(1);
                    }
                    println!("composite metadata -> {}", comp_map.display());
                }
            }
            Err(e) => {
                eprintln!("composite atlas error: {}", e);
                // Non-fatal — regular atlas was already saved
            }
        }
    }
}

fn cmd_preview(file: &PathBuf, tile_name: &str, out: &PathBuf, show_grid: bool) {
    let (pax_file, palettes) = load_pax(file);

    let tile_raw = match pax_file.tile.get(tile_name) {
        Some(t) => t,
        None => pixl_core::resolve::base_tile_name(tile_name)
            .and_then(|base| pax_file.tile.get(base))
            .unwrap_or_else(|| {
                eprintln!("error: tile '{}' not found", tile_name);
                process::exit(1);
            }),
    };

    let palette = match palettes.get(&tile_raw.palette) {
        Some(p) => p,
        None => {
            eprintln!("error: palette '{}' not found", tile_raw.palette);
            process::exit(1);
        }
    };

    // Use unified resolver — handles grid, RLE, compose, template, symmetry
    let (grid, w, h) = match pixl_core::resolve::resolve_tile_grid(
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

    let preview_scale = 16u32;
    let img = pixl_render::renderer::render_grid(&grid, palette, preview_scale);
    let preview = pixl_render::preview::render_preview(&img, w, h, preview_scale, show_grid);

    ensure_parent_dir(out);
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
    weight_overrides: &[(String, f64)],
    pin_args: &[(usize, usize, String)],
    format: &OutputFormat,
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
        let cc = tile_raw.corner_class.as_ref();
        let mut te = pixl_wfc::adjacency::TileEdges::new(
            name,
            &ec.map(|e| e.n.clone()).unwrap_or_default(),
            &ec.map(|e| e.e.clone()).unwrap_or_default(),
            &ec.map(|e| e.s.clone()).unwrap_or_default(),
            &ec.map(|e| e.w.clone()).unwrap_or_default(),
            tile_raw.weight,
        );
        if let Some(cc) = cc {
            te.ne = cc.ne.clone();
            te.se = cc.se.clone();
            te.sw = cc.sw.clone();
            te.nw = cc.nw.clone();
        }
        tile_edges.push(te);
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
        let sorted_names: Vec<String> = order
            .iter()
            .map(|&i| tile_names_ordered[i].clone())
            .collect();
        let sorted_edges: Vec<_> = order.iter().map(|&i| tile_edges[i].clone()).collect();
        let sorted_affordances: Vec<_> =
            order.iter().map(|&i| tile_affordances[i].clone()).collect();
        let sorted_grids: Vec<Vec<Vec<char>>> =
            order.iter().map(|&i| tile_grids[i].clone()).collect();
        tile_names_ordered = sorted_names;
        tile_edges = sorted_edges;
        tile_affordances = sorted_affordances;
        tile_grids = sorted_grids;
    }

    // Apply weight overrides from --weight flags
    for (wname, wval) in weight_overrides {
        if let Some(idx) = tile_names_ordered.iter().position(|n| n == wname) {
            tile_edges[idx].weight = *wval;
        }
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
            tags: tile_raw.tags.clone(),
            target_layer: tile_raw.target_layer.clone(),
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
                pixl_wfc::adjacency::TileEdges::new(
                    &variant_name,
                    &rotated_ec.n,
                    &rotated_ec.e,
                    &rotated_ec.s,
                    &rotated_ec.w,
                    variant_weight,
                ),
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
        eprintln!(
            "hint: no rules provided. Use -r 'border:wall' -r 'region:room:walkable:4x4:center'"
        );
    }

    // Convert --pin args to Pin structs
    let extra_pins: Vec<pixl_wfc::wfc::Pin> = pin_args
        .iter()
        .filter_map(|(x, y, name)| {
            tile_names_ordered
                .iter()
                .position(|n| n == name)
                .map(|tile_idx| pixl_wfc::wfc::Pin {
                    x: *x,
                    y: *y,
                    tile_idx,
                })
        })
        .collect();

    let narrate_config = pixl_wfc::narrate::NarrateConfig {
        width,
        height,
        seed,
        max_retries: 5,
        predicates,
        extra_pins,
    };

    let is_json = matches!(format, OutputFormat::Json);

    if !is_json {
        println!(
            "narrate: {}x{} map, seed={}, {} rules",
            width,
            height,
            seed,
            rules.len()
        );
    }

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
            // Build the tile-name grid (used by both formats)
            let name_grid: Vec<Vec<&str>> = result
                .grid
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|&idx| {
                            tile_names_ordered
                                .get(idx)
                                .map(|s| s.as_str())
                                .unwrap_or("?")
                        })
                        .collect()
                })
                .collect();

            if is_json {
                // JSON output — machine-readable, no PNG rendering
                let grid_json: Vec<Vec<&str>> = name_grid;
                let json = serde_json::json!({
                    "grid": grid_json,
                    "width": width,
                    "height": height,
                    "seed": result.seed,
                    "retries": result.retries,
                    "pins_applied": result.pins_applied,
                    "tiles": tile_names_ordered,
                });
                println!("{}", serde_json::to_string(&json).unwrap());
            } else {
                println!(
                    "ok: generated in {} retries, {} pins applied",
                    result.retries, result.pins_applied
                );

                // Render the map to PNG
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
                            let tile_img = pixl_render::renderer::render_grid(
                                &tile_grids[tile_idx],
                                palette,
                                scale,
                            );
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

                ensure_parent_dir(out);
                if let Err(e) = img.save(out) {
                    eprintln!("error: {}", e);
                    process::exit(1);
                }
                println!("map -> {}", out.display());

                // Print tile name grid
                println!();
                for row in &name_grid {
                    println!("  {}", row.join(" "));
                }
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

    let variants =
        pixl_core::vary::generate_variants(tile_name, &base_grid, palette, count, seed, '.');

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

    println!(
        "# Generated {} variant(s) from '{}' (seed={})",
        variants.len(),
        tile_name,
        seed
    );
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

    println!(
        "# Generated {} stamp(s) for pattern '{}'",
        stamps.len(),
        pattern
    );
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
    let tile_names: Option<Vec<&str>> =
        tiles_filter.map(|s| s.split(',').map(|t| t.trim()).collect());

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

fn cmd_render_sprite(
    file_path: &PathBuf,
    spriteset_name: &str,
    sprite_name: &str,
    scale: u32,
    out: &PathBuf,
) {
    let source = match std::fs::read_to_string(file_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", file_path.display(), e);
            process::exit(1);
        }
    };

    let pax = match pixl_core::parser::parse_pax(&source) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    let palettes = match pixl_core::parser::resolve_all_palettes(&pax) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    let spriteset = match pax.spriteset.get(spriteset_name) {
        Some(ss) => ss,
        None => {
            eprintln!("error: spriteset '{}' not found", spriteset_name);
            let names: Vec<_> = pax.spriteset.keys().collect();
            if !names.is_empty() {
                eprintln!("  available: {:?}", names);
            }
            process::exit(1);
        }
    };

    let palette = match palettes.get(&spriteset.palette) {
        Some(p) => p,
        None => {
            eprintln!("error: palette '{}' not found", spriteset.palette);
            process::exit(1);
        }
    };

    let (sw, sh) = match pixl_core::types::parse_size(&spriteset.size) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    let sprite = match spriteset.sprite.iter().find(|s| s.name == sprite_name) {
        Some(s) => s,
        None => {
            let names: Vec<_> = spriteset.sprite.iter().map(|s| &s.name).collect();
            eprintln!(
                "error: sprite '{}' not found in '{}'",
                sprite_name, spriteset_name
            );
            if !names.is_empty() {
                eprintln!("  available: {:?}", names);
            }
            process::exit(1);
        }
    };

    // Resolve frames using the new animate module
    let resolved =
        match pixl_core::animate::resolve_sprite_frames(sprite, sw, sh, palette, sprite.fps) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("error: {}", e);
                process::exit(1);
            }
        };

    // Apply color cycles if the spriteset references any
    let cycle_refs: Vec<&pixl_core::types::Cycle> = spriteset
        .cycles
        .iter()
        .filter_map(|name| pax.cycle.get(name))
        .collect();

    let out_str = out.to_str().unwrap_or("");

    if out_str.ends_with(".gif") {
        // Render as animated GIF
        let gif_frames: Vec<(image::RgbaImage, u32)> = resolved
            .iter()
            .enumerate()
            .map(|(i, frame)| {
                // Apply cycles at this frame's tick
                let tick = i as u64;
                let effective = if !cycle_refs.is_empty() {
                    let cycled = pixl_core::animate::resolve_frames_with_cycles(
                        &[frame.clone()],
                        &cycle_refs,
                        palette,
                        tick,
                    );
                    cycled.into_iter().next().unwrap()
                } else {
                    frame.clone()
                };
                let img = pixl_render::renderer::render_grid(&effective.grid, palette, scale);
                (img, effective.duration_ms)
            })
            .collect();

        match pixl_render::gif::encode_gif(&gif_frames, sprite.r#loop) {
            Ok(bytes) => {
                if let Err(e) = std::fs::write(out, &bytes) {
                    eprintln!("error: {}", e);
                    process::exit(1);
                }
                println!(
                    "{} frames -> {} ({} fps)",
                    resolved.len(),
                    out.display(),
                    sprite.fps
                );
            }
            Err(e) => {
                eprintln!("error: {}", e);
                process::exit(1);
            }
        }
    } else {
        // Render as horizontal spritesheet PNG
        let frame_w = sw * scale;
        let frame_h = sh * scale;
        let sheet_w = frame_w * resolved.len() as u32;
        let mut sheet = image::RgbaImage::new(sheet_w, frame_h);

        for (i, frame) in resolved.iter().enumerate() {
            let img = pixl_render::renderer::render_grid(&frame.grid, palette, scale);
            image::imageops::overlay(&mut sheet, &img, (i as u32 * frame_w) as i64, 0);
        }

        if let Err(e) = sheet.save(out) {
            eprintln!("error: {}", e);
            process::exit(1);
        }
        println!(
            "{} frames -> {} ({}x{} spritesheet)",
            resolved.len(),
            out.display(),
            sheet_w,
            frame_h
        );
    }
}

fn cmd_backdrop_import(
    image_path: &PathBuf,
    name: &str,
    max_colors: u32,
    tile_size: u32,
    out: &PathBuf,
) {
    use image::GenericImageView;

    let img = match image::open(image_path) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("error: cannot open image {}: {}", image_path.display(), e);
            process::exit(1);
        }
    };

    let (w, h) = img.dimensions();
    println!("Importing {}x{} image as backdrop '{}'...", w, h, name);

    match pixl_render::pixelize::import_backdrop(&img, name, max_colors, tile_size) {
        Ok(result) => {
            if let Err(e) = std::fs::write(out, &result.pax_source) {
                eprintln!("error: cannot write {}: {}", out.display(), e);
                process::exit(1);
            }
            println!(
                "  {} tile slots ({} cols x {} rows)",
                result.tile_count, result.cols, result.rows
            );
            println!(
                "  {} unique tiles (dedup ratio: {:.0}%)",
                result.unique_tiles,
                (1.0 - result.unique_tiles as f64 / result.tile_count as f64) * 100.0
            );
            println!(
                "  PAX size: {:.1} KB",
                result.pax_source.len() as f64 / 1024.0
            );
            println!("Saved: {}", out.display());
        }
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    }
}

fn cmd_backdrop_render(
    file_path: &PathBuf,
    name: &str,
    out: &PathBuf,
    frames: u32,
    duration: u32,
    scale: u32,
) {
    use image::GenericImageView;

    let source = match std::fs::read_to_string(file_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", file_path.display(), e);
            process::exit(1);
        }
    };

    let pax = match pixl_core::parser::parse_pax(&source) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    let backdrop = match pixl_core::parser::resolve_backdrop(name, &pax) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    // Resolve palettes
    let palettes = match pixl_core::parser::resolve_all_palettes(&pax) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    // Resolve extended palette
    let backdrop_raw = &pax.backdrop[name];
    let palette_ext = if let Some(ext_name) = &backdrop_raw.palette_ext {
        if let Some(ext_raw) = pax.palette_ext.get(ext_name) {
            match pixl_core::parser::resolve_palette_ext(ext_name, ext_raw, &palettes) {
                Ok(pe) => pe,
                Err(e) => {
                    eprintln!("error resolving palette_ext: {}", e);
                    process::exit(1);
                }
            }
        } else {
            // No extended palette — build from base only
            let base = palettes
                .get(&backdrop_raw.palette)
                .cloned()
                .unwrap_or_else(|| pixl_core::types::Palette {
                    symbols: std::collections::HashMap::new(),
                });
            pixl_core::types::PaletteExt {
                base: base.symbols,
                extended: std::collections::HashMap::new(),
            }
        }
    } else {
        let base = palettes
            .get(&backdrop_raw.palette)
            .cloned()
            .unwrap_or_else(|| pixl_core::types::Palette {
                symbols: std::collections::HashMap::new(),
            });
        pixl_core::types::PaletteExt {
            base: base.symbols,
            extended: std::collections::HashMap::new(),
        }
    };

    // Resolve tile grids — parse RLE with ext support for each backdrop_tile
    let mut tile_grids: std::collections::HashMap<String, Vec<Vec<String>>> =
        std::collections::HashMap::new();
    for (tile_name, tile_raw) in &pax.backdrop_tile {
        let (tw, th) = tile_raw
            .size
            .as_deref()
            .and_then(|s| pixl_core::types::parse_size(s).ok())
            .unwrap_or((backdrop.tile_width, backdrop.tile_height));

        if let Some(rle) = &tile_raw.rle {
            match pixl_core::rle::parse_rle_ext(rle, tw, th, &palette_ext) {
                Ok(grid) => {
                    tile_grids.insert(tile_name.clone(), grid);
                }
                Err(e) => {
                    eprintln!("warning: tile '{}' RLE error: {}", tile_name, e);
                }
            }
        } else if let Some(grid_str) = &tile_raw.grid {
            // Plain grid (single-char symbols)
            let grid: Vec<Vec<String>> = grid_str
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .map(|line| line.chars().map(|c| c.to_string()).collect())
                .collect();
            tile_grids.insert(tile_name.clone(), grid);
        }
    }

    println!(
        "Backdrop '{}': {}x{} ({} tiles resolved)",
        name,
        backdrop.width,
        backdrop.height,
        tile_grids.len()
    );

    if frames == 0 {
        // Static render
        let img = pixl_render::backdrop::render_backdrop(&backdrop, &tile_grids, &palette_ext);
        let final_img = if scale > 1 {
            image::imageops::resize(
                &img,
                img.width() * scale,
                img.height() * scale,
                image::imageops::Nearest,
            )
        } else {
            img
        };
        if let Err(e) = final_img.save(out) {
            eprintln!("error: {}", e);
            process::exit(1);
        }
        println!("Saved static backdrop: {}", out.display());
    } else {
        // Animated GIF
        let gif_bytes = match pixl_render::backdrop::export_backdrop_gif(
            &backdrop,
            &tile_grids,
            &palette_ext,
            &pax.cycle,
            &palettes,
            Some(&pax),
            frames,
            duration,
            scale,
        ) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("error: {}", e);
                process::exit(1);
            }
        };
        if let Err(e) = std::fs::write(out, &gif_bytes) {
            eprintln!("error: {}", e);
            process::exit(1);
        }
        println!(
            "Saved {}-frame animated backdrop: {}",
            frames,
            out.display()
        );
    }
}

fn cmd_convert(
    input: &PathBuf,
    out: &PathBuf,
    width: Option<u32>,
    colors: u32,
    preview: Option<u32>,
) {
    let image_extensions = ["png", "jpg", "jpeg", "bmp", "gif", "webp", "tiff", "tga"];

    let inputs: Vec<PathBuf> = if input.is_dir() {
        // Collect all image files in directory
        match std::fs::read_dir(input) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.extension()
                        .and_then(|e| e.to_str())
                        .map(|e| image_extensions.contains(&e.to_lowercase().as_str()))
                        .unwrap_or(false)
                })
                .collect(),
            Err(e) => {
                eprintln!("error: cannot read directory {}: {}", input.display(), e);
                process::exit(1);
            }
        }
    } else {
        vec![input.clone()]
    };

    if inputs.is_empty() {
        eprintln!("error: no image files found in {}", input.display());
        process::exit(1);
    }

    println!("Converting {} image(s) to pixel art...", inputs.len());

    if let Some(w) = width {
        // Single-resolution mode
        std::fs::create_dir_all(out).unwrap_or_else(|e| {
            eprintln!("error: cannot create output dir: {e}");
            process::exit(1);
        });

        for path in &inputs {
            let img = match image::open(path) {
                Ok(i) => i,
                Err(e) => {
                    eprintln!("  skip {}: {}", path.display(), e);
                    continue;
                }
            };

            let result = pixl_render::pixelize::pixelize(&img, w, colors);
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("image");

            let final_img = if let Some(scale) = preview {
                image::DynamicImage::ImageRgba8(result.image)
                    .resize_exact(
                        result.width * scale,
                        result.height * scale,
                        image::imageops::Nearest,
                    )
                    .to_rgba8()
            } else {
                result.image
            };

            let out_path = out.join(format!("{stem}.png"));
            if let Err(e) = final_img.save(&out_path) {
                eprintln!("  error saving {}: {}", out_path.display(), e);
            } else {
                println!(
                    "  {} -> {} ({}x{}, {} colors)",
                    path.display(),
                    out_path.display(),
                    result.width,
                    result.height,
                    colors
                );
            }
        }
    } else {
        // Batch mode — 3 presets
        for path in &inputs {
            match pixl_render::pixelize::convert_batch(path, out) {
                Ok(batch) => {
                    println!(
                        "  {} ({}x{})",
                        path.display(),
                        batch.original_size.0,
                        batch.original_size.1
                    );
                    for r in &batch.results {
                        println!(
                            "    {} -> {}x{}, {} colors",
                            r.preset_name, r.width, r.height, r.num_colors
                        );
                    }
                }
                Err(e) => {
                    eprintln!("  error: {}: {}", path.display(), e);
                }
            }
        }
    }

    println!("Done. Output in {}", out.display());
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

fn cmd_scan(
    input: &PathBuf,
    out: &PathBuf,
    patch_size: u32,
    stride: Option<u32>,
    min_colors: usize,
    max_bg: f64,
    tile_size: Option<u32>,
    save_patches: bool,
    preview: bool,
) {
    use pixl_render::scan::{self, ScanConfig};

    let config = ScanConfig {
        patch_size,
        stride: stride.unwrap_or(patch_size),
        min_colors,
        max_bg_ratio: max_bg,
        native_tile_size: tile_size,
        ..Default::default()
    };

    println!("Scanning {}...", input.display());
    println!("  Patch size: {}x{}", patch_size, patch_size);
    println!("  Stride: {}", config.stride);
    println!();

    let manifest = if input.is_dir() {
        match scan::scan_directory(input, &config) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("error: {e}");
                process::exit(1);
            }
        }
    } else {
        match scan::scan_image(input, &config) {
            Ok(result) => scan::ScanManifest {
                patch_size: config.patch_size,
                stride: config.stride,
                sources: vec![result],
                total_patches_raw: 0,   // filled below
                total_patches_quality: 0,
                total_filtered: 0,
                categories: std::collections::HashMap::new(),
            },
            Err(e) => {
                eprintln!("error: {e}");
                process::exit(1);
            }
        }
    };

    // Print results
    let total_raw: usize = manifest.sources.iter().map(|s| s.total_patches).sum();
    let total_quality: usize = manifest.sources.iter().map(|s| s.quality_patches).sum();
    let filtered = total_raw - total_quality;

    println!("Results:");
    for src in &manifest.sources {
        println!(
            "  {} ({}x{}): {} patches ({} quality)",
            src.source.file_name().unwrap_or_default().to_string_lossy(),
            src.source_size.0,
            src.source_size.1,
            src.total_patches,
            src.quality_patches,
        );
    }
    println!();
    println!("  Total raw:      {}", total_raw);
    println!("  Quality passed: {}", total_quality);
    println!(
        "  Filtered out:   {} ({:.0}%)",
        filtered,
        if total_raw > 0 { filtered as f64 / total_raw as f64 * 100.0 } else { 0.0 }
    );

    // Category breakdown
    let mut categories = std::collections::HashMap::new();
    for src in &manifest.sources {
        for patch in &src.patches {
            *categories.entry(patch.category.clone()).or_insert(0usize) += 1;
        }
    }
    if !categories.is_empty() {
        println!();
        println!("  Categories:");
        let mut cats: Vec<_> = categories.into_iter().collect();
        cats.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        for (cat, count) in &cats {
            println!("    {}: {}", cat, count);
        }
    }

    // Save patches and manifest
    if save_patches && total_quality > 0 {
        println!();
        println!("Saving to {}...", out.display());

        // Load source images for patch extraction
        let mut source_images = vec![];
        for src in &manifest.sources {
            if let Ok(img) = image::open(&src.source) {
                source_images.push((src.source.clone(), img.to_rgba8()));
            }
        }

        if let Err(e) = scan::save_scan(&manifest, &source_images, out) {
            eprintln!("error saving scan: {e}");
            process::exit(1);
        }
        println!("  Saved {} patches to {}/patches/", total_quality, out.display());
        println!("  Manifest: {}/scan_manifest.json", out.display());
    }

    // Generate contact sheet preview
    if preview && total_quality > 0 {
        let patches_dir = out.join("patches");
        let mut patch_files: Vec<_> = std::fs::read_dir(&patches_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "png"))
            .map(|e| e.path())
            .collect();
        patch_files.sort();
        patch_files.truncate(64); // max 64 patches in preview

        let cols = 8u32;
        let rows = ((patch_files.len() as u32 + cols - 1) / cols).max(1);
        let ps = patch_size;
        let gap = 2u32;
        let sheet_w = cols * (ps + gap) + gap;
        let sheet_h = rows * (ps + gap) + gap;

        let mut sheet = image::RgbaImage::from_pixel(sheet_w, sheet_h, image::Rgba([40, 40, 40, 255]));

        for (i, path) in patch_files.iter().enumerate() {
            if let Ok(img) = image::open(path) {
                let rgba = img.to_rgba8();
                let col = (i as u32) % cols;
                let row = (i as u32) / cols;
                let x_off = gap + col * (ps + gap);
                let y_off = gap + row * (ps + gap);
                image::imageops::overlay(&mut sheet, &rgba, x_off as i64, y_off as i64);
            }
        }

        let preview_path = out.join("preview.png");
        // Scale up 4x for visibility
        let scaled = image::imageops::resize(&sheet, sheet_w * 4, sheet_h * 4, image::imageops::FilterType::Nearest);
        match scaled.save(&preview_path) {
            Ok(()) => println!("  Preview: {}", preview_path.display()),
            Err(e) => eprintln!("  warning: failed to save preview: {e}"),
        }
    }

    // Print next steps
    println!();
    println!("Next: prepare training data with:");
    println!("  pixl prepare {} --out training/data_custom", out.display());
}

fn cmd_prepare(
    scan_dir: &PathBuf,
    out: &PathBuf,
    style: &str,
    aug: u8,
    color_aug: bool,
    max_per_bin: usize,
    max_colors: usize,
) {
    use pixl_core::prepare::*;
    use std::collections::HashMap;

    let manifest_path = scan_dir.join("scan_manifest.json");
    let patches_dir = scan_dir.join("patches");

    if !manifest_path.exists() {
        eprintln!("error: no scan_manifest.json in {}", scan_dir.display());
        eprintln!("Run: pixl scan <images> --out {}", scan_dir.display());
        process::exit(1);
    }

    // Load manifest
    let manifest_json = match std::fs::read_to_string(&manifest_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read manifest: {e}");
            process::exit(1);
        }
    };
    let manifest: pixl_render::scan::ScanManifest =
        serde_json::from_str(&manifest_json).unwrap_or_else(|e| {
            eprintln!("error: invalid manifest: {e}");
            process::exit(1);
        });

    let patch_size = manifest.patch_size as usize;
    println!("Preparing training data from {}", scan_dir.display());
    println!("  Style tag: {style}");
    println!("  Augmentation: {}x geo{}", aug, if color_aug { " + 3x color" } else { "" });
    println!("  Stratification: max {} per bin", max_per_bin);
    println!();

    // Group patches by category for per-category palette extraction
    let mut category_patches: HashMap<String, Vec<pixl_render::scan::PatchInfo>> = HashMap::new();
    for source in &manifest.sources {
        for patch in &source.patches {
            category_patches
                .entry(patch.category.clone())
                .or_default()
                .push(patch.clone());
        }
    }

    // Extract palettes per category
    println!("Extracting palettes...");
    let mut category_palettes: HashMap<String, Vec<(char, [u8; 3])>> = HashMap::new();
    let symbol_pool = ".#+=~gorhwsABCDE";

    for (category, patches) in &category_patches {
        let mut all_pixels: Vec<Vec<u8>> = vec![];
        for patch_info in patches {
            let patch_path = patches_dir.join(&patch_info.filename);
            if let Ok(img) = image::open(&patch_path) {
                all_pixels.push(img.to_rgba8().into_raw());
            }
        }
        let pixel_refs: Vec<&[u8]> = all_pixels.iter().map(|v| v.as_slice()).collect();
        let palette = extract_palette_from_pixels(&pixel_refs, max_colors, symbol_pool);
        println!("  {}: {} colors from {} patches", category, palette.len(), patches.len());
        category_palettes.insert(category.clone(), palette);
    }

    // Quantize + augment + label
    println!("\nQuantizing and augmenting...");
    let mut all_samples: Vec<(TrainingSample, GridFeatures)> = vec![];
    let color_shifts: Vec<&str> = if color_aug {
        vec!["", "warm", "cool", "dark"]
    } else {
        vec![""]
    };

    for (category, patches) in &category_patches {
        let palette = match category_palettes.get(category) {
            Some(p) if !p.is_empty() => p,
            _ => continue,
        };

        let mut cat_count = 0;
        for patch_info in patches {
            let patch_path = patches_dir.join(&patch_info.filename);
            let img = match image::open(&patch_path) {
                Ok(i) => i.to_rgba8(),
                Err(_) => continue,
            };

            let pixels = img.into_raw();
            let grid = quantize_to_grid(&pixels, patch_size, patch_size, palette);

            // Check non-void density
            let non_void: usize = grid.iter().flat_map(|r| r.iter()).filter(|&&c| c != '.').count();
            if non_void < patch_size * patch_size / 20 {
                continue;
            }

            let features = compute_features(&grid);

            for &color_shift in &color_shifts {
                let shifted_palette = if color_shift.is_empty() {
                    palette.clone()
                } else {
                    shift_palette(palette, color_shift)
                };
                let pal_desc = palette_to_desc(&shifted_palette);

                for (aug_grid, aug_tag) in augment_grid(&grid, aug) {
                    let label = make_label(&features, style, category, aug_tag, color_shift);
                    let grid_str = grid_to_string(&aug_grid);
                    let sample = make_sample(&pal_desc, &label, &grid_str);
                    all_samples.push((sample, features.clone()));
                    cat_count += 1;
                }
            }
        }
        println!("  {}: {} training pairs", category, cat_count);
    }

    println!("\n  Total before stratification: {}", all_samples.len());

    // Stratify (auto-selects 3×3 or 5×5 bins based on coverage)
    let (stratified, bins_filled) = stratified_sample(all_samples, max_per_bin, 42);
    println!("  After stratification: {} ({} bins filled)", stratified.len(), bins_filled);

    // Split: 90/5/5
    let n = stratified.len();
    let train_end = (n as f64 * 0.9) as usize;
    let valid_end = (n as f64 * 0.95) as usize;

    let train = &stratified[..train_end];
    let valid = &stratified[train_end..valid_end];
    let test = &stratified[valid_end..];

    // Write
    std::fs::create_dir_all(out).unwrap_or_else(|e| {
        eprintln!("error: cannot create {}: {e}", out.display());
        process::exit(1);
    });

    write_jsonl(train, &out.join("train.jsonl")).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        process::exit(1);
    });
    write_jsonl(valid, &out.join("valid.jsonl")).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        process::exit(1);
    });
    write_jsonl(test, &out.join("test.jsonl")).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        process::exit(1);
    });

    println!("\n  train: {} samples", train.len());
    println!("  valid: {} samples", valid.len());
    println!("  test:  {} samples", test.len());
    println!("\n  Output: {}", out.display());

    // Training time estimate
    let iters_3ep = train.len() * 3;
    let iters_5ep = train.len() * 5;
    println!("\n  Estimated training time (M4 Pro, ~2 it/sec):");
    println!("    3 epochs: {} iters -> ~{} min", iters_3ep, iters_3ep / 2 / 60);
    println!("    5 epochs: {} iters -> ~{} min", iters_5ep, iters_5ep / 2 / 60);

    println!("\nNext: train with:");
    println!("  pixl train {} --adapter training/adapters/{}", out.display(), style);
}

/// Discover all `data_*` directories under a base path that contain `train.jsonl`.
fn discover_datasets(base: &std::path::Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(base) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = p.file_name().unwrap_or_default().to_string_lossy();
                if name.starts_with("data_") && p.join("train.jsonl").exists() {
                    dirs.push(p);
                }
            }
        }
    }
    dirs.sort();
    dirs
}

/// Extract the suffix after `data_` from a dataset directory name.
fn dataset_suffix(dir: &std::path::Path) -> String {
    dir.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .strip_prefix("data_")
        .unwrap_or("")
        .to_string()
}

/// Merge multiple `train.jsonl` files, deduplicating by exact line content.
/// Returns the temp directory path and merged sample count.
fn merge_datasets(dirs: &[PathBuf]) -> (PathBuf, usize) {
    use std::collections::HashSet;
    use std::io::Write;

    let merged_dir = std::env::temp_dir().join("pixl_merged_data");
    let _ = std::fs::remove_dir_all(&merged_dir);
    std::fs::create_dir_all(&merged_dir).unwrap_or_else(|e| {
        eprintln!("error: cannot create temp merge dir: {e}");
        process::exit(1);
    });

    let mut seen = HashSet::new();
    let mut train_out = std::fs::File::create(merged_dir.join("train.jsonl")).unwrap();
    let mut valid_out = std::fs::File::create(merged_dir.join("valid.jsonl")).unwrap();
    let mut test_out = std::fs::File::create(merged_dir.join("test.jsonl")).unwrap();

    let mut total = 0usize;

    for dir in dirs {
        // Merge train.jsonl (deduplicate)
        if let Ok(content) = std::fs::read_to_string(dir.join("train.jsonl")) {
            for line in content.lines().filter(|l| !l.is_empty()) {
                if seen.insert(line.to_string()) {
                    writeln!(train_out, "{}", line).unwrap();
                    total += 1;
                }
            }
        }
        // Merge valid.jsonl (deduplicate)
        if let Ok(content) = std::fs::read_to_string(dir.join("valid.jsonl")) {
            for line in content.lines().filter(|l| !l.is_empty()) {
                if seen.insert(line.to_string()) {
                    writeln!(valid_out, "{}", line).unwrap();
                }
            }
        }
        // Merge test.jsonl (deduplicate)
        if let Ok(content) = std::fs::read_to_string(dir.join("test.jsonl")) {
            for line in content.lines().filter(|l| !l.is_empty()) {
                if seen.insert(line.to_string()) {
                    writeln!(test_out, "{}", line).unwrap();
                }
            }
        }
    }

    (merged_dir, total)
}

fn cmd_train(
    data_dir: &PathBuf,
    adapter: &PathBuf,
    model: &str,
    epochs: usize,
    lr: f64,
    layers: usize,
    resume: bool,
    sources: Option<&str>,
    exclude: Option<&str>,
) {
    // Resolve the effective data directory — either merged from --sources/--exclude or the given data_dir
    let (effective_dir, _cleanup) = if sources.is_some() || exclude.is_some() {
        // Find the base training directory: parent of data_dir, or data_dir itself if it looks like a training root
        let base = if data_dir.join("train.jsonl").exists() {
            data_dir.parent().unwrap_or(data_dir).to_path_buf()
        } else {
            data_dir.to_path_buf()
        };

        let all_datasets = discover_datasets(&base);
        if all_datasets.is_empty() {
            eprintln!("error: no data_* directories with train.jsonl found in {}", base.display());
            process::exit(1);
        }

        let selected: Vec<PathBuf> = if let Some(src) = sources {
            let names: Vec<&str> = src.split(',').map(|s| s.trim()).collect();
            all_datasets.iter()
                .filter(|d| names.contains(&dataset_suffix(d).as_str()))
                .cloned()
                .collect()
        } else {
            // --exclude only: start with all datasets
            all_datasets.clone()
        };

        let selected: Vec<PathBuf> = if let Some(excl) = exclude {
            let excl_names: Vec<&str> = excl.split(',').map(|s| s.trim()).collect();
            selected.into_iter()
                .filter(|d| !excl_names.contains(&dataset_suffix(d).as_str()))
                .collect()
        } else {
            selected
        };

        if selected.is_empty() {
            eprintln!("error: no datasets matched the given --sources/--exclude filters");
            eprintln!("\nAvailable datasets:");
            for d in &all_datasets {
                eprintln!("  {}", dataset_suffix(d));
            }
            process::exit(1);
        }

        println!("Merging {} datasets:", selected.len());
        for d in &selected {
            let suffix = dataset_suffix(d);
            let count = std::fs::read_to_string(d.join("train.jsonl"))
                .unwrap_or_default()
                .lines()
                .filter(|l| !l.is_empty())
                .count();
            println!("  {} ({} samples)", suffix, count);
        }
        println!();

        let (merged, total) = merge_datasets(&selected);
        println!("Merged: {} unique train samples (duplicates removed)\n", total);
        (merged, true)
    } else {
        (data_dir.clone(), false)
    };

    let train_path = effective_dir.join("train.jsonl");
    if !train_path.exists() {
        eprintln!("error: no train.jsonl in {}", effective_dir.display());
        eprintln!("Run: pixl prepare <scan_dir> --out {}", effective_dir.display());
        process::exit(1);
    }

    // Count training samples
    let train_count = std::fs::read_to_string(&train_path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .count();

    let iters = train_count * epochs;
    let est_min = iters / 2 / 60;

    println!("PIXL LoRA Training");
    println!("==================");
    println!("  Model:   {model}");
    println!("  Data:    {train_count} train samples");
    println!("  Epochs:  {epochs}");
    println!("  Iters:   {iters}");
    println!("  Est:     ~{est_min} min on M4 Pro");
    println!("  Adapter: {}", adapter.display());
    if resume {
        println!("  Resuming from existing adapter");
    }
    println!();

    // Find Python with mlx-lm
    let python = pixl_mcp::inference::find_python_with_mlx();

    println!("Using Python: {}", python);
    println!();

    std::fs::create_dir_all(adapter).unwrap_or_else(|e| {
        eprintln!("error: cannot create adapter dir: {e}");
        process::exit(1);
    });

    let mut cmd = std::process::Command::new(&python);
    cmd.args(["-m", "mlx_lm", "lora"])
        .arg("--model").arg(model)
        .arg("--train")
        .arg("--data").arg(&effective_dir)
        .arg("--adapter-path").arg(adapter)
        .arg("--fine-tune-type").arg("lora")
        .arg("--num-layers").arg(layers.to_string())
        .arg("--batch-size").arg("1")
        .arg("--learning-rate").arg(format!("{lr}"))
        .arg("--iters").arg(iters.to_string())
        .arg("--val-batches").arg("25")
        .arg("--steps-per-eval").arg("500")
        .arg("--save-every").arg("2000")
        .arg("--max-seq-length").arg("512")
        .arg("--seed").arg("42");

    if resume {
        let adapter_file = adapter.join("adapters.safetensors");
        if adapter_file.exists() {
            cmd.arg("--resume-adapter-file").arg(&adapter_file);
        }
    }

    let status = cmd
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .unwrap_or_else(|e| {
            eprintln!("error: failed to start training: {e}");
            process::exit(1);
        });

    if status.success() {
        // Write adapter metadata
        let meta = serde_json::json!({
            "model": model,
            "data_dir": data_dir.display().to_string(),
            "train_samples": train_count,
            "epochs": epochs,
            "learning_rate": lr,
            "lora_layers": layers,
        });
        let _ = std::fs::write(
            adapter.join("style_adapter.json"),
            serde_json::to_string_pretty(&meta).unwrap_or_default(),
        );

        println!("\nTraining complete!");
        println!("Adapter: {}", adapter.display());
        println!("\nTo use:");
        println!("  pixl serve --model {} --adapter {}", model, adapter.display());
    } else {
        eprintln!("\nTraining failed with exit code: {:?}", status.code());
        process::exit(status.code().unwrap_or(1));
    }
}

fn cmd_retrain(file: &PathBuf, adapter: &PathBuf, style: &str, epochs: usize) {
    println!("PIXL Retrain — feedback → data → adapter");
    println!("==========================================\n");

    // Step 1: Load the pax file and export feedback as training data
    let (pax_file, palettes) = load_pax(file);

    let palette_name = pax_file.tile.values()
        .next()
        .map(|t| t.palette.as_str())
        .unwrap_or("");
    let palette = match palettes.get(palette_name) {
        Some(p) => p,
        None => {
            eprintln!("error: no palette found in {}", file.display());
            process::exit(1);
        }
    };

    // Collect all tile grids as training pairs
    let mut grids: Vec<(String, Vec<Vec<char>>)> = Vec::new();
    for (name, tile_raw) in &pax_file.tile {
        if tile_raw.template.is_some() {
            continue;
        }
        let size_str = tile_raw.size.as_deref().unwrap_or("16x16");
        let (w, h) = match pixl_core::types::parse_size(size_str) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if let Some(ref grid_str) = tile_raw.grid {
            if let Ok(grid) = pixl_core::grid::parse_grid(grid_str, w, h, palette) {
                grids.push((name.clone(), grid));
            }
        }
    }

    if grids.is_empty() {
        eprintln!("error: no tiles with grids found in {}", file.display());
        process::exit(1);
    }

    println!("  Source: {} ({} tiles)", file.display(), grids.len());

    // Step 2: Write training pairs directly as JSONL
    let data_dir = adapter.parent()
        .unwrap_or(std::path::Path::new("."))
        .join(format!("data_{}", style));
    std::fs::create_dir_all(&data_dir).unwrap_or_else(|e| {
        eprintln!("error: cannot create {}: {e}", data_dir.display());
        process::exit(1);
    });

    let system_prompt = "You are a pixel art tile generator. Given a description, output a PAX-format character grid.\nRules:\n- Use only the symbols from the palette provided\n- Each row must be exactly the specified width\n- Total rows must equal the specified height\n- '.' means transparent/void\n- Output ONLY the grid, no explanation";

    let palette_desc: String = palette.symbols.iter()
        .map(|(sym, rgba)| format!("'{}'=({},{},{})", sym, rgba.r, rgba.g, rgba.b))
        .collect::<Vec<_>>()
        .join(" ");

    let mut samples = Vec::new();
    let rotation_labels = ["orig", "r90", "r180", "r270"];
    for (name, grid) in &grids {
        let features = pixl_core::prepare::compute_features(grid);
        let mut rotated = grid.clone();
        for &rot_label in &rotation_labels {
            let grid_str = pixl_core::prepare::grid_to_string(&rotated);
            let label = pixl_core::prepare::make_label(&features, style, &name, rot_label, "");
            let sample = pixl_core::prepare::make_sample(&palette_desc, &label, &grid_str);
            samples.push(sample);
            rotated = pixl_core::prepare::rotate_90(&rotated);
        }
    }

    // Shuffle and split
    pixl_core::prepare::fisher_yates_shuffle_pub(&mut samples, 42);
    let n = samples.len();
    let train_end = (n as f64 * 0.9) as usize;
    let valid_end = (n as f64 * 0.95) as usize;

    let train = &samples[..train_end];
    let valid = &samples[train_end..valid_end];
    let test = &samples[valid_end..];

    pixl_core::prepare::write_jsonl(train, &data_dir.join("train.jsonl")).unwrap();
    pixl_core::prepare::write_jsonl(valid, &data_dir.join("valid.jsonl")).unwrap();
    pixl_core::prepare::write_jsonl(test, &data_dir.join("test.jsonl")).unwrap();

    println!("  Data: {} train, {} valid, {} test", train.len(), valid.len(), test.len());
    println!("  Output: {}\n", data_dir.display());

    // Step 3: Train
    cmd_train(&data_dir, adapter, "mlx-community/Qwen2.5-3B-Instruct-4bit", epochs, 0.00002, 16, false, None, None);
}

fn cmd_generate_set(
    file: &PathBuf,
    set_type: &str,
    theme: &str,
    count: usize,
    out: &PathBuf,
    adapter: Option<&std::path::Path>,
    model: &str,
) {
    println!("PIXL Generate Set");
    println!("=================\n");
    println!("  Type:    {}", set_type);
    println!("  Theme:   {}", theme);
    println!("  Count:   {}", count);
    println!("  Output:  {}", out.display());

    // Load palette from pax file
    let (pax_file, palettes) = load_pax(file);
    let palette_name = pax_file.tile.values()
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

    // Build palette description
    let palette_desc: String = palette.symbols.iter()
        .map(|(sym, rgba)| format!("'{}'=({},{},{})", sym, rgba.r, rgba.g, rgba.b))
        .collect::<Vec<_>>()
        .join(" ");

    // Descriptions per set type
    let descriptions: Vec<String> = match set_type {
        "walls" => (0..count).map(|i| {
            let variants = [
                "stone wall with brick pattern and mortar lines",
                "rough hewn wall with deep shadow cracks",
                "smooth wall with carved decorative detail",
                "weathered wall with exposed stone layers",
                "reinforced wall with metal bracket accents",
                "ancient wall with moss and water stains",
            ];
            format!("{} {}", theme, variants[i % variants.len()])
        }).collect(),
        "floors" => (0..count).map(|i| {
            let variants = [
                "stone floor with tile pattern",
                "cobblestone floor with worn surface",
                "smooth flagstone floor",
            ];
            format!("{} {}", theme, variants[i % variants.len()])
        }).collect(),
        "enemies" => (0..count).map(|i| {
            let variants = [
                "front-facing enemy creature",
                "armored enemy warrior",
                "robed magic-user enemy",
                "undead skeleton enemy",
                "small imp creature",
            ];
            format!("{} {}", theme, variants[i % variants.len()])
        }).collect(),
        "items" => (0..count).map(|i| {
            let variants = [
                "treasure chest icon",
                "potion bottle icon",
                "sword weapon icon",
                "shield armor icon",
                "key item icon",
                "scroll magic item",
            ];
            format!("{} {}", theme, variants[i % variants.len()])
        }).collect(),
        _ => (0..count).map(|i| format!("{} tile variant {}", theme, i + 1)).collect(),
    };

    std::fs::create_dir_all(out).unwrap_or_else(|e| {
        eprintln!("error: cannot create {}: {e}", out.display());
        process::exit(1);
    });

    let system_prompt = "You are a pixel art tile generator. Given a description, output a PAX-format character grid.\nRules:\n- Use only the symbols from the palette provided\n- Each row must be exactly the specified width\n- Total rows must equal the specified height\n- '.' means transparent/void\n- Output ONLY the grid, no explanation";

    // Start inference server
    let adapter_path = adapter
        .map(|a| PathBuf::from(a))
        .or_else(|| std::env::var("PIXL_ADAPTER").ok().map(PathBuf::from));

    let inf_config = pixl_mcp::inference::InferenceConfig {
        model: model.to_string(),
        adapter_path,
        port: 8099,
        ..Default::default()
    };
    let mut server = pixl_mcp::inference::InferenceServer::new(inf_config);

    println!("\n  Starting inference server...");

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        if let Err(e) = server.ensure_running().await {
            eprintln!("error: cannot start inference server: {e}");
            process::exit(1);
        }

        println!("  Generating {} {}...\n", count, set_type);

        let mut generated = 0;
        for (i, desc) in descriptions.iter().enumerate() {
            let user_prompt = format!(
                "Palette: {}\nstyle:{}, type:{}, density:solid, detail:complex, colors:rich",
                palette_desc, theme, desc
            );

            // Rejection sampling: retry up to 5 times
            let mut best_grid = String::new();
            let mut best_unique = 0usize;

            for _attempt in 0..5 {
                let raw = match server.generate(system_prompt, &user_prompt).await {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("  warning: generation failed for '{}': {e}", desc);
                        break;
                    }
                };

                // Extract grid lines
                let mut grid_lines = Vec::new();
                for line in raw.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with("```") {
                        continue;
                    }
                    if trimmed.len() >= 8 {
                        grid_lines.push(&trimmed[..trimmed.len().min(16)]);
                    }
                    if grid_lines.len() >= 16 {
                        break;
                    }
                }

                let grid_str = grid_lines.join("\n");
                let unique: std::collections::HashSet<char> = grid_str
                    .chars()
                    .filter(|c| !c.is_whitespace() && *c != '.')
                    .collect();

                if unique.len() > best_unique {
                    best_unique = unique.len();
                    best_grid = grid_str;
                }
                if unique.len() >= 3 {
                    break;
                }
            }

            if best_grid.is_empty() {
                eprintln!("  [{}/{count}] SKIP: {} (no valid grid)", i + 1, desc);
                continue;
            }

            // Parse and normalize grid (pad short rows, truncate long rows)
            let mut grid: Vec<Vec<char>> = best_grid
                .lines()
                .map(|l| l.chars().collect())
                .collect();

            if grid.is_empty() { continue; }

            let target_w = grid.iter().map(|r| r.len()).max().unwrap_or(16).min(16);
            for row in &mut grid {
                row.truncate(target_w);
                while row.len() < target_w {
                    row.push('.');
                }
            }
            // Pad to 16 rows if needed
            while grid.len() < 16 {
                grid.push(vec!['.'; target_w]);
            }
            grid.truncate(16);

            let img = pixl_render::renderer::render_grid(&grid, palette, 8);
            let out_path = out.join(format!("{}_{:03}.png", set_type, i));
            match img.save(&out_path) {
                Ok(()) => {
                    println!("  [{}/{count}] {} ({} colors)", i + 1, out_path.display(), best_unique);
                    generated += 1;
                }
                Err(e) => eprintln!("  warning: failed to save {}: {e}", out_path.display()),
            }

            // Also save the grid as a .txt for reference
            let txt_path = out.join(format!("{}_{:03}.txt", set_type, i));
            let _ = std::fs::write(&txt_path, &best_grid);
        }

        println!("\nGenerated {generated}/{count} {} to {}", set_type, out.display());
    });
}

fn cmd_datasets(dir: Option<&std::path::Path>) {
    let cwd = std::env::current_dir().unwrap_or_default();

    // Determine base directories to scan
    let mut search_dirs = vec![];
    if let Some(d) = dir {
        search_dirs.push(d.to_path_buf());
    }

    for candidate in &["training", "../training"] {
        let p = cwd.join(candidate);
        if p.exists() && p.is_dir() {
            search_dirs.push(p);
        }
    }

    if search_dirs.is_empty() {
        println!("No training directories found.");
        println!("Specify one with: pixl datasets --dir path/to/training/");
        return;
    }

    println!("PIXL Training Datasets");
    println!("======================\n");

    let mut total_samples = 0usize;
    let mut total_datasets = 0usize;

    // Column widths
    let w_name = 24;
    let w_samples = 10;
    let w_style = 16;
    let w_source = 30;

    println!(
        "  {:<w_name$}  {:>w_samples$}  {:<w_style$}  {}",
        "Name", "Samples", "Style", "Source"
    );
    println!(
        "  {:<w_name$}  {:>w_samples$}  {:<w_style$}  {}",
        "-".repeat(w_name), "-".repeat(w_samples), "-".repeat(w_style), "-".repeat(w_source)
    );

    for base in &search_dirs {
        let datasets = discover_datasets(base);
        for dataset_dir in datasets {
            let suffix = dataset_suffix(&dataset_dir);
            let train_path = dataset_dir.join("train.jsonl");
            let sample_count = std::fs::read_to_string(&train_path)
                .unwrap_or_default()
                .lines()
                .filter(|l| !l.is_empty())
                .count();

            // Try to read dataset_info.json for metadata
            let mut style_tag = String::from("-");
            let mut source_info = String::from("-");
            let info_path = dataset_dir.join("dataset_info.json");
            if info_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&info_path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(s) = json.get("style").and_then(|v| v.as_str()) {
                            style_tag = s.to_string();
                        }
                        if let Some(s) = json.get("source").and_then(|v| v.as_str()) {
                            source_info = s.to_string();
                        }
                        // Also check for sources array
                        if source_info == "-" {
                            if let Some(arr) = json.get("sources").and_then(|v| v.as_array()) {
                                let names: Vec<&str> = arr.iter()
                                    .filter_map(|v| v.as_str())
                                    .collect();
                                if !names.is_empty() {
                                    source_info = names.join(", ");
                                }
                            }
                        }
                    }
                }
            }

            // Truncate source info if too long
            if source_info.len() > w_source {
                source_info = format!("{}...", &source_info[..w_source - 3]);
            }

            println!(
                "  {:<w_name$}  {:>w_samples$}  {:<w_style$}  {}",
                suffix, sample_count, style_tag, source_info
            );

            total_samples += sample_count;
            total_datasets += 1;
        }
    }

    println!(
        "\n  {} dataset(s), {} total train samples\n",
        total_datasets, total_samples
    );

    if total_datasets > 0 {
        println!("Usage:");
        println!("  pixl train training/data_NAME --adapter training/adapters/my-style");
        println!("  pixl train training/ --sources eotb_optimal,matched --adapter my-adapter");
    } else {
        println!("No datasets found. Create one with:");
        println!("  pixl scan my_sprites/ --out my_scan");
        println!("  pixl prepare my_scan/ --out training/data_custom --style my-game");
    }
}

fn cmd_adapters(dir: Option<&std::path::Path>) {
    // Scan default locations + provided dir
    let mut search_dirs = vec![];
    if let Some(d) = dir {
        search_dirs.push(d.to_path_buf());
    }

    // Common adapter locations relative to cwd
    let cwd = std::env::current_dir().unwrap_or_default();
    for candidate in &[
        "training/adapters",
        "adapters",
        "../training/adapters",
    ] {
        let p = cwd.join(candidate);
        if p.exists() {
            search_dirs.push(p);
        }
    }

    if search_dirs.is_empty() {
        println!("No adapter directories found.");
        println!("Specify one with: pixl adapters --dir path/to/adapters/");
        return;
    }

    let mut found = 0;
    for search_dir in &search_dirs {
        let adapters = pixl_mcp::adapters::list_adapters(search_dir);
        if adapters.is_empty() {
            continue;
        }

        println!("Adapters in {}:", search_dir.display());
        println!();

        for a in &adapters {
            found += 1;
            println!("  {} {}", if found == 1 { "→" } else { " " }, a.name);
            println!("    Path: {}", a.path.display());
            if let Some(ref model) = a.model {
                println!("    Model: {model}");
            }
            if let Some(samples) = a.train_samples {
                print!("    Trained on: {samples} samples");
                if let Some(epochs) = a.epochs {
                    print!(", {epochs} epochs");
                }
                println!();
            }
            if let Some(ref created) = a.created {
                println!("    Created: {created}");
            }
            println!();
        }
    }

    if found == 0 {
        println!("No adapters found.");
        println!("\nTrain one with:");
        println!("  pixl scan my_sprites/ --out my_scan");
        println!("  pixl prepare my_scan/ --out my_data --style my-game");
        println!("  pixl train my_data --adapter training/adapters/my-game");
    } else {
        println!("{found} adapter(s) found.");
        println!("\nTo use an adapter:");
        println!("  pixl serve --adapter training/adapters/NAME --file project.pax");
    }
}

fn cmd_compare(
    file: &PathBuf,
    prompt: &str,
    adapter_a: &PathBuf,
    adapter_b: &PathBuf,
    out: &PathBuf,
    model: &str,
) {
    use image::{ImageBuffer, Rgb, RgbaImage};

    println!("PIXL Adapter Comparison");
    println!("=======================\n");
    println!("  Prompt:    {prompt}");
    println!("  Adapter A: {}", adapter_a.display());
    println!("  Adapter B: {}", adapter_b.display());
    println!("  Output:    {}", out.display());

    let (pax_file, palettes) = load_pax(file);
    let palette_name = pax_file.tile.values()
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

    let palette_desc: String = palette.symbols.iter()
        .map(|(sym, rgba)| format!("'{}'=({},{},{})", sym, rgba.r, rgba.g, rgba.b))
        .collect::<Vec<_>>()
        .join(" ");

    let system_prompt = "You are a pixel art tile generator. Given a description, output a PAX-format character grid.\nRules:\n- Use only the symbols from the palette provided\n- Each row must be exactly the specified width\n- Total rows must equal the specified height\n- '.' means transparent/void\n- Output ONLY the grid, no explanation";

    let user_prompt = format!(
        "Palette: {}\n{}, density:solid, detail:complex, colors:rich",
        palette_desc, prompt
    );

    let rt = tokio::runtime::Runtime::new().unwrap();

    let generate_with_adapter = |adapter_path: &PathBuf, label: &str| -> Option<RgbaImage> {
        println!("\n  Generating with {label}...");

        let inf_config = pixl_mcp::inference::InferenceConfig {
            model: model.to_string(),
            adapter_path: Some(adapter_path.clone()),
            port: 8099,
            ..Default::default()
        };
        let mut server = pixl_mcp::inference::InferenceServer::new(inf_config);

        rt.block_on(async {
            if let Err(e) = server.ensure_running().await {
                eprintln!("    error: {e}");
                return None;
            }

            // Rejection sampling
            let mut best_grid = String::new();
            let mut best_unique = 0;
            for _ in 0..5 {
                let raw = match server.generate(system_prompt, &user_prompt).await {
                    Ok(r) => r,
                    Err(e) => { eprintln!("    error: {e}"); break; }
                };

                let mut lines = Vec::new();
                for l in raw.lines() {
                    let t = l.trim();
                    if !t.is_empty() && !t.starts_with("```") && t.len() >= 8 {
                        lines.push(&t[..t.len().min(16)]);
                    }
                    if lines.len() >= 16 { break; }
                }
                let grid_str = lines.join("\n");
                let unique: std::collections::HashSet<char> = grid_str
                    .chars().filter(|c| !c.is_whitespace() && *c != '.').collect();

                if unique.len() > best_unique {
                    best_unique = unique.len();
                    best_grid = grid_str;
                }
                if unique.len() >= 3 { break; }
            }

            server.stop();

            if best_grid.is_empty() { return None; }

            let grid: Vec<Vec<char>> = best_grid.lines()
                .map(|l| l.chars().collect()).collect();

            println!("    {} colors", best_unique);
            Some(pixl_render::renderer::render_grid(&grid, palette, 8))
        })
    };

    let img_a = generate_with_adapter(adapter_a, "Adapter A");
    let img_b = generate_with_adapter(adapter_b, "Adapter B");

    // Stitch side by side with a gap
    let gap = 8u32;
    let tile_w = 16 * 8; // 128px at 8x scale
    let tile_h = 16 * 8;
    let label_h = 20u32;
    let total_w = tile_w * 2 + gap;
    let total_h = tile_h + label_h;

    let mut combined: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_pixel(
        total_w, total_h, Rgb([30, 30, 30])
    );

    // Paste tiles
    if let Some(ref img) = img_a {
        for y in 0..img.height().min(tile_h) {
            for x in 0..img.width().min(tile_w) {
                let p = img.get_pixel(x, y);
                combined.put_pixel(x, y + label_h, Rgb([p[0], p[1], p[2]]));
            }
        }
    }
    if let Some(ref img) = img_b {
        let x_off = tile_w + gap;
        for y in 0..img.height().min(tile_h) {
            for x in 0..img.width().min(tile_w) {
                let p = img.get_pixel(x, y);
                combined.put_pixel(x + x_off, y + label_h, Rgb([p[0], p[1], p[2]]));
            }
        }
    }

    // Save
    if let Some(parent) = out.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match combined.save(out) {
        Ok(()) => println!("\n  Comparison saved: {}", out.display()),
        Err(e) => eprintln!("\n  error saving: {e}"),
    }
}

fn cmd_blend_style(file_a: &PathBuf, file_b: &PathBuf, blend: f64) {
    println!("PIXL Style Blend");
    println!("================\n");

    // Extract style latent from each file
    let extract_latent = |file: &PathBuf| -> pixl_core::style::StyleLatent {
        let (pax_file, palettes) = load_pax(file);
        let palette_name = pax_file.tile.values()
            .next()
            .map(|t| t.palette.as_str())
            .unwrap_or("");
        let palette = match palettes.get(palette_name) {
            Some(p) => p,
            None => {
                eprintln!("error: no palette in {}", file.display());
                process::exit(1);
            }
        };

        let mut grids: Vec<Vec<Vec<char>>> = Vec::new();
        for (_, tile_raw) in &pax_file.tile {
            if tile_raw.template.is_some() { continue; }
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
        let refs: Vec<&Vec<Vec<char>>> = grids.iter().collect();
        pixl_core::style::StyleLatent::extract(&refs, palette, '.')
    };

    let latent_a = extract_latent(file_a);
    let latent_b = extract_latent(file_b);

    println!("Style A ({} tiles):", latent_a.sample_count);
    println!("{}\n", latent_a.describe());
    println!("Style B ({} tiles):", latent_b.sample_count);
    println!("{}\n", latent_b.describe());

    let distance = latent_a.distance(&latent_b);
    println!("Distance: {:.3} (0=identical, 1=very different)\n", distance);

    let blended = latent_a.blend(&latent_b, blend);
    println!("Blended ({:.0}% A, {:.0}% B):", (1.0 - blend) * 100.0, blend * 100.0);
    println!("{}", blended.describe());
}

