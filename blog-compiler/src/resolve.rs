//! Resolve source ownership, output routes, asset mounts, and cross-file references.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use chrono::NaiveDate;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use pulldown_cmark::{Event, Tag};

use crate::{Build, Error, Result, markdown, model::*};

/// A loaded site owns all documents and assets. Compilation does not mutate it.
pub struct Site {
    /// Resolved source registry; renderers borrow it for one compilation.
    pub(crate) data: SiteData,
}

impl Site {
    /// Load an entire content tree, rejecting symlinks and conflicting routes.
    pub fn load(root: impl AsRef<Path>) -> Result<Self> {
        let root = canonical(root.as_ref())?;
        let config_path = root.join("site.toml");
        let config: Config = toml::from_str(&read(&config_path)?)
            .map_err(|error| Error::at(&config_path, 1, error))?;
        let origin =
            url::Url::parse(&config.url).map_err(|error| Error::at(&config_path, 1, error))?;
        if !matches!(origin.scheme(), "http" | "https") || origin.host_str().is_none() {
            return Err(Error::at(
                &config_path,
                1,
                "site URL must use http or https",
            ));
        }
        let content = canonical(&root.join("content"))?;
        let mut documents = BTreeMap::new();
        for path in files(&content)? {
            if path.extension().is_some_and(|value| value == "md") {
                let document = markdown::parse(&path, &read(&path)?)?;
                documents.insert(path, document);
            }
        }
        if documents.is_empty() {
            return Err(Error(
                "content directory contains no Markdown documents".into(),
            ));
        }
        if !matches!(documents.get(&content.join("index.md")).map(|document| &document.metadata), Some(Metadata::Page(page)) if page.route == "/index.html")
        {
            return Err(Error(
                "content/index.md must be a page with route /index.html".into(),
            ));
        }
        let mut routes = BTreeMap::new();
        let mut previews = BTreeSet::new();
        for document in documents.values() {
            let outputs = match &document.metadata {
                Metadata::Page(page) => {
                    let mut outputs = vec![page.route.clone()];
                    outputs.extend(page.aliases.clone());
                    outputs
                }
                Metadata::Post(post) => {
                    date(post).map_err(|error| Error::at(&document.path, 2, error))?;
                    if post.reading_minutes == Some(0) {
                        return Err(Error::at(
                            &document.path,
                            2,
                            "reading_minutes must be positive",
                        ));
                    }
                    if let Some(preview) = &post.home
                        && (preview.order == 0 || !previews.insert(preview.order))
                    {
                        return Err(Error::at(
                            &document.path,
                            2,
                            "homepage order must be positive and unique",
                        ));
                    }
                    for tag in &post.filter_tags {
                        identifier(tag).map_err(|error| Error::at(&document.path, 2, error))?;
                    }
                    vec![post_route(document, post)?]
                }
                Metadata::Fragment(fragment) => {
                    identifier(&fragment.id)
                        .map_err(|error| Error::at(&document.path, 2, error))?;
                    vec![]
                }
                Metadata::Project(project) => {
                    if project.image.is_some() == project.icon_text.is_some() {
                        return Err(Error::at(
                            &document.path,
                            2,
                            "project requires exactly one of image or icon_text",
                        ));
                    }
                    if project.image.is_some()
                        && project.image_alt.as_deref().is_none_or(str::is_empty)
                    {
                        return Err(Error::at(
                            &document.path,
                            2,
                            "project image requires image_alt",
                        ));
                    }
                    if let Some(screenshot) = &project.screenshot
                        && (screenshot.src.trim().is_empty() || screenshot.alt.trim().is_empty())
                    {
                        return Err(Error::at(
                            &document.path,
                            2,
                            "project screenshot requires nonempty src and alt",
                        ));
                    }
                    vec![]
                }
                Metadata::Publication(_) => vec![],
            };
            for route in outputs {
                validate_route(&route)?;
                if let Some(other) = routes.insert(route.clone(), document.path.clone()) {
                    return Err(Error::at(
                        &document.path,
                        2,
                        format!(
                            "duplicate route {route:?}, already owned by {}",
                            other.display()
                        ),
                    ));
                }
            }
        }
        let mut assets = BTreeMap::new();
        let mut asset_routes = BTreeSet::new();
        for mount in &config.assets {
            let relative = Path::new(mount);
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
            {
                return Err(Error::at(
                    &config_path,
                    1,
                    "asset mounts must be normal paths relative to the site root",
                ));
            }
            let source = root.join(relative);
            let entries = if source.is_dir() {
                files(&source)?
            } else {
                reject_symlink(&source)?;
                vec![canonical(&source)?]
            };
            for path in entries {
                let path = canonical(&path)?;
                if !path.starts_with(&root) || path.starts_with(&content) {
                    return Err(Error(format!(
                        "asset escapes its root or overlaps content: {}",
                        path.display()
                    )));
                }
                let route = format!(
                    "/{}",
                    path.strip_prefix(&root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/")
                );
                if routes.contains_key(&route) || !asset_routes.insert(route.clone()) {
                    return Err(Error(format!("duplicate asset/output route {route:?}")));
                }
                assets.insert(path, route);
            }
        }
        let mut topics = BTreeSet::new();
        for topic in &config.topics {
            identifier(&topic.id)?;
            if topic.id == "all" || !topics.insert(&topic.id) {
                return Err(Error(
                    "topic identifiers must be unique and cannot be 'all'".into(),
                ));
            }
        }
        Ok(Self {
            data: SiteData {
                root,
                config,
                documents,
                routes,
                assets,
            },
        })
    }

    /// Render and validate every route and asset without touching output files.
    pub fn compile(&self) -> Result<Build> {
        crate::render::validate_documents(&self.data)?;
        let mut files = BTreeMap::new();
        for (route, path) in &self.data.routes {
            let document = &self.data.documents[path];
            let html = crate::render::page(&self.data, document, route)?;
            files.insert(route.trim_start_matches('/').into(), html.into_bytes());
        }
        for (path, route) in &self.data.assets {
            files.insert(
                route.trim_start_matches('/').into(),
                fs::read(path).map_err(|error| Error(format!("{}: {error}", path.display())))?,
            );
        }
        let build = Build {
            files,
            source_root: self.data.root.clone(),
        };
        build.validate()?;
        Ok(build)
    }
}

/// Read UTF-8 source with a filesystem-oriented diagnostic.
pub(crate) fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|error| Error(format!("{}: {error}", path.display())))
}

