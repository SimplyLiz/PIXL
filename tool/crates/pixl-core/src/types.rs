use serde::Deserialize;
use std::collections::HashMap;

// ── Top-level PAX file ──────────────────────────────────────────────

#[derive(Debug, Deserialize, serde::Serialize)]
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
    #[serde(default)]
    pub anim_clock: HashMap<String, AnimClock>,
    #[serde(default)]
    pub tilemap: HashMap<String, crate::tilemap::TilemapRaw>,
    #[serde(default)]
    pub palette_ext: HashMap<String, PaletteExtRaw>,
    #[serde(default)]
    pub backdrop_tile: HashMap<String, BackdropTileRaw>,
    #[serde(default)]
    pub backdrop: HashMap<String, BackdropRaw>,
    #[serde(default)]
    pub composite: HashMap<String, CompositeRaw>,
    // PAX 2.1: style latent section — optional statistical fingerprint
    #[serde(default)]
    pub style: HashMap<String, crate::style::StyleLatent>,
}

// ── Header ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct Header {
    pub version: String,
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub theme: Option<String>,
    /// Color profile: "srgb" (default) | "linear" | "display-p3"
    #[serde(default)]
    pub color_profile: Option<String>,
}

// ── Theme ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct Theme {
    pub palette: String,
    #[serde(default)]
    pub scale: Option<u32>,
    #[serde(default)]
    pub canvas: Option<u32>,
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

fn default_scale() -> u32 {
    1
}
fn default_canvas() -> u32 {
    16
}

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

#[derive(Debug, Deserialize, serde::Serialize)]
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

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct Cycle {
    pub palette: String,
    pub symbols: Vec<String>, // cycle through these symbols' colors (not indices)
    #[serde(default = "default_direction")]
    pub direction: String,
    #[serde(default = "default_fps")]
    pub fps: u32,
}

fn default_direction() -> String {
    "forward".to_string()
}
fn default_fps() -> u32 {
    8
}

// ── Animation Clock (Neo Geo auto-animation) ───────────────────────

/// Global animation clock that tiles can opt into.
/// All tiles sharing a clock stay perfectly synchronized.
/// Tiles cycle through `{name}_0`..`{name}_{frames-1}` companion tiles.
#[derive(Debug, Deserialize, serde::Serialize)]
pub struct AnimClock {
    #[serde(default = "default_anim_fps")]
    pub fps: u32,
    #[serde(default = "default_anim_frames")]
    pub frames: u32,
    /// "loop" | "ping-pong"
    #[serde(default = "default_anim_mode")]
    pub mode: String,
}

fn default_anim_fps() -> u32 {
    6
}
fn default_anim_frames() -> u32 {
    4
}
fn default_anim_mode() -> String {
    "loop".to_string()
}

// ── Stamp ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, serde::Serialize)]
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

#[derive(Debug, Deserialize, serde::Serialize)]
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
    pub auto_rotate_weight: Option<String>, // "equal" | "source_only" (default)
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub edge_class: Option<EdgeClassRaw>,
    #[serde(default)]
    pub corner_class: Option<CornerClassRaw>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Target tilemap layer for AI-driven placement (e.g. "terrain", "walls", "effects").
    #[serde(default)]
    pub target_layer: Option<String>,
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
    // Grid data — exactly one of these should be present (or template/delta)
    #[serde(default)]
    pub grid: Option<String>,
    #[serde(default)]
    pub rle: Option<String>,
    #[serde(default)]
    pub layout: Option<String>,
    // PAX 2.1: pattern fill encoding — tiles the pattern to fill declared size
    #[serde(default)]
    pub fill: Option<String>,
    #[serde(default)]
    pub fill_size: Option<String>,
    // PAX 2.1: delta encoding — inherit grid from base tile, apply pixel patches
    #[serde(default)]
    pub delta: Option<String>,
    #[serde(default)]
    pub patches: Vec<PatchRaw>,
}

fn default_weight() -> f64 {
    1.0
}

/// PAX 2.1: pixel override for delta tiles — `{ x, y, sym }`.
#[derive(Debug, Deserialize, Clone, serde::Serialize)]
pub struct PatchRaw {
    pub x: u32,
    pub y: u32,
    pub sym: String,
}

#[derive(Debug, Deserialize, Clone, serde::Serialize)]
pub struct EdgeClassRaw {
    pub n: String,
    pub e: String,
    pub s: String,
    pub w: String,
}

