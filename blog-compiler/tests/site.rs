//! Production regressions compare the current authored site with generated output.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use blog_compiler::{
    Build, Site, markdown,
    model::{BlockContent, CardKind, Config, Directive, Document, Metadata, Post},
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
    let path = PathBuf::from(route.trim_start_matches('/'));
    let bytes = build
        .files()
        .get(&path)
        .unwrap_or_else(|| panic!("missing generated page {route}"));
    Html::parse_document(std::str::from_utf8(bytes).unwrap())
}

/// Retrieve elements by CSS selector without depending on serialization whitespace.
fn select<'a>(html: &'a Html, selector: &str) -> Vec<ElementRef<'a>> {
    html.select(&Selector::parse(selector).unwrap()).collect()
}

/// Read the authored card list so editorial additions retain complete ordered output.
fn authored_card_titles(section: &str, kind: CardKind) -> Vec<String> {
    let path = root().join("content/sections").join(section);
    let section = markdown::parse(&path, &fs::read_to_string(&path).unwrap()).unwrap();
    let mut titles = Vec::new();
    for block in section.blocks {
        let BlockContent::Component {
            directive: Directive::Cards { kind: actual },
            body,
        } = block.content
        else {
            continue;
        };
        assert!(matches!(
            (kind, actual),
            (CardKind::Project, CardKind::Project) | (CardKind::Publication, CardKind::Publication)
        ));
        for block in body {
            let BlockContent::Markdown(text) = block.content else {
                panic!("card references must remain Markdown lists");
            };
            for event in Parser::new_ext(&text, markdown::options()) {
                if let Event::Start(Tag::Link { dest_url, .. }) = event {
                    let source = path.parent().unwrap().join(dest_url.as_ref());
                    let record =
                        markdown::parse(&source, &fs::read_to_string(&source).unwrap()).unwrap();
                    let title = match (kind, record.metadata) {
                        (CardKind::Project, Metadata::Project(project)) => project.title,
                        (CardKind::Publication, Metadata::Publication(publication)) => {
                            Parser::new_ext(&publication.title, markdown::options())
                                .filter_map(|event| match event {
                                    Event::Text(text) | Event::Code(text) => {
                                        Some(text.into_string())
                                    }
                                    _ => None,
                                })
                                .collect()
                        }
                        _ => panic!("card collections must reference their declared record kind"),
                    };
                    titles.push(title);
                }
            }
        }
    }
    titles
}

/// Read every current Markdown record, including nested folders and non-post pages.
fn authored_documents(directory: &Path) -> Vec<Document> {
    let mut documents = Vec::new();
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            documents.extend(authored_documents(&path));
        } else if path.extension().is_some_and(|extension| extension == "md") {
            documents.push(markdown::parse(&path, &fs::read_to_string(&path).unwrap()).unwrap());
        }
    }
    documents
}

/// Expected public URL: an explicit route wins over the source filename default.
fn article_route(source: &Path, post: &Post) -> String {
    post.route.clone().unwrap_or_else(|| {
        format!(
            "/blog/{}.html",
            source.file_stem().unwrap().to_str().unwrap()
        )
    })
}

/// Current posts in listing order, with source paths breaking equal-date ties.
fn authored_posts() -> Vec<(PathBuf, Post)> {
    let mut posts: Vec<_> = authored_documents(&root().join("content"))
        .into_iter()
        .filter_map(|document| match document.metadata {
            Metadata::Post(post) => Some((document.path, post)),
            _ => None,
        })
        .collect();
    posts.sort_by(|(a, pa), (b, pb)| {
        pb.date
            .to_string()
            .cmp(&pa.date.to_string())
            .then_with(|| a.cmp(b))
    });
    posts
}

/// Resolve card destinations against their page so index aliases compare equally.
fn post_links(html: &Html, route: &str) -> Vec<(String, String)> {
    let base = url::Url::parse("https://example.test")
        .unwrap()
        .join(route)
        .unwrap();
    select(html, ".blog-entry-title a")
        .iter()
        .map(|node| {
            let target = base.join(node.value().attr("href").unwrap()).unwrap();
            let path = percent_encoding::percent_decode_str(target.path())
                .decode_utf8()
                .unwrap()
                .into_owned();
            (path, node.text().collect())
        })
        .collect()
}