/// Canonical paths are authoritative source identities and cannot be ambiguous.
fn canonical(path: &Path) -> Result<PathBuf> {
    fs::canonicalize(path).map_err(|error| Error(format!("{}: {error}", path.display())))
}

/// Source traversal deliberately excludes symlinks to keep ownership explicit.
fn reject_symlink(path: &Path) -> Result<()> {
    if fs::symlink_metadata(path)
        .map_err(|error| Error(format!("{}: {error}", path.display())))?
        .file_type()
        .is_symlink()
    {
        return Err(Error(format!(
            "symlinks are not supported in source trees: {}",
            path.display()
        )));
    }
    Ok(())
}

/// Walk deterministically without following symlinks or accepting special files.
fn files(path: &Path) -> Result<Vec<PathBuf>> {
    reject_symlink(path)?;
    let mut result = Vec::new();
    for entry in
        fs::read_dir(path).map_err(|error| Error(format!("{}: {error}", path.display())))?
    {
        let entry = entry.map_err(|error| Error(error.to_string()))?;
        let path = entry.path();
        reject_symlink(&path)?;
        let kind = entry
            .file_type()
            .map_err(|error| Error(error.to_string()))?;
        if kind.is_dir() {
            result.extend(files(&path)?);
        } else if kind.is_file() {
            result.push(canonical(&path)?);
        } else {
            return Err(Error(format!(
                "unsupported source file: {}",
                path.display()
            )));
        }
    }
    result.sort();
    Ok(result)
}

/// Validate identifiers used as fragment anchors and filter keys.
fn identifier(text: &str) -> Result<()> {
    if text.is_empty()
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(Error(format!(
            "expected a lowercase identifier, got {text:?}"
        )));
    }
    Ok(())
}

/// Routes are canonical output paths, never filesystem traversal expressions.
fn validate_route(route: &str) -> Result<()> {
    if !route.starts_with('/')
        || !route.ends_with(".html")
        || route.contains(['\\', '?', '#', '%', ':'])
        || route.chars().any(char::is_whitespace)
        || route[1..]
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(Error(format!(
            "invalid output route {route:?}; expected /path/name.html"
        )));
    }
    Ok(())
}

/// Convert a TOML date to a validated calendar value; datetime inputs are rejected.
pub(crate) fn date(post: &Post) -> Result<NaiveDate> {
    if post.date.time.is_some() || post.date.offset.is_some() {
        return Err(Error(
            "article date must be a calendar date without a time".into(),
        ));
    }
    NaiveDate::parse_from_str(&post.date.to_string(), "%Y-%m-%d")
        .map_err(|error| Error(format!("invalid article date: {error}")))
}

/// Derive stable article URLs from filenames unless an explicit route was authored.
pub(crate) fn post_route(document: &Document, post: &Post) -> Result<String> {
    if let Some(route) = &post.route {
        return Ok(route.clone());
    }
    let stem = document
        .path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| Error("article filename is not UTF-8".into()))?;
    Ok(format!("/blog/{stem}.html"))
}

/// Resolve a referenced source document relative to the originating document.
pub(crate) fn source<'a>(
    site: &'a SiteData,
    origin: &Document,
    reference: &str,
) -> Result<&'a Document> {
    let path = canonical(&origin.path.parent().unwrap().join(reference)).map_err(|error| {
        Error(format!(
            "{}: cannot resolve {reference:?}: {error}",
            origin.path.display()
        ))
    })?;
    site.documents.get(&path).ok_or_else(|| {
        Error(format!(
            "{}: referenced Markdown is outside the content tree: {reference:?}",
            origin.path.display()
        ))
    })
}

