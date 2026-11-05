//! Validate emitted links and publish complete builds through a recoverable rename.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use scraper::{Html, Selector};

use crate::{Error, Result};

/// A complete output snapshot, owned independently of its source site.
pub struct Build {
    /// Relative output paths and exact bytes, ordered for deterministic publication.
    pub(crate) files: BTreeMap<PathBuf, Vec<u8>>,
    /// Source root used to prevent overwriting compiler, content, or asset inputs.
    pub(crate) source_root: PathBuf,
}

/// Identifies directories that the compiler may replace on subsequent builds.
const MARKER: &str = ".blog-compiler-output";

impl Build {
    /// Inspect output bytes without writing a directory, useful for consumers/tests.
    pub fn files(&self) -> &BTreeMap<PathBuf, Vec<u8>> {
        &self.files
    }

    /// Check IDs, document structure, and local href/src targets in generated HTML.
    /// Assets are opaque bytes; external destinations deliberately require no network.
    pub fn validate(&self) -> Result<()> {
        let id_selector = Selector::parse("[id]").unwrap();
        let link_selector = Selector::parse("[href], [src]").unwrap();
        let main_selector = Selector::parse("main").unwrap();
        let heading_selector = Selector::parse("h1").unwrap();
        let mut documents = BTreeMap::new();
        let mut ids = BTreeMap::new();
        for (path, bytes) in &self.files {
            if path.extension().is_none_or(|value| value != "html") {
                continue;
            }
            let text = std::str::from_utf8(bytes).map_err(|error| Error(error.to_string()))?;
            let html = Html::parse_document(text);
            if html.select(&main_selector).count() != 1
                || html.select(&heading_selector).count() != 1
            {
                return Err(Error(format!(
                    "{}: generated pages require exactly one main and one h1",
                    path.display()
                )));
            }
            let mut anchors = BTreeSet::new();
            for node in html.select(&id_selector) {
                let id = node.value().attr("id").unwrap().to_owned();
                if !anchors.insert(id.clone()) {
                    return Err(Error(format!(
                        "{}: duplicate anchor {id:?}",
                        path.display()
                    )));
                }
            }
            ids.insert(path.clone(), anchors);
            documents.insert(path.clone(), html);
        }
        for (path, html) in &documents {
            let base = url::Url::parse(&format!(
                "https://compiler.invalid/{}",
                path.to_string_lossy()
            ))
            .map_err(|error| Error(error.to_string()))?;
            for node in html.select(&link_selector) {
                for attribute in ["href", "src"] {
                    let Some(reference) = node.value().attr(attribute) else {
                        continue;
                    };
                    if reference.starts_with("//") {
                        return Err(Error(format!(
                            "{}: protocol-relative URL {reference:?}",
                            path.display()
                        )));
                    }
                    if let Ok(absolute) = url::Url::parse(reference) {
                        if matches!(absolute.scheme(), "http" | "https" | "mailto" | "tel") {
                            continue;
                        }
                        return Err(Error(format!(
                            "{}: unsupported URL {reference:?}",
                            path.display()
                        )));
                    }
                    let url = base
                        .join(reference)
                        .map_err(|error| Error(format!("{}: {error}", path.display())))?;
                    let decoded = percent_encoding::percent_decode_str(url.path())
                        .decode_utf8()
                        .map_err(|error| Error(error.to_string()))?;
                    let target = PathBuf::from(decoded.trim_start_matches('/'));
                    if !self.files.contains_key(&target) {
                        return Err(Error(format!(
                            "{}: missing local target {reference:?}",
                            path.display()
                        )));
                    }
                    if let Some(fragment) = url.fragment().filter(|value| !value.is_empty()) {
                        let fragment = percent_encoding::percent_decode_str(fragment)
                            .decode_utf8()
                            .map_err(|error| Error(error.to_string()))?;
                        if let Some(anchors) = ids.get(&target)
                            && !anchors.contains(fragment.as_ref())
                        {
                            return Err(Error(format!(
                                "{}: missing anchor in {reference:?}",
                                path.display()
                            )));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Publish a fully validated build. Existing compiler output is recoverable
    /// until staging has replaced it; unrelated nonempty directories are refused.
    pub fn write(&self, output: impl AsRef<Path>) -> Result<()> {
        self.validate()?;
        let output = output.as_ref();
        let name = output
            .file_name()
            .filter(|name| !name.is_empty())
            .ok_or_else(|| Error("output must name a directory".into()))?;
        let parent = output
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let parent = fs::canonicalize(parent)
            .map_err(|error| Error(format!("output parent {}: {error}", parent.display())))?;
        let destination = parent.join(name);
        // A dedicated output child may never replace a source path or contain it.
        if self.source_root.starts_with(&destination)
            || destination.starts_with(self.source_root.join("content"))
            || destination.starts_with(self.source_root.join("blog-compiler"))
            || destination.starts_with(self.source_root.join(".git"))
            || self
                .files
                .keys()
                .any(|path| self.source_root.join(path).starts_with(&destination))
        {
            return Err(Error(
                "output overlaps site sources or compiler files".into(),
            ));
        }
        if let Ok(metadata) = fs::symlink_metadata(&destination) {
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(Error("output must be a real directory".into()));
            }
            let nonempty = fs::read_dir(&destination)
                .map_err(io_error)?
                .next()
                .is_some();
            if nonempty && !destination.join(MARKER).is_file() {
                return Err(Error(
                    "refusing to replace a nonempty directory without the compiler output marker"
                        .into(),
                ));
            }
        }
        let stage = parent.join(format!(".dist-stage-{}", std::process::id()));
        let backup = parent.join(format!(".dist-backup-{}", std::process::id()));
        if backup.exists() {
            return Err(Error(format!(
                "recovery directory already exists: {}",
                backup.display()
            )));
        }
        fs::create_dir(&stage).map_err(io_error)?;
        let staged = (|| -> Result<()> {
            for (path, bytes) in &self.files {
                let target = stage.join(path);
                fs::create_dir_all(target.parent().unwrap()).map_err(io_error)?;
                fs::write(target, bytes).map_err(io_error)?;
            }
            fs::write(stage.join(MARKER), b"blog-compiler output v1\n").map_err(io_error)?;
            Ok(())
        })();
        if let Err(error) = staged {
            let _ = fs::remove_dir_all(&stage);
            return Err(error);
        }
        let had_output = destination.exists();
        if had_output && let Err(error) = fs::rename(&destination, &backup) {
            let _ = fs::remove_dir_all(&stage);
            return Err(io_error(error));
        }
        if let Err(error) = fs::rename(&stage, &destination) {
            if had_output {
                fs::rename(&backup, &destination).map_err(|recovery| Error(format!("publication failed: {error}; recovery failed: {recovery}; previous output: {}", backup.display())))?;
            }
            let _ = fs::remove_dir_all(&stage);
            return Err(io_error(error));
        }
        if had_output {
            fs::remove_dir_all(backup).map_err(io_error)?;
        }
        Ok(())
    }
}

/// Convert filesystem failures at the publication boundary into diagnostics.
fn io_error(error: std::io::Error) -> Error {
    Error(error.to_string())
}