#[test]
fn production_routes_cards_publications_and_curated_previews_are_preserved() {
    let build = compile();
    let expected_routes: BTreeSet<_> = authored_documents(&root().join("content"))
        .into_iter()
        .flat_map(|document| match document.metadata {
            Metadata::Page(page) => std::iter::once(page.route).chain(page.aliases).collect(),
            Metadata::Post(post) => vec![article_route(&document.path, &post)],
            _ => Vec::new(),
        })
        .map(|route| PathBuf::from(route.trim_start_matches('/')))
        .collect();
    let actual_routes = build
        .files()
        .keys()
        .filter(|path| path.extension().is_some_and(|value| value == "html"))
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(actual_routes, expected_routes);
    let home = page(&build, "index.html");
    let project_titles: Vec<String> = select(&home, ".projects-grid .card-title")
        .iter()
        .map(|node| node.text().collect())
        .collect();
    assert_eq!(
        project_titles,
        authored_card_titles("projects.md", CardKind::Project)
    );
    let publication_titles: Vec<String> = select(&home, ".publications-list .pub-title")
        .iter()
        .map(|node| node.text().collect())
        .collect();
    assert_eq!(
        publication_titles,
        authored_card_titles("research.md", CardKind::Publication)
    );
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
    let posts = authored_posts();
    let mut previews: Vec<_> = posts
        .iter()
        .filter(|(_, post)| post.home.is_some())
        .collect();
    previews.sort_by_key(|(_, post)| post.home.as_ref().unwrap().order);
    let expected_previews: Vec<_> = previews
        .iter()
        .map(|(source, post)| (article_route(source, post), post.title.clone()))
        .collect();
    assert_eq!(post_links(&home, "/index.html"), expected_previews);
    let expected_listing: Vec<_> = posts
        .iter()
        .map(|(source, post)| (article_route(source, post), post.title.clone()))
        .collect();
    for route in ["/blog.html", "/blog/index.html"] {
        let blog = page(&build, route);
        assert_eq!(post_links(&blog, route), expected_listing, "{route}");
        let filters: Vec<_> = select(&blog, ".blog-entry-card")
            .iter()
            .map(|node| node.value().attr("data-tags").unwrap())
            .collect();
        assert_eq!(
            filters,
            posts
                .iter()
                .map(|(_, post)| post.filter_tags.join(","))
                .collect::<Vec<_>>()
        );
    }
    let blog = page(&build, "blog.html");
    let config: Config =
        toml::from_str(&fs::read_to_string(root().join("site.toml")).unwrap()).unwrap();
    let filters: Vec<(String, String)> = select(&blog, ".filter-btn")
        .iter()
        .map(|node| {
            (
                node.value().attr("data-filter").unwrap().to_owned(),
                node.text().collect(),
            )
        })
        .collect();
    let expected_filters = std::iter::once(("all".to_owned(), "All Topics".to_owned()))
        .chain(
            config
                .topics
                .into_iter()
                .map(|topic| (topic.id, topic.label)),
        )
        .collect::<Vec<_>>();
    assert_eq!(filters, expected_filters);
    assert_eq!(select(&home, ".theme-toggle-btn").len(), 1);
    assert_eq!(
        select(&home, "#contact img")[0].value().attr("src"),
        Some("email.svg")
    );
}

#[test]
fn current_articles_render_their_authored_titles_dates_and_tags() {
    let build = compile();
    for (source, post) in authored_posts() {
        let route = article_route(&source, &post);
        let generated = page(&build, &route);
        let titles: Vec<String> = select(&generated, "h1.post-title")
            .iter()
            .map(|node| node.text().collect())
            .collect();
        assert_eq!(titles, [post.title], "{route}");
        let dates: Vec<_> = select(&generated, ".post-header time")
            .iter()
            .map(|node| node.value().attr("datetime").unwrap())
            .collect();
        assert_eq!(dates, [post.date.to_string()], "{route}");
        let tags: Vec<String> = select(&generated, ".post-header .tags-list .tag")
            .iter()
            .map(|node| node.text().collect())
            .collect();
        assert_eq!(
            tags,
            post.tags
                .iter()
                .map(|tag| format!("#{tag}"))
                .collect::<Vec<_>>(),
            "{route}"
        );
        assert_eq!(select(&generated, ".post-content").len(), 1, "{route}");
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
        let project_tags: Vec<String> = card
            .select(&Selector::parse(".tag").unwrap())
            .map(|node| node.text().collect())
            .collect();
        let requested_tag = match project {
            "neocogi-ui" => "UI",
            "neocogi-platform" => "Rendering",
            "neocogi-com" => "COM",
            _ => unreachable!(),
        };
        assert!(project_tags.iter().any(|tag| tag == requested_tag));
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
