//! Source-oriented errors shared by the compiler stages.

use std::{fmt, path::Path};

/// A diagnostic that owns its message and can outlive any parsed source buffer.
#[derive(Debug)]
pub struct Error(pub String);

/// Compiler operations fail with one actionable diagnostic rather than partial output.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Attach a one-based source line to a syntax or semantic failure.
    pub fn at(path: &Path, line: usize, message: impl fmt::Display) -> Self {
        Self(format!("{}:{line}: {message}", path.display()))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for Error {}
