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

/// Published article features that must survive compilation without HTML fixtures.
struct ArticleContract {
    /// Generated route identifying the Markdown article being checked.
    route: &'static str,
    /// Ordered H2 sections, verifying that the full article body is present.
    headings: &'static [&'static str],
    /// One verbatim marker per code block, preserving characters and line breaks.
    code_markers: &'static [&'static str],
    /// Resolved figure URLs in article order.
    figures: &'static [&'static str],
    /// Number of semantic callout containers.
    callouts: usize,
    /// Number of quotations, including the Pacer disclaimer.
    quotations: usize,
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
fn articles_render_complete_sections_verbatim_code_figures_and_callouts() {
    let build = compile();
    let contracts = [
        ArticleContract {
            route: "blog/2019-10-15-the-road-to-damascus-5m-bids.html",
            headings: &[
                "1. The Problem Space: Digital Advertising Exchanges",
                "2. The Pacer: Governing Campaign Budgets",
                "3. Actor Architecture with MailboxProcessors",
                "4. The Bottleneck: Profiling CoreFX Sockets",
                "5. The Breakthrough: Marrying F# with libuv in C11",
                "6. Lessons Learned",
                "References",
            ],
            code_markers: &["$ sudo ./perfcollect collect session -pid <pid>\n$ sudo perf script"],
            figures: &[
                "../images/actor-pacer.png",
                "../images/flamegraph-no-optim.png",
                "../images/flamegraph-optim.png",
            ],
            callouts: 2,
            quotations: 1,
        },
        ArticleContract {
            route: "blog/2023-08-20-tiny-actor-model-in-c11.html",
            headings: &[
                "1. The Cooperative Invariant",
                "2. Bounded Queues & Backpressure",
                "3. Practical Applications",
                "Resources",
            ],
            code_markers: &["tcpm_proc_t* target = &sys->procs[target_pid];\n"],
            figures: &[],
            callouts: 1,
            quotations: 0,
        },
        ArticleContract {
            route: "blog/2024-11-05-idiomatic-rust-microui.html",
            headings: &[
                "1. The Problem with Direct C Translations",
                "2. Command Buffer Arenas & Token-Based Contexts",
                "3. Ergonomic Rust API",
                "Conclusion",
            ],
            code_markers: &[
                "MouseButton::Left \n        && self.focus == id;",
                "#[repr(C)]\n#[derive(Clone, Copy)]\npub enum Command",
                "ctx.window(\"Diagnostics\", Rect::new(10, 10, 260, 200), |ctx| {",
            ],
            figures: &["../images/microui.png"],
            callouts: 1,
            quotations: 0,
        },
        ArticleContract {
            route: "blog/2026-03-12-fast-divide-and-conquer-delaunay.html",
            headings: &[
                "1. The Divide-and-Conquer Strategy",
                "2. Robust Geometric Predicates",
                "3. Compact Quad-Edge Topology in C99",
                "4. Empirical Performance",
                "Resources & Code",
            ],
            code_markers: &[
                "return (acx * bcy) - (acy * bcx);\n}",
                "uint32_t next;   // Index to next CCW edge\n",
            ],
            figures: &[],
            callouts: 1,
            quotations: 1,
        },
    ];
    for contract in contracts {
        let generated = page(&build, contract.route);
        let headings: Vec<String> = select(&generated, ".post-content h2")
            .iter()
            .map(|node| node.text().collect())
            .collect();
        assert_eq!(headings, contract.headings, "{}", contract.route);
        let code = select(&generated, ".post-content pre code");
        assert_eq!(
            code.len(),
            contract.code_markers.len(),
            "{}",
            contract.route
        );
        for (block, marker) in code.iter().zip(contract.code_markers) {
            let text: String = block.text().collect();
            assert!(
                text.contains(marker),
                "{}: missing code marker {marker:?}",
                contract.route
            );
        }
        let figures = select(&generated, ".post-content figure img");
        let sources: Vec<_> = figures
            .iter()
            .map(|node| node.value().attr("src").unwrap())
            .collect();
        assert_eq!(sources, contract.figures, "{}", contract.route);
        assert!(
            figures
                .iter()
                .all(|node| node.value().attr("alt").is_some_and(|alt| !alt.is_empty()))
        );
        assert_eq!(
            select(&generated, ".post-content figcaption").len(),
            figures.len()
        );
        assert_eq!(
            select(&generated, ".post-content .post-callout").len(),
            contract.callouts,
            "{}",
            contract.route
        );
        assert_eq!(
            select(&generated, ".post-content blockquote").len(),
            contract.quotations,
            "{}",
            contract.route
        );
    }
}

/// Copy only declared Markdown, configuration, and asset inputs into an isolated root.
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
fn declared_source_inputs_are_sufficient_and_assets_are_copied_exactly() {
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
