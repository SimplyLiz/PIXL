use serde::Deserialize;
use std::collections::HashMap;

// ── Top-level PAX file ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PaxFile {
    pub pax: Header,
    #[serde(default)]
    pub theme: HashMap<String, Theme>,
    #[serde(default)]
    pub palette: HashMap<String, PaletteRaw>,
    #[serde(default)]
    pub palette_swap: HashMap<String, PaletteSwap>,
    #[serde(default)]
    pub cycle: HashMap<String, Cycle>,
    #[serde(default)]
    pub stamp: HashMap<String, StampRaw>,
    #[serde(default)]
    pub tile: HashMap<String, TileRaw>,
    #[serde(default)]
    pub spriteset: HashMap<String, SpritesetRaw>,
    #[serde(default)]
    pub object: HashMap<String, ObjectRaw>,
    #[serde(default)]
    pub tile_run: HashMap<String, TileRun>,
    #[serde(default)]
    pub wfc_rules: Option<WfcRules>,
    #[serde(default)]
    pub atlas: Option<AtlasConfig>,
}

// ── Header ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct Header {
    pub version: String,
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub theme: Option<String>,
}

// ── Theme ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Theme {
    pub palette: String,
    #[serde(default = "default_scale")]
    pub scale: u32,
    #[serde(default = "default_canvas")]
    pub canvas: u32,
    #[serde(default)]
    pub max_palette_size: Option<u32>,
    #[serde(default)]
    pub light_source: Option<String>,
    #[serde(default)]
    pub extends: Option<String>,
    #[serde(default)]
    pub roles: HashMap<String, String>,
    #[serde(default)]
    pub constraints: HashMap<String, toml::Value>,
}

fn default_scale() -> u32 { 1 }
fn default_canvas() -> u32 { 16 }

// ── Palette ─────────────────────────────────────────────────────────

/// Raw palette as deserialized from TOML: HashMap<String, String>
/// Must be converted to `Palette` via `resolve_palette()`
pub type PaletteRaw = HashMap<String, String>;

#[derive(Debug, Clone)]
pub struct Palette {
    pub symbols: HashMap<char, Rgba>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

// ── Palette Swap ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PaletteSwap {
    pub base: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub partial: bool,
    #[serde(default)]
    pub map: HashMap<String, String>,
}

// ── Color Cycle ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct Cycle {
    pub palette: String,
    pub symbols: Vec<String>,       // cycle through these symbols' colors (not indices)
    #[serde(default = "default_direction")]
    pub direction: String,
    #[serde(default = "default_fps")]
    pub fps: u32,
}

fn default_direction() -> String { "forward".to_string() }
fn default_fps() -> u32 { 8 }

// ── Stamp ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StampRaw {
    pub palette: String,
    pub size: String,
    pub grid: String,
}

#[derive(Debug, Clone)]
pub struct Stamp {
    pub palette: String,
    pub width: u32,
    pub height: u32,
    pub grid: Vec<Vec<char>>,
}

// ── Tile ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TileRaw {
    pub palette: String,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub encoding: Option<String>,
    #[serde(default)]
    pub symmetry: Option<String>,
    #[serde(default)]
    pub auto_rotate: Option<String>,
    #[serde(default)]
    pub auto_rotate_weight: Option<String>,  // "equal" | "source_only" (default)
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub edge_class: Option<EdgeClassRaw>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_weight")]
    pub weight: f64,
    #[serde(default)]
    pub palette_swaps: Vec<String>,
    #[serde(default)]
    pub cycles: Vec<String>,
    #[serde(default)]
    pub nine_slice: Option<NineSlice>,
    #[serde(default)]
    pub visual_height_extra: Option<u32>,
    #[serde(default)]
    pub semantic: Option<SemanticRaw>,
    // Grid data — exactly one of these should be present (or template)
    #[serde(default)]
    pub grid: Option<String>,
    #[serde(default)]
    pub rle: Option<String>,
    #[serde(default)]
    pub layout: Option<String>,
}

fn default_weight() -> f64 { 1.0 }

