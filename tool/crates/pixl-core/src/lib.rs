//! # pixl-core
//!
//! Core library for the [PIXL](https://github.com/SimplyLiz/PIXL) toolchain.
//!
//! `pixl-core` provides types, parser, validator, and utilities for working with
//! `.pax` files — a TOML-based pixel art tileset format designed for LLM-assisted
//! game development.
//!
//! ## Usage
//!
//! ```rust,no_run
//! let source = std::fs::read_to_string("tileset.pax").unwrap();
//! let pax = pixl_core::parser::parse(&source).unwrap();
//! let errors = pixl_core::validate::validate(&pax);
//! ```
//!
//! ## Modules
//!
//! - [`parser`] — Parse `.pax` TOML into typed structures
//! - [`types`] — Core data types (`Pax`, `Tile`, `Palette`, `Theme`, etc.)
//! - [`validate`] — Validate tileset consistency (edges, palettes, sizes)
//! - [`edges`] — Edge class extraction and compatibility checking
//! - [`grid`] / [`rle`] / [`compose`] — Three-tier tile encoding
//! - [`style`] — Style latent extraction and scoring (OKLab color space)
//! - [`blueprint`] — Anatomy-guided character sprite coordinates
//! - [`theme`] — Built-in theme library
//! - [`tilemap`] — 2D tilemap data structures
//! - [`animate`] — Sprite animation pipeline
//! - [`completeness`] — Tileset gap analysis for WFC

pub mod animate;
pub mod blueprint;
pub mod completeness;
pub mod compose;
pub mod composite;
pub mod corpus;
pub mod cycle;
pub mod edges;
pub mod feedback;
pub mod grid;
pub mod knowledge;
pub mod oklab;
pub mod parser;
pub mod project;
pub mod resolve;
pub mod rle;
pub mod rotate;
pub mod skeleton;
pub mod stampgen;
pub mod style;
pub mod symmetry;
pub mod template;
pub mod theme;
pub mod tilemap;
pub mod types;
pub mod validate;
pub mod vary;
