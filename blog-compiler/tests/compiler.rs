//! Contract regressions exercise the public compiler, filesystem publication, and CLI.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use blog_compiler::{
    Build, Site, markdown,
    model::{BlockContent, Directive},
};
use scraper::{Html, Selector};
use tempfile::TempDir;

/// An isolated, minimal site containing all layout dependencies and no real content.
struct Fixture {
    /// Owns and removes source and output files after each test.
    directory: TempDir,
}

impl Fixture {
    /// Create a valid two-page site with one alias and mounted CSS/JavaScript.
    fn new() -> Self {
        let value = Self {
            directory: tempfile::tempdir().unwrap(),
        };
        value.write(
            "site.toml",
            r#"
url = "https://example.test"
author = "Author & Co"
badge = "AC"
copyright_year = 2026
footer_note = "A test site"
assets = ["css", "js"]
navigation = []
socials = []
blog_socials = []
topics = []
"#,
        );
        value.write("css/style.css", "body { color: black; }");
        value.write("js/main.js", "'use strict';");
        value.home("# Home\n\nHello.");
        value.write("content/blog.md", "+++\nkind = \"page\"\nlayout = \"blog\"\nroute = \"/blog.html\"\naliases = [\"/blog/index.html\"]\ntitle = \"Blog\"\ndescription = \"Posts\"\n+++\n\n# Blog\n\nPosts.\n\n:::posts view=\"list\"\n:::\n");
        value
    }

    /// Fixture root, independent of the repository's working directory.
    fn root(&self) -> &Path {
        self.directory.path()
    }