#[derive(Debug, Deserialize, Clone)]
pub struct EdgeClassRaw {
    pub n: String,
    pub e: String,
    pub s: String,
    pub w: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NineSlice {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SemanticRaw {
    #[serde(default)]
    pub affordance: Option<String>,
    #[serde(default)]
    pub collision: Option<String>,
    #[serde(default)]
    pub tags: HashMap<String, toml::Value>,  // mixed types: bool, string, int
}

// ── Resolved Tile (after parsing grid/rle/compose/template) ─────────

#[derive(Debug, Clone)]
pub struct Tile {
    pub name: String,
    pub palette: String,
    pub width: u32,
    pub height: u32,
    pub encoding: Encoding,
    pub symmetry: Symmetry,
    pub auto_rotate: AutoRotate,
    pub edge_class: EdgeClass,
    pub tags: Vec<String>,
    pub weight: f64,
    pub palette_swaps: Vec<String>,
    pub cycles: Vec<String>,
    pub nine_slice: Option<NineSlice>,
    pub visual_height_extra: Option<u32>,
    pub semantic: Option<Semantic>,
    pub grid: Vec<Vec<char>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Grid,
    Rle,
    Compose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symmetry {
    None,
    Horizontal,
    Vertical,
    Quad,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoRotate {
    None,
    FourWay,
    Flip,
    EightWay,
}

#[derive(Debug, Clone)]
pub struct EdgeClass {
    pub n: String,
    pub e: String,
    pub s: String,
    pub w: String,
}

#[derive(Debug, Clone)]
pub struct Semantic {
    pub affordance: String,
    pub collision: CollisionShape,
    pub tags: HashMap<String, toml::Value>,  // mixed types: bool, string, int
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionShape {
    Full,
    None,
    HalfTop,
    SlopeNe,
    SlopeNw,
    SlopeSe,
    SlopeSw,
    Custom,
}

// ── Spriteset ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SpritesetRaw {
    pub palette: String,
    pub size: String,
    #[serde(default)]
    pub palette_swaps: Vec<String>,
    #[serde(default)]
    pub cycles: Vec<String>,
    #[serde(default)]
    pub sprite: Vec<SpriteRaw>,
}

#[derive(Debug, Deserialize)]
pub struct SpriteRaw {
    pub name: String,
    #[serde(default = "default_fps")]
    pub fps: u32,
    #[serde(default = "default_loop")]
    pub r#loop: bool,
    #[serde(default)]
    pub tags: Vec<AnimTagRaw>,
    #[serde(default)]
    pub frames: Vec<FrameRaw>,
}

fn default_loop() -> bool { true }

#[derive(Debug, Deserialize)]
pub struct AnimTagRaw {
    pub name: String,
    pub from_frame: u32,
    pub to_frame: u32,
}

#[derive(Debug, Deserialize)]
pub struct FrameRaw {
    pub index: u32,
    #[serde(default)]
    pub encoding: Option<String>,
    #[serde(default)]
    pub grid: Option<String>,
    #[serde(default)]
    pub base: Option<u32>,
    #[serde(default)]
    pub changes: Vec<DeltaChange>,
    #[serde(default)]
    pub link_to: Option<u32>,
    #[serde(default)]
    pub duration_ms: Option<u32>,
    #[serde(default)]
    pub mirror: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeltaChange {
    pub x: u32,
    pub y: u32,
    pub sym: String,
}

// ── Resolved animation types ────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Spriteset {
    pub name: String,
    pub palette: String,
    pub width: u32,
    pub height: u32,
    pub palette_swaps: Vec<String>,
    pub cycles: Vec<String>,
    pub sprites: Vec<Sprite>,
}

#[derive(Debug, Clone)]
pub struct Sprite {
    pub name: String,
    pub fps: u32,
    pub loop_mode: bool,
    pub tags: Vec<AnimTag>,
    pub frames: Vec<Frame>,
}

#[derive(Debug, Clone)]
pub struct AnimTag {
    pub name: String,
    pub from_frame: u32,
    pub to_frame: u32,
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub index: u32,
    pub duration_ms: Option<u32>,
    pub grid: Vec<Vec<char>>,
}

// ── Multi-tile Objects ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ObjectRaw {
    pub size_tiles: String,
    #[serde(default)]
    pub base_tile: Option<String>,
    #[serde(default)]
    pub above_player_rows: Vec<u32>,
    #[serde(default)]
    pub below_player_rows: Vec<u32>,
    pub tiles: String,
    #[serde(default)]
    pub collision: Option<String>,
}

// ── Tile Run Groups ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TileRun {
    #[serde(default = "default_orientation")]
    pub orientation: String,
    pub left: String,
    pub middle: String,
    pub right: String,
    #[serde(default)]
    pub single: Option<String>,
}

fn default_orientation() -> String { "horizontal".to_string() }

// ── WFC Rules ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct WfcRules {
    #[serde(default)]
    pub forbids: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default = "default_boost")]
    pub require_boost: f64,
    #[serde(default)]
    pub variant_groups: HashMap<String, Vec<String>>,
}

fn default_boost() -> f64 { 3.0 }

// ── Atlas Config ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AtlasConfig {
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_padding")]
    pub padding: u32,
    #[serde(default = "default_atlas_scale")]
    pub scale: u32,
    #[serde(default = "default_columns")]
    pub columns: u32,
    #[serde(default)]
    pub include: Vec<String>,
    pub output: String,
    #[serde(default)]
    pub map_output: Option<String>,
}

fn default_format() -> String { "texturepacker".to_string() }
fn default_padding() -> u32 { 1 }
fn default_atlas_scale() -> u32 { 1 }
fn default_columns() -> u32 { 8 }

// ── Size parsing helper ─────────────────────────────────────────────

pub fn parse_size(s: &str) -> Result<(u32, u32), String> {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() != 2 {
        return Err(format!("invalid size '{}': expected 'WxH'", s));
    }
    let w = parts[0].parse::<u32>().map_err(|_| format!("invalid width in '{}'", s))?;
    let h = parts[1].parse::<u32>().map_err(|_| format!("invalid height in '{}'", s))?;
    Ok((w, h))
}

// ── Color parsing helper ────────────────────────────────────────────

impl Rgba {
    pub fn from_hex(hex: &str) -> Result<Self, String> {
        let hex = hex.trim_start_matches('#');
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16)
                    .map_err(|_| format!("invalid hex color: #{}", hex))?;
                let g = u8::from_str_radix(&hex[2..4], 16)
                    .map_err(|_| format!("invalid hex color: #{}", hex))?;
                let b = u8::from_str_radix(&hex[4..6], 16)
                    .map_err(|_| format!("invalid hex color: #{}", hex))?;
                Ok(Rgba { r, g, b, a: 255 })
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16)
                    .map_err(|_| format!("invalid hex color: #{}", hex))?;
                let g = u8::from_str_radix(&hex[2..4], 16)
                    .map_err(|_| format!("invalid hex color: #{}", hex))?;
                let b = u8::from_str_radix(&hex[4..6], 16)
                    .map_err(|_| format!("invalid hex color: #{}", hex))?;
                let a = u8::from_str_radix(&hex[6..8], 16)
                    .map_err(|_| format!("invalid hex color: #{}", hex))?;
                Ok(Rgba { r, g, b, a })
            }
            _ => Err(format!(
                "invalid hex color '#{hex}': expected 6 or 8 hex digits"
            )),
        }
    }
}
