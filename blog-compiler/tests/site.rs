//! Migration regressions exercise the actual Markdown site and its static assets.

use std::{
    fs,
    path::{Path, PathBuf},
};

use blog_compiler::{
    Build, Site, markdown,
    model::{BlockContent, CardKind, Directive, Metadata},
};
use pulldown_cmark::{Event, Parser, Tag};
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

/// Read the authored card list so editorial additions retain complete ordered output.
fn authored_project_titles() -> Vec<String> {
    let path = root().join("content/sections/projects.md");
    let section = markdown::parse(&path, &fs::read_to_string(&path).unwrap()).unwrap();
    let mut titles = Vec::new();
    for block in section.blocks {
        let BlockContent::Component {
            directive: Directive::Cards {
                kind: CardKind::Project,
            },
            body,
        } = block.content
        else {
            continue;
        };
        for block in body {
            let BlockContent::Markdown(text) = block.content else {
                panic!("project card references must remain Markdown lists");
            };
            for event in Parser::new_ext(&text, markdown::options()) {
                if let Event::Start(Tag::Link { dest_url, .. }) = event {
                    let source = path.parent().unwrap().join(dest_url.as_ref());
                    let record =
                        markdown::parse(&source, &fs::read_to_string(&source).unwrap()).unwrap();
                    let Metadata::Project(project) = record.metadata else {
                        panic!("project collections must reference project records");
                    };
                    titles.push(project.title);
                }
            }
        }
    }
    assert!(
        !titles.is_empty(),
        "the homepage must retain its authored project collection"
    );
    titles
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
    let project_titles: Vec<String> = select(&home, ".projects-grid .card-title")
        .iter()
        .map(|node| node.text().collect())
        .collect();
    assert_eq!(project_titles, authored_project_titles());
    assert_eq!(select(&home, ".publications-list .pub-item").len(), 6);
    // Editorial additions must render completely without freezing the milestone count.
    let about_path = root().join("content/sections/about.md");
    let about = markdown::parse(&about_path, &fs::read_to_string(&about_path).unwrap()).unwrap();
    let Metadata::Fragment(about) = about.metadata else {
        panic!("the About source must remain a fragment");
    };
    let titles: Vec<String> = select(&home, ".timeline-role")
        .iter()
        .map(|node| node.text().collect())
        .collect();
    let roles: Vec<String> = select(&home, ".timeline-meta")
        .iter()
        .map(|node| node.text().collect())
        .collect();
    assert_eq!(
        titles,
        about
            .milestones
            .iter()
            .map(|milestone| milestone.title.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        roles,
        about
            .milestones
            .iter()
            .map(|milestone| milestone.meta.as_str())
            .collect::<Vec<_>>()
    );
    let sections: Vec<_> = select(&home, "main > section")
        .iter()
        .map(|node| node.value().attr("id").unwrap())
        .collect();
    assert_eq!(
        sections,
        [
            "self",
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

#[test]
fn neocogi_projects_have_ordered_screenshots_without_blog_entries() {
    let build = compile();
    let home = page(&build, "index.html");
    let cards = select(&home, ".projects-grid > .card");
    let expected = ["neocogi-ui", "neocogi-platform", "neocogi-com"];
    for (card, project) in cards.iter().zip(expected) {
        let link = card
            .select(&Selector::parse(".card-title a").unwrap())
            .next()
            .unwrap();
        assert_eq!(link.text().collect::<String>(), project);
        assert_eq!(
            link.value().attr("href"),
            Some(format!("https://github.com/NeoCogi/{project}").as_str())
        );
        let screenshot = card
            .select(&Selector::parse(".project-screenshot img").unwrap())
            .next()
            .unwrap();
        assert_eq!(
            screenshot.value().attr("src"),
            Some(format!("images/{project}.png").as_str())
        );
        assert!(
            screenshot
                .value()
                .attr("alt")
                .is_some_and(|alt| !alt.trim().is_empty())
        );
        let full_size = card
            .select(&Selector::parse(".project-screenshot > a").unwrap())
            .next()
            .unwrap();
        assert_eq!(
            full_size.value().attr("href"),
            screenshot.value().attr("src")
        );
        assert!(
            card.select(&Selector::parse(".card-body").unwrap())
                .next()
                .unwrap()
                .text()
                .collect::<String>()
                .len()
                > 40
        );
    }
    assert_eq!(select(&home, ".project-screenshot").len(), expected.len());
    for route in ["blog.html", "blog/index.html"] {
        let blog = page(&build, route);
        let titles: Vec<String> = select(&blog, ".blog-entry-title")
            .iter()
            .map(|node| node.text().collect())
            .collect();
        assert!(
            expected
                .iter()
                .all(|project| !titles.iter().any(|title| title == project))
        );
    }
}