/// Encode output path segments without changing separators or double-encoding URLs.
fn encode_path(path: &str) -> String {
    /// RFC 3986 unreserved characters are retained within each path segment.
    const ENCODE: &percent_encoding::AsciiSet = &NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'_')
        .remove(b'.')
        .remove(b'~');
    path.split('/')
        .map(|part| utf8_percent_encode(part, ENCODE).to_string())
        .collect::<Vec<_>>()
        .join("/")
}

/// Generate a relative URL between two canonical published paths.
pub(crate) fn relative(current: &str, destination: &str) -> String {
    let current: Vec<_> = current.trim_start_matches('/').split('/').collect();
    let target: Vec<_> = destination.trim_start_matches('/').split('/').collect();
    let directory = &current[..current.len() - 1];
    let common = directory
        .iter()
        .zip(&target)
        .take_while(|(a, b)| a == b)
        .count();
    let path = format!(
        "{}{}",
        "../".repeat(directory.len() - common),
        target[common..].join("/")
    );
    encode_path(&path)
}

/// Resolve authored URLs to emitted routes while keeping asset and source ownership.
pub(crate) fn link(
    site: &SiteData,
    origin: &Document,
    current: &str,
    reference: &str,
) -> Result<String> {
    if reference.starts_with('#') {
        return Ok(reference.to_owned());
    }
    if let Ok(url) = url::Url::parse(reference) {
        if matches!(url.scheme(), "http" | "https" | "mailto" | "tel") {
            return Ok(reference.to_owned());
        }
        return Err(Error(format!(
            "{}: unsupported URL scheme in {reference:?}",
            origin.path.display()
        )));
    }
    if reference.starts_with("//") {
        return Err(Error(format!(
            "protocol-relative URLs are unsupported: {reference:?}"
        )));
    }
    let cut = reference.find(['?', '#']).unwrap_or(reference.len());
    let (path, suffix) = reference.split_at(cut);
    if path.is_empty() {
        return Ok(format!("{}{suffix}", current.rsplit('/').next().unwrap()));
    }
    let destination = if path.starts_with('/') {
        path.to_owned()
    } else {
        let decoded = percent_encoding::percent_decode_str(path)
            .decode_utf8()
            .map_err(|error| Error(error.to_string()))?;
        let resolved = canonical(&origin.path.parent().unwrap().join(decoded.as_ref()))?;
        if let Some(document) = site.documents.get(&resolved) {
            match &document.metadata {
                Metadata::Page(page) => page.route.clone(),
                Metadata::Post(post) => post_route(document, post)?,
                _ => {
                    return Err(Error(format!(
                        "{}: fragment/record {reference:?} has no output page",
                        origin.path.display()
                    )));
                }
            }
        } else {
            site.assets.get(&resolved).cloned().ok_or_else(|| {
                Error(format!(
                    "{}: {reference:?} is not a mounted asset or page",
                    origin.path.display()
                ))
            })?
        }
    };
    Ok(format!("{}{suffix}", relative(current, &destination)))
}

/// Parse a collection body as a Markdown list of links, rejecting unrelated content.
pub(crate) fn card_sources<'a>(
    site: &'a SiteData,
    document: &Document,
    blocks: &[Block],
    kind: CardKind,
) -> Result<Vec<&'a Document>> {
    let mut result = Vec::new();
    for block in blocks {
        let BlockContent::Markdown(text) = &block.content else {
            return Err(Error::at(
                &document.path,
                block.line,
                "card bodies must be Markdown lists of links",
            ));
        };
        let mut list_depth = 0;
        let mut in_link = false;
        for event in pulldown_cmark::Parser::new_ext(text, markdown::options()) {
            match event {
                Event::Start(Tag::List(_)) => list_depth += 1,
                Event::End(pulldown_cmark::TagEnd::List(_)) => list_depth -= 1,
                Event::Start(Tag::Link { dest_url, .. }) if list_depth > 0 => {
                    let target = source(site, document, &dest_url)?;
                    if !matches!(
                        (kind, &target.metadata),
                        (CardKind::Project, Metadata::Project(_))
                            | (CardKind::Publication, Metadata::Publication(_))
                    ) {
                        return Err(Error::at(
                            &document.path,
                            block.line,
                            "card reference has the wrong document kind",
                        ));
                    }
                    result.push(target);
                    in_link = true;
                }
                Event::End(pulldown_cmark::TagEnd::Link) => in_link = false,
                Event::Text(text) if in_link || text.trim().is_empty() => {}
                Event::Start(Tag::Item | Tag::Paragraph)
                | Event::End(pulldown_cmark::TagEnd::Item | pulldown_cmark::TagEnd::Paragraph)
                | Event::SoftBreak
                | Event::HardBreak => {}
                _ => {
                    return Err(Error::at(
                        &document.path,
                        block.line,
                        "card bodies accept only a list of Markdown links",
                    ));
                }
            }
        }
    }
    if result.is_empty() {
        return Err(Error::at(&document.path, 1, "card collection is empty"));
    }
    Ok(result)
}