/// Optional corner classes for 8-neighbor WFC terrain matching (Godot-style).
#[derive(Debug, Deserialize, Clone, serde::Serialize)]
pub struct CornerClassRaw {
    #[serde(default)]
    pub ne: Option<String>,
    #[serde(default)]
    pub se: Option<String>,
    #[serde(default)]
    pub sw: Option<String>,
    #[serde(default)]
    pub nw: Option<String>,
}

#[derive(Debug, Deserialize, Clone, serde::Serialize)]
pub struct NineSlice {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
}

#[derive(Debug, Deserialize, Clone, serde::Serialize)]
pub struct SemanticRaw {
    #[serde(default)]
    pub affordance: Option<String>,
    #[serde(default)]
    pub collision: Option<String>,
    /// Polygon collision points for "polygon" collision type: [[x1,y1], [x2,y2], ...]
    #[serde(default)]
    pub collision_points: Option<Vec<Vec<u32>>>,
    #[serde(default)]
    pub tags: HashMap<String, toml::Value>, // mixed types: bool, string, int
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
    /// Target tilemap layer for AI-driven placement.
    /// Values: "background", "terrain", "walls", "platform", "foreground", "effects".
    pub target_layer: Option<String>,
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
    Fill,  // PAX 2.1: pattern tiling
    Delta, // PAX 2.1: base tile + pixel patches
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

/// Resolved corner classes for 8-neighbor WFC terrain matching.
#[derive(Debug, Clone, Default)]
pub struct CornerClass {
    pub ne: Option<String>,
    pub se: Option<String>,
    pub sw: Option<String>,
    pub nw: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Semantic {
    pub affordance: String,
    pub collision: CollisionShape,
    pub collision_points: Option<Vec<(u32, u32)>>,
    pub tags: HashMap<String, toml::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollisionShape {
    Full,
    None,
    HalfTop,
    SlopeNe,
    SlopeNw,
    SlopeSe,
    SlopeSw,
    Polygon,
    Custom,
}

// ── Spriteset ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize, serde::Serialize)]
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

#[derive(Debug, Deserialize, serde::Serialize)]
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
    /// Neo Geo/Super Scaler-style scale factor (0.0-1.0 = shrink, >1.0 = enlarge).
    #[serde(default)]
    pub scale: Option<f64>,
}

fn default_loop() -> bool {
    true
}

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct AnimTagRaw {
    pub name: String,
    pub from_frame: u32,
    pub to_frame: u32,
}

#[derive(Debug, Deserialize, serde::Serialize)]
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

#[derive(Debug, Deserialize, Clone, serde::Serialize)]
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

#[derive(Debug, Deserialize, serde::Serialize)]
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

// ── Composite Sprites (multi-tile assembly) ─────────────────────────

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct CompositeRaw {
    pub size: String,
    pub tile_size: String,
    pub layout: String,
    #[serde(default)]
    pub offset: HashMap<String, Vec<i32>>,
    #[serde(default)]
    pub variant: HashMap<String, CompositeVariantRaw>,
    #[serde(default)]
    pub anim: HashMap<String, CompositeAnimRaw>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct CompositeVariantRaw {
    pub slot: HashMap<String, String>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct CompositeAnimRaw {
    #[serde(default = "default_composite_fps")]
    pub fps: u32,
    #[serde(default = "default_true")]
    pub r#loop: bool,
    #[serde(default)]
    pub mirror: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub frame: Vec<CompositeFrameRaw>,
}

fn default_composite_fps() -> u32 {
    8
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct CompositeFrameRaw {
    pub index: u32,
    #[serde(default)]
    pub swap: HashMap<String, String>,
    #[serde(default)]
    pub offset: HashMap<String, Vec<i32>>,
}

#[derive(Debug, Clone)]
pub struct Composite {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub cols: u32,
    pub rows: u32,
    pub slots: Vec<Vec<TileRef>>,
    pub offsets: HashMap<(u32, u32), (i32, i32)>,
    pub variants: HashMap<String, HashMap<(u32, u32), TileRef>>,
    pub animations: HashMap<String, CompositeAnim>,
}

#[derive(Debug, Clone)]
pub struct CompositeAnim {
    pub fps: u32,
    pub loop_mode: bool,
    pub mirror: Option<String>,
    pub source: Option<String>,
    pub frames: Vec<CompositeFrame>,
}

#[derive(Debug, Clone)]
pub struct CompositeFrame {
    pub index: u32,
    pub swaps: HashMap<(u32, u32), TileRef>,
    pub offsets: HashMap<(u32, u32), (i32, i32)>,
}

// ── Tile Run Groups ─────────────────────────────────────────────────

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct TileRun {
    #[serde(default = "default_orientation")]
    pub orientation: String,
    pub left: String,
    pub middle: String,
    pub right: String,
    #[serde(default)]
    pub single: Option<String>,
}

fn default_orientation() -> String {
    "horizontal".to_string()
}

// ── WFC Rules ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct WfcRules {
    #[serde(default)]
    pub forbids: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default = "default_boost")]
    pub require_boost: f64,
    #[serde(default)]
    pub variant_groups: HashMap<String, Vec<String>>,
    /// When true, the tileset has been verified as sub-complete by
    /// `pixl check --subcomplete`. WFC can skip backtracking.
    #[serde(default)]
    pub subcomplete: bool,
}

fn default_boost() -> f64 {
    3.0
}

// ── Atlas Config ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, serde::Serialize)]
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

fn default_format() -> String {
    "texturepacker".to_string()
}
fn default_padding() -> u32 {
    1
}
fn default_atlas_scale() -> u32 {
    1
}
fn default_columns() -> u32 {
    8
}

// ── Extended Palette ────────────────────────────────────────────────

/// Extended palette for 17-48 colors. Multi-char symbols (e.g. "2a", "3f")
/// used only in RLE encoding where tokens are whitespace-separated.
#[derive(Debug, Deserialize, serde::Serialize)]
pub struct PaletteExtRaw {
    pub base: String,
    #[serde(flatten)]
    pub symbols: HashMap<String, String>, // "2a" => "#rrggbbaa"
}

/// Resolved extended palette: merges base single-char palette + multi-char extensions.
#[derive(Debug, Clone)]
pub struct PaletteExt {
    pub base: HashMap<char, Rgba>,
    pub extended: HashMap<String, Rgba>,
}

// ── Backdrop Tile ──────────────────────────────────────────────────

/// Lightweight tile for backdrop composition — no edge/WFC metadata.
#[derive(Debug, Deserialize, serde::Serialize)]
pub struct BackdropTileRaw {
    pub palette: String,
    #[serde(default)]
    pub palette_ext: Option<String>,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub grid: Option<String>,
    #[serde(default)]
    pub rle: Option<String>,
    /// Frame-based animation: list of (tile_name, duration_ms) pairs.
    #[serde(default)]
    pub animation: Vec<BackdropTileFrameRaw>,
    /// Neo Geo-style global animation clock reference.
    /// Tile cycles through `{name}_0`..`{name}_{frames-1}` companions.
    #[serde(default)]
    pub anim_clock: Option<String>,
}

/// A single frame in an animated backdrop tile.
#[derive(Debug, Deserialize, serde::Serialize)]
pub struct BackdropTileFrameRaw {
    pub tile: String,
    #[serde(default = "default_frame_duration")]
    pub duration_ms: u32,
}

fn default_frame_duration() -> u32 {
    120
}

// ── Backdrop Scene ─────────────────────────────────────────────────

/// Large composed background scene with procedural animation zones.
/// Supports either a single `tilemap` (backward compatible) or multiple `layer`s.
#[derive(Debug, Deserialize, serde::Serialize)]
pub struct BackdropRaw {
    pub palette: String,
    #[serde(default)]
    pub palette_ext: Option<String>,
    pub size: String,
    #[serde(default = "default_tile_size")]
    pub tile_size: String,
    /// Single-layer tilemap (backward compatible). Ignored if `layer` is present.
    #[serde(default)]
    pub tilemap: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub zone: Vec<BackdropZoneRaw>,
    /// Multi-layer support: each layer has its own tilemap, scroll factor, blend mode.
    #[serde(default)]
    pub layer: Vec<BackdropLayerRaw>,
}

fn default_tile_size() -> String {
    "16x16".to_string()
}

/// A single parallax layer in a backdrop.
#[derive(Debug, Deserialize, serde::Serialize)]
pub struct BackdropLayerRaw {
    pub name: String,
    pub tilemap: String,
    /// Parallax scroll factor: 0.0 = fixed (far), 1.0 = moves with camera (near).
    #[serde(default = "default_scroll_factor")]
    pub scroll_factor: f64,
    /// Layer opacity (0.0 = transparent, 1.0 = opaque).
    #[serde(default = "default_opacity")]
    pub opacity: f64,
    /// Blend mode: "normal", "additive", "multiply", "screen".
    #[serde(default = "default_blend_mode")]
    pub blend: String,
    /// Pixel offset.
    #[serde(default)]
    pub offset_x: i32,
    #[serde(default)]
    pub offset_y: i32,
    /// GBA BLDY-style fade: { target = "black"|"white", amount = 0.0-1.0 }
    #[serde(default)]
    pub fade: Option<BackdropFade>,
    /// Genesis Window Plane: rectangular region that doesn't scroll with parallax.
    #[serde(default)]
    pub scroll_lock: Option<ZoneRect>,
}

/// Fade-to-black or fade-to-white on a layer (GBA BLDY register equivalent).
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct BackdropFade {
    /// "black" or "white"
    pub target: String,
    /// 0.0 = no fade, 1.0 = fully faded
    #[serde(default)]
    pub amount: f64,
}

fn default_scroll_factor() -> f64 {
    1.0
}
fn default_opacity() -> f64 {
    1.0
}
fn default_blend_mode() -> String {
    "normal".to_string()
}

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct BackdropZoneRaw {
    pub name: String,
    pub rect: ZoneRect,
    pub behavior: String, // "cycle", "wave", "flicker", "scroll_down", "hscroll_sine", "color_gradient", "mosaic", "window"
    #[serde(default)]
    pub cycle: Option<String>,
    #[serde(default)]
    pub speed: Option<f64>,
    #[serde(default)]
    pub wrap: Option<bool>,
    #[serde(default)]
    pub density: Option<f64>,
    #[serde(default)]
    pub seed: Option<u64>,
    #[serde(default)]
    pub phase_rows: Option<u32>,
    #[serde(default)]
    pub wave_dx: Option<i32>,
    /// Which layer this zone applies to (default: all layers).
    #[serde(default)]
    pub layer: Option<String>,
    // ── Scanline effect params (SNES HDMA-style) ──
    /// Horizontal sine wave amplitude in pixels.
    #[serde(default)]
    pub amplitude: Option<u32>,
    /// Sine wave period in scanlines.
    #[serde(default)]
    pub period: Option<u32>,
    // ── Color gradient params ──
    /// Gradient start color (hex).
    #[serde(default)]
    pub from: Option<String>,
    /// Gradient end color (hex).
    #[serde(default)]
    pub to: Option<String>,
    /// Gradient direction: "vertical" or "horizontal".
    #[serde(default)]
    pub direction: Option<String>,
    // ── Mosaic params (GBA-style, independent X/Y) ──
    /// Mosaic block width in pixels.
    #[serde(default)]
    pub size_x: Option<u32>,
    /// Mosaic block height in pixels.
    #[serde(default)]
    pub size_y: Option<u32>,
    // ── Window params (GBA WIN0/WIN1-style) ──
    /// Which layers are visible inside this window.
    #[serde(default)]
    pub layers_visible: Option<Vec<String>>,
    /// Override blend mode inside window.
    #[serde(default)]
    pub blend_override: Option<String>,
    /// Override opacity inside window.
    #[serde(default)]
    pub opacity_override: Option<f64>,
    // ── Palette ramp params (Konami raster) ──
    /// Which palette symbol to ramp across the zone.
    #[serde(default)]
    pub symbol: Option<String>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct ZoneRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Resolved backdrop ready for rendering.
#[derive(Debug, Clone)]
pub struct Backdrop {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub cols: u32,
    pub rows: u32,
    pub layers: Vec<BackdropLayer>,
    pub zones: Vec<BackdropZone>,
}

/// Resolved layer with parsed tilemap entries.
#[derive(Debug, Clone)]
pub struct BackdropLayer {
    pub name: String,
    pub tilemap: Vec<Vec<TileRef>>,
    pub scroll_factor: f64,
    pub opacity: f64,
    pub blend: BlendMode,
    pub offset_x: i32,
    pub offset_y: i32,
    /// GBA BLDY-style fade.
    pub fade: Option<(FadeTarget, f64)>,
    /// Genesis Window Plane: viewport-pinned region that ignores scroll.
    pub scroll_lock: Option<ZoneRect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadeTarget {
    Black,
    White,
}

/// A reference to a tile in the tilemap, with optional flip flags and brightness modifier.
///
/// Format: `tile_name[!flags][:modifier]`
/// - Flip flags: `!h` (horizontal), `!v` (vertical), `!d` (diagonal/transpose)
/// - Modifiers: `:shadow` (halve RGB), `:highlight` (halve + midpoint)
///
/// Examples: `wall`, `wall!h`, `wall!hv:shadow`, `floor:highlight`
#[derive(Debug, Clone)]
pub struct TileRef {
    pub name: String,
    pub flip_h: bool,
    pub flip_v: bool,
    pub flip_d: bool,
    pub modifier: TileModifier,
}

/// Genesis VDP-inspired per-tile brightness modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileModifier {
    None,
    /// RGB_out = RGB / 2 (darken)
    Shadow,
    /// RGB_out = RGB / 2 + 128 (brighten)
    Highlight,
}

impl TileRef {
    pub fn parse(s: &str) -> Self {
        // Split off modifier first (after last ':')
        let (rest, modifier) = if let Some(colon) = s.rfind(':') {
            let mod_str = &s[colon + 1..];
            let m = match mod_str {
                "shadow" | "s" => TileModifier::Shadow,
                "highlight" | "hi" => TileModifier::Highlight,
                _ => TileModifier::None,
            };
            if m != TileModifier::None {
                (&s[..colon], m)
            } else {
                (s, TileModifier::None)
            }
        } else {
            (s, TileModifier::None)
        };

        // Split off flip flags
        if let Some(bang) = rest.find('!') {
            let name = rest[..bang].to_string();
            let flags = &rest[bang + 1..];
            TileRef {
                name,
                flip_h: flags.contains('h'),
                flip_v: flags.contains('v'),
                flip_d: flags.contains('d'),
                modifier,
            }
        } else {
            TileRef {
                name: rest.to_string(),
                flip_h: false,
                flip_v: false,
                flip_d: false,
                modifier,
            }
        }
    }

    pub fn has_flip(&self) -> bool {
        self.flip_h || self.flip_v || self.flip_d
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Normal,
    Additive,
    Multiply,
    Screen,
}

impl BlendMode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "additive" | "add" => BlendMode::Additive,
            "multiply" | "mul" => BlendMode::Multiply,
            "screen" => BlendMode::Screen,
            _ => BlendMode::Normal,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BackdropZone {
    pub name: String,
    pub rect: ZoneRect,
    pub behavior: ZoneBehavior,
    pub layer: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ZoneBehavior {
    Cycle {
        cycle: String,
    },
    Wave {
        cycle: String,
        phase_rows: u32,
        wave_dx: i32,
    },
    Flicker {
        cycle: String,
        density: f64,
        seed: u64,
    },
    ScrollDown {
        speed: f64,
        wrap: bool,
    },
    /// SNES HDMA-style sinusoidal horizontal scroll per scanline.
    HScrollSine {
        amplitude: u32,
        period: u32,
        speed: f64,
    },
    /// Per-pixel color tint gradient across the zone rect.
    ColorGradient {
        from: Rgba,
        to: Rgba,
        vertical: bool,
    },
    /// GBA-style pixelation with independent X/Y block sizes.
    Mosaic {
        size_x: u32,
        size_y: u32,
    },
    /// GBA WIN0/WIN1-style rendering window: control layer visibility + effects.
    Window {
        layers_visible: Vec<String>,
        blend_override: Option<BlendMode>,
        opacity_override: Option<f64>,
    },
    /// Genesis VSRAM-style per-column vertical scroll with sine offset.
    VScrollSine {
        amplitude: u32,
        period: u32,
        speed: f64,
    },
    /// Konami raster-style per-scanline palette entry interpolation.
    PaletteRamp {
        symbol: String,
        from: Rgba,
        to: Rgba,
    },
}

// ── Size parsing helper ─────────────────────────────────────────────

pub fn parse_size(s: &str) -> Result<(u32, u32), String> {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() != 2 {
        return Err(format!("invalid size '{}': expected 'WxH'", s));
    }
    let w = parts[0]
        .parse::<u32>()
        .map_err(|_| format!("invalid width in '{}'", s))?;
    let h = parts[1]
        .parse::<u32>()
        .map_err(|_| format!("invalid height in '{}'", s))?;
    if w == 0 || h == 0 {
        return Err(format!(
            "invalid size '{}': width and height must be > 0",
            s
        ));
    }
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
