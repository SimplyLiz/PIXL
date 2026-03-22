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