    /// Write a source or asset, creating its containing directories.
    fn write(&self, path: &str, text: &str) {
        let path = self.root().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// Replace homepage content while retaining a valid standalone contract.
    fn home(&self, body: &str) {
        self.write("content/index.md", &format!("+++\nkind = \"page\"\nlayout = \"home\"\nroute = \"/index.html\"\ntitle = \"Home\"\ndescription = \"Description\"\n+++\n\n{body}\n\n## Contact\n"));
    }

    /// Add a reusable Markdown fragment at an arbitrary content-relative path.
    fn fragment(&self, path: &str, body: &str) {
        self.write(
            &format!("content/{path}"),
            &format!("+++\nkind = \"fragment\"\nid = \"fragment\"\n+++\n\n{body}\n"),
        );
    }

    /// Add a dated article with optional extra metadata and arbitrary Markdown.
    fn post(&self, name: &str, extra: &str, body: &str) {
        self.write(&format!("content/posts/{name}.md"), &format!("+++\nkind = \"post\"\ntitle = \"Article <&>\"\ndescription = \"Summary\"\ndate = 2026-01-02\nexcerpt = \"Listing **summary**\"\ntags = [\"Rust\"]\nfilter_tags = [\"rust\"]\n{extra}\n+++\n\n{body}\n"));
    }

    /// Load and compile through the same public boundary used by the CLI.
    fn compile(&self) -> blog_compiler::Result<Build> {
        Site::load(self.root())?.compile()
    }

    /// Get an actionable diagnostic without requiring Debug on a successful build.
    fn error(&self) -> String {
        match self.compile() {
            Ok(_) => panic!("expected failure"),
            Err(error) => error.to_string(),
        }
    }
}

/// Read one generated HTML document as UTF-8.
fn output(build: &Build, path: &str) -> String {
    String::from_utf8(build.files()[&PathBuf::from(path)].clone()).unwrap()
}

/// Extract text using the actual HTML parser rather than testing serialization trivia.
fn texts(html: &str, selector: &str) -> Vec<String> {
    Html::parse_document(html)
        .select(&Selector::parse(selector).unwrap())
        .map(|node| node.text().collect())
        .collect()
}

#[test]
fn builds_aliases_with_rebased_links_and_escaped_metadata() {
    let fixture = Fixture::new();
    fixture.post("article", "", "## Section\n\nText.");
    let build = fixture.compile().unwrap();
    let root = output(&build, "blog.html");
    let alias = output(&build, "blog/index.html");
    assert!(root.contains("href=\"blog/article.html\""));
    assert!(alias.contains("href=\"article.html\""));
    assert!(alias.contains("href=\"../css/style.css\""));
    assert_eq!(texts(&root, ".blog-entry-title"), vec!["Article <&>"]);
    assert!(output(&build, "blog/article.html").contains("Article &lt;&amp;&gt;"));
    assert_eq!(texts(&alias, ".blog-entry-excerpt strong"), vec!["summary"]);
}

#[test]
fn includes_keep_source_relative_links_assets_and_reference_definitions() {
    let fixture = Fixture::new();
    fixture.post("article", "", "## Target\n\nBody.");
    fixture.write("css/my diagram.svg", "<svg/>");
    fixture.fragment("sections/intro.md", "A [post](../posts/article.md#target).\n\n:::callout\n[Referenced][article]\n:::\n\n[article]: ../posts/article.md\n\n:::figure src=\"../../css/my diagram.svg\" alt=\"A diagram\"\nCaption **text**.\n:::");
    fixture.home("# Home\n\n:::include src=\"sections/intro.md\" view=\"markdown\"\n:::");
    let html = output(&fixture.compile().unwrap(), "index.html");
    assert!(html.contains("href=\"blog/article.html#target\""));
    assert!(html.contains("src=\"css/my%20diagram.svg\""));
    assert_eq!(texts(&html, ".post-callout a"), vec!["Referenced"]);
    assert_eq!(texts(&html, "figcaption strong"), vec!["text"]);
}

#[test]
fn code_examples_are_never_executed_or_expanded_as_directives() {
    let fixture = Fixture::new();
    fixture.post("code", "", "```markdown\n:::include src=\"missing.md\" view=\"markdown\"\n:::\n<script>alert('example')</script>\n```\n\n    :::unknown\n    :::\n\n`<&>`");
    let html = output(&fixture.compile().unwrap(), "blog/code.html");
    assert_eq!(texts(&html, "pre").len(), 2);
    assert!(html.contains("&lt;script&gt;"));
    assert!(!html.contains("<script>alert"));
    assert!(html.contains(":::include"));
}

#[test]
fn malformed_frontmatter_and_unknown_fields_fail() {
    let fixture = Fixture::new();
    fixture.write("content/bad.md", "# Missing front matter");
    assert!(fixture.error().contains("bad.md:1:"));
    fixture.write(
        "content/bad.md",
        "+++\nkind = \"fragment\"\nid = \"x\"\ntypo = true\n+++\n",
    );
    assert!(fixture.error().contains("unknown field"));
    fixture.write("content/bad.md", "+++\nkind = \"fragment\"\nid = \"x\"");
    assert!(fixture.error().contains("unclosed TOML"));
    fixture.write("content/bad.md", "+++\nkind = \"mystery\"\n+++\n");
    assert!(fixture.error().contains("unknown variant"));
}

#[test]
fn malformed_directives_report_source_locations() {
    let fixture = Fixture::new();
    for (body, expected) in [
        (":::mystery\n:::", "unknown directive"),
        (":::callout typo=\"yes\"\n:::", "accepts no attributes"),
        (
            ":::include src=\"x.md\" view=\"markdown\" typo=true\n:::",
            "unknown field",
        ),
        (":::callout\nUnclosed", "unclosed directive"),
        (":::", "unexpected directive"),
        (
            ":::include src=\"x.md\" view=\"markdown\"\nUnexpected body\n:::",
            "empty body",
        ),
        (":::figure src=\"broken\n:::", "invalid"),
    ] {
        fixture.home(&format!("# Home\n\n{body}"));
        let error = fixture.error();
        assert!(error.contains("index.md:"), "{error}");
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn missing_and_wrong_kind_references_fail() {
    let fixture = Fixture::new();
    fixture.home("# Home\n\n:::include src=\"missing.md\" view=\"markdown\"\n:::");
    assert!(fixture.error().contains("missing.md"));
    fixture.home("# Home\n\n:::include src=\"blog.md\" view=\"markdown\"\n:::");
    assert!(fixture.error().contains("requires kind"));
    fixture.home("# Home\n\n:::cards kind=\"project\"\n- [Blog](blog.md)\n:::");
    assert!(fixture.error().contains("wrong document kind"));
    fixture.home("# Home\n\n:::cards kind=\"project\"\nProse instead of links.\n:::");
    assert!(fixture.error().contains("list of Markdown links"));
}

#[test]
fn include_cycles_fail_even_when_fragments_are_not_reachable_from_pages() {
    let fixture = Fixture::new();
    fixture.fragment("a.md", ":::include src=\"b.md\" view=\"markdown\"\n:::");
    fixture.fragment("b.md", ":::include src=\"a.md\" view=\"markdown\"\n:::");
    assert!(fixture.error().contains("include cycle"));
}

#[test]
fn duplicate_and_traversing_routes_fail() {
    let fixture = Fixture::new();
    fixture.post("first", "route = \"/index.html\"", "Body.");
    assert!(fixture.error().contains("duplicate route"));
    for route in [
        "../escape.html",
        "/../escape.html",
        "/a//b.html",
        "/a/./b.html",
        "/page.html?x",
        "/page%20name.html",
    ] {
        fixture.post("first", &format!("route = \"{route}\""), "Body.");
        assert!(fixture.error().contains("invalid output route"));
    }
}

#[test]
fn local_assets_anchors_and_unpublished_fragment_links_are_checked() {
    let fixture = Fixture::new();
    fixture.home("# Home\n\n[Missing](/absent.html)");
    assert!(fixture.error().contains("missing local target"));
    fixture.home("# Home\n\n[Missing](#absent)");
    assert!(fixture.error().contains("missing anchor"));
    fixture.home("# Home\n\n![Missing](../css/absent.png)");
    assert!(fixture.error().contains("absent.png"));
    fixture.fragment("fragment.md", "Fragment.");
    fixture.home("# Home\n\n[Unpublished](fragment.md)");
    assert!(fixture.error().contains("has no output page"));
}

#[test]
fn unsafe_schemes_and_raw_html_fail_but_code_is_safe() {
    let fixture = Fixture::new();
    for body in [
        "[Bad](javascript:alert)",
        "![Bad](data:image/png;base64,AA)",
        "[Bad](file:///tmp/a)",
        "[Bad](//example.test)",
        "<div>Raw</div>",
    ] {
        fixture.home(&format!("# Home\n\n{body}"));
        let error = fixture.error();
        assert!(
            error.contains("URL") || error.contains("raw HTML"),
            "{error}"
        );
    }
}

#[test]
fn heading_ids_are_stable_unique_and_addressable() {
    let fixture = Fixture::new();
    fixture.home(
        "# Home\n\n## Repeated title\n\nOne.\n\n## Repeated title\n\n[Second](#repeated-title-2)",
    );
    let html = output(&fixture.compile().unwrap(), "index.html");
    assert!(html.contains("id=\"repeated-title\""));
    assert!(html.contains("id=\"repeated-title-2\""));
}

#[test]
fn curated_previews_and_listing_have_independent_order_and_content() {
    let fixture = Fixture::new();
    fixture.post(
        "a",
        "[home]\norder = 2\nexcerpt = \"Preview A\"\ntags = [\"A\"]",
        "Body.",
    );
    fixture.post(
        "b",
        "[home]\norder = 1\nexcerpt = \"Preview B\"\ntags = [\"B\"]",
        "Body.",
    );
    fixture.post("c", "", "Body.");
    fixture.home("# Home\n\n:::posts view=\"preview\"\n## Featured\n\nCurated.\n:::");
    let build = fixture.compile().unwrap();
    assert_eq!(
        texts(&output(&build, "index.html"), ".blog-entry-excerpt"),
        vec!["Preview B", "Preview A"]
    );
    assert_eq!(
        texts(&output(&build, "blog.html"), ".blog-entry-title").len(),
        3
    );
    fixture.post(
        "c",
        "[home]\norder = 1\nexcerpt = \"C\"\ntags = []",
        "Body.",
    );
    assert!(fixture.error().contains("positive and unique"));
}

#[test]
fn calendar_dates_and_reading_time_are_validated() {
    let fixture = Fixture::new();
    fixture.post("a", "reading_minutes = 0", "Body.");
    assert!(fixture.error().contains("must be positive"));
    fixture.post("a", "", "Body.");
    let path = fixture.root().join("content/posts/a.md");
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("2026-01-02", "2026-02-30");
    fs::write(&path, text).unwrap();
    assert!(fixture.error().contains("date"));
    fixture.post("a", "", "Body.");
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("2026-01-02", "2026-01-02T12:00:00Z");
    fs::write(&path, text).unwrap();
    assert!(fixture.error().contains("without a time"));
}

#[test]
fn reading_time_falls_back_to_document_text() {
    let fixture = Fixture::new();
    fixture.post("a", "", &"word ".repeat(201));
    let html = output(&fixture.compile().unwrap(), "blog/a.html");
    assert_eq!(texts(&html, ".reading-time"), vec!["• 2 min read"]);
}

#[test]
fn builds_are_deterministic_and_assets_are_byte_identical() {
    let fixture = Fixture::new();
    let first = fixture.compile().unwrap();
    let second = fixture.compile().unwrap();
    assert_eq!(first.files(), second.files());
    assert_eq!(
        first.files()[&PathBuf::from("css/style.css")],
        fs::read(fixture.root().join("css/style.css")).unwrap()
    );
}

#[test]
fn publication_replaces_complete_output_and_removes_stale_files() {
    let fixture = Fixture::new();
    let path = fixture.root().join("dist");
    fixture.post("removed", "", "Body.");
    fixture.compile().unwrap().write(&path).unwrap();
    assert!(path.join("blog/removed.html").is_file());
    fs::remove_file(fixture.root().join("content/posts/removed.md")).unwrap();
    fixture.compile().unwrap().write(&path).unwrap();
    assert!(!path.join("blog/removed.html").exists());
}

#[test]
fn output_safety_preserves_sources_and_unrelated_directories() {
    let fixture = Fixture::new();
    let build = fixture.compile().unwrap();
    for directory in [
        fixture.root().to_owned(),
        fixture.root().join("content"),
        fixture.root().join("css"),
    ] {
        assert!(build.write(directory).is_err());
    }
    fixture.write("unrelated/keep.txt", "Keep me");
    assert!(build.write(fixture.root().join("unrelated")).is_err());
    assert_eq!(
        fs::read_to_string(fixture.root().join("unrelated/keep.txt")).unwrap(),
        "Keep me"
    );
}

#[test]
fn cli_failure_leaves_previous_output_untouched_and_check_writes_nothing() {
    let fixture = Fixture::new();
    let output_dir = fixture.root().join("dist");
    let binary = env!("CARGO_BIN_EXE_blog-compiler");
    let checked = Command::new(binary)
        .args(["check", "--root"])
        .arg(fixture.root())
        .output()
        .unwrap();
    assert!(checked.status.success());
    assert!(!output_dir.exists());
    let built = Command::new(binary)
        .args(["build", "--root"])
        .arg(fixture.root())
        .output()
        .unwrap();
    assert!(built.status.success());
    let before = fs::read(output_dir.join("index.html")).unwrap();
    fixture.home("# Home\n\n[Broken](/missing.html)");
    let failed = Command::new(binary)
        .args(["build", "--root"])
        .arg(fixture.root())
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert_eq!(before, fs::read(output_dir.join("index.html")).unwrap());
    let invalid = Command::new(binary)
        .args(["check", "--out", "ignored"])
        .output()
        .unwrap();
    assert!(!invalid.status.success());
}

#[cfg(unix)]
#[test]
fn symlink_sources_and_output_are_rejected() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let build = fixture.compile().unwrap();
    symlink(
        fixture.root().join("css/style.css"),
        fixture.root().join("content/escape.md"),
    )
    .unwrap();
    assert!(fixture.error().contains("symlinks are not supported"));
    symlink(fixture.root().join("css"), fixture.root().join("dist")).unwrap();
    assert!(build.write(fixture.root().join("dist")).is_err());
}

#[test]
fn parser_preserves_nested_component_types_and_crlf_locations() {
    let parsed = markdown::parse(
        Path::new("example.md"),
        "+++\r\nkind = \"fragment\"\r\nid = \"x\"\r\n+++\r\n\r\n:::callout\r\nHello.\r\n:::\r\n",
    )
    .unwrap();
    assert_eq!(parsed.blocks[0].line, 6);
    assert!(matches!(
        parsed.blocks[0].content,
        BlockContent::Component {
            directive: Directive::Callout,
            ..
        }
    ));
}

#[test]
fn missing_homepage_and_excessive_nesting_are_diagnostics() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.root().join("content/index.md")).unwrap();
    assert!(fixture.error().contains("content/index.md"));
    fixture.home(&format!(
        "# Home\n\n{}Text\n{}",
        ":::callout\n".repeat(65),
        ":::\n".repeat(65)
    ));
    assert!(fixture.error().contains("nesting exceeds"));
}

