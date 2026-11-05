//! Compile owned Markdown documents into a validated, deterministic static website.
//!
//! Parsing, reference resolution, rendering, and filesystem publication are separate
//! boundaries. A failed compilation never modifies the published output directory.

pub mod build;
pub mod error;
pub mod markdown;
pub mod model;
pub mod render;
pub mod resolve;

pub use build::Build;
pub use error::{Error, Result};
pub use resolve::Site;
