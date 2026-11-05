//! Migration regressions exercise the actual Markdown site and its static assets.

use std::{
    fs,
    path::{Path, PathBuf},
};

use blog_compiler::{Build, Site};
use scraper::{ElementRef, Html, Selector};

/// Repository source root; these tests do not depend on the shell working directory.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned()
}

/// Compile the real content with production configuration and validation enabled.
fn compile() -> Build {
    Site::load(root()).unwrap().compile().unwrap()
}

/// Read a generated HTML document using the same parser as the link validator.
fn page(build: &Build, route: &str) -> Html {
    Html::parse_document(std::str::from_utf8(&build.files()[&PathBuf::from(route)]).unwrap())
}

/// Retrieve elements by CSS selector without depending on serialization whitespace.
fn select<'a>(html: &'a Html, selector: &str) -> Vec<ElementRef<'a>> {
    html.select(&Selector::parse(selector).unwrap()).collect()
}

/// Normalize content structure while ignoring new heading anchors and HTML spelling.
fn normalized(element: ElementRef<'_>) -> String {
    let tag = match element.value().name() {
        "i" => "em",
        other => other,
    };
    let mut output = format!("<{tag}>");
    for child in element.children() {
        if let Some(text) = child.value().as_text() {
            if element.value().name() == "code" {
                output.push_str(text);
            } else {
                output.push_str(&text.split_whitespace().collect::<Vec<_>>().join(" "));
            }
        } else if let Some(child) = ElementRef::wrap(child) {
            output.push_str(&normalized(child));
        }
    }
    output.push_str(&format!("</{tag}>"));
    output
}

#[test]
fn production_routes_cards_publications_and_curated_previews_are_preserved() {
    let build = compile();
    assert_eq!(
        build
            .files()
            .keys()
            .filter(|path| path.extension().is_some_and(|value| value == "html"))
            .count(),
        8
    );
    let home = page(&build, "index.html");
    assert_eq!(select(&home, ".projects-grid .card").len(), 12);
    assert_eq!(select(&home, ".publications-list .pub-item").len(), 6);
    assert_eq!(select(&home, ".timeline-item").len(), 5);
    let sections: Vec<_> = select(&home, "main > section")
        .iter()
        .map(|node| node.value().attr("id").unwrap())
        .collect();
    assert_eq!(
        sections,
        [
            "hero",
            "experience",
            "projects",
            "research",
            "blog-preview",
            "about",
            "contact"
        ]
    );
    let previews: Vec<_> = select(&home, ".blog-entry-title a")
        .iter()
        .map(|node| node.value().attr("href").unwrap())
        .collect();
    assert_eq!(
        previews,
        [
            "blog/2026-03-12-fast-divide-and-conquer-delaunay.html",
            "blog/2024-11-05-idiomatic-rust-microui.html",
            "blog/2019-10-15-the-road-to-damascus-5m-bids.html"
        ]
    );
    let blog = page(&build, "blog.html");
    assert_eq!(select(&blog, ".blog-entry-card").len(), 4);
    assert_eq!(select(&blog, ".filter-btn").len(), 7);
    assert_eq!(select(&home, ".theme-toggle-btn").len(), 1);
    assert_eq!(
        select(&home, "#contact img")[0].value().attr("src"),
        Some("email.svg")
    );
}

#[test]
fn migrated_article_bodies_preserve_the_original_content_and_code() {
    let build = compile();
    for (path, bytes) in build.files() {
        if !path.to_string_lossy().starts_with("blog/20") {
            continue;
        }
        // Historical HTML remains a migration reference while user edits are retained.
        // Compilation itself never reads it; the independence test below proves that.
        let original = Html::parse_document(&fs::read_to_string(root().join(path)).unwrap());
        let generated = Html::parse_document(std::str::from_utf8(bytes).unwrap());
        assert_eq!(
            normalized(select(&original, ".post-content")[0]),
            normalized(select(&generated, ".post-content")[0]),
            "{}",
            path.display()
        );
    }
}

/// Copy only the declared Markdown/configuration/asset inputs, excluding old HTML.
fn copy_tree(source: &Path, destination: &Path) {
    if source.is_file() {
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(source, destination).unwrap();
    } else {
        fs::create_dir_all(destination).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            copy_tree(&entry.path(), &destination.join(entry.file_name()));
        }
    }
}

#[test]
fn compiler_needs_no_legacy_html_and_copies_assets_exactly() {
    let directory = tempfile::tempdir().unwrap();
    for path in [
        "site.toml",
        "content",
        "css",
        "js",
        "images",
        "docs",
        "email.svg",
    ] {
        copy_tree(&root().join(path), &directory.path().join(path));
    }
    let isolated = Site::load(directory.path()).unwrap().compile().unwrap();
    assert_eq!(compile().files(), isolated.files());
    for (path, bytes) in isolated.files() {
        if path.extension().is_none_or(|value| value != "html") {
            assert_eq!(
                bytes,
                &fs::read(root().join(path)).unwrap(),
                "{}",
                path.display()
            );
        }
    }
}