#[test]
fn card_reference_cycles_fail_without_recursing_indefinitely() {
    let fixture = Fixture::new();
    fixture.write("content/self.md", "+++\nkind = \"project\"\ntitle = \"Self\"\nicon_text = \"S\"\ntags = []\n[action]\nlabel = \"View\"\nhref = \"https://example.test\"\n+++\n\n:::cards kind=\"project\"\n- [Self](self.md)\n:::\n");
    assert!(fixture.error().contains("card reference cycle"));
}

#[test]
fn timelines_follow_authored_additions_removals_and_order() {
    let fixture = Fixture::new();
    fixture.home("# Home\n\n:::include src=\"timeline.md\" view=\"markdown\"\n:::");
    for titles in [
        &[][..],
        &["First & Founder"][..],
        &["First & Founder", "Second", "Third"][..],
        &["Third", "First & Founder"][..],
    ] {
        let mut source = String::from("+++\nkind = \"fragment\"\nid = \"timeline\"\n");
        for title in titles {
            source.push_str(&format!("[[milestones]]\ntitle = \"{title}\"\nmeta = \"A role\"\ndescription = \"Work with **emphasis**.\"\n"));
        }
        source.push_str("+++\n\n:::timeline\n:::\n");
        fixture.write("content/timeline.md", &source);
        let html = output(&fixture.compile().unwrap(), "index.html");
        assert_eq!(texts(&html, ".timeline-role"), titles);
        assert_eq!(texts(&html, ".timeline-meta"), vec!["A role"; titles.len()]);
        assert_eq!(
            texts(&html, ".timeline-desc strong"),
            vec!["emphasis"; titles.len()]
        );
    }
}
