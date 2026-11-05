//! Rust-owned HTML components. Source Markdown never supplies raw HTML structure.

mod components;
mod icons;
mod layout;

use std::{collections::BTreeMap, path::PathBuf};

use maud::{Markup, PreEscaped, html};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::{Error, Result, markdown, model::*, resolve};

pub(crate) use icons::icon;

/// Per-page rendering state. Includes borrow source records; only traversal and
/// heading counters mutate, with no shared state between output aliases.
pub(crate) struct Context<'a> {
    /// All resolved source documents and global configuration.
    site: &'a SiteData,
    /// Current published route, including an alias when applicable.
    route: &'a str,
    /// Active include chain, used for cycle detection and useful diagnostics.
    stack: Vec<PathBuf>,
    /// Heading slug counters scoped to this output page.
    headings: BTreeMap<String, usize>,
}

/// Contextual typography for existing HTML components.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Style {
    /// Ordinary article Markdown.
    Normal,
    /// Hero heading, gradient emphasis, and subtitle.
    Hero,
    /// Homepage section heading and subtitle.
    Section,
    /// Blog index title and subtitle.
    Blog,
    /// Biography-card typography.
    About,
    /// Inline content, omitting paragraph wrappers.
    Inline,
    /// Publication description with the existing paragraph class.
    Publication,
    /// Publishing-guide callout typography.
    Guide,
}

/// Compile one standalone page, preserving the context of every included source.
pub(crate) fn page(site: &SiteData, document: &Document, route: &str) -> Result<String> {
    let mut context = Context {
        site,
        route,
        stack: vec![document.path.clone()],
        headings: BTreeMap::new(),
    };
    let body = match &document.metadata {
        Metadata::Page(page) => match page.layout {
            Layout::Home => blocks(document, &document.blocks, Style::Normal, &mut context)?,
            Layout::Blog => {
                let intro = intro(
                    document,
                    &document.blocks,
                    Style::Blog,
                    page.eyebrow.as_deref(),
                    &mut context,
                )?;
                let rest = blocks(document, &document.blocks[1..], Style::Normal, &mut context)?;
                html! { div class="container" style="padding-top: 3.5rem;" { (intro) (rest) } }
            }
        },
        Metadata::Post(post) => components::article(document, post, &mut context)?,
        _ => {
            return Err(Error::at(
                &document.path,
                1,
                "only pages and posts have standalone routes",
            ));
        }
    };
    Ok(layout::document(document, body, &context)?.into_string())
}

/// Validate orphan records too, so `check` covers the complete authoring tree.
pub(crate) fn validate_documents(site: &SiteData) -> Result<()> {
    for document in site.documents.values() {
        let mut context = Context {
            site,
            route: "/index.html",
            stack: vec![document.path.clone()],
            headings: BTreeMap::new(),
        };
        match &document.metadata {
            Metadata::Project(project) => {
                components::project(document, project, &mut context)?;
            }
            Metadata::Publication(publication) => {
                components::publication(document, publication, &mut context)?;
            }
            Metadata::Fragment(_) => {
                blocks(document, &document.blocks, Style::Normal, &mut context)?;
            }
            Metadata::Page(_) | Metadata::Post(_) => {}
        }
    }
    Ok(())
}

/// Render a block sequence while retaining the source identity across containers.
fn blocks(
    document: &Document,
    values: &[Block],
    style: Style,
    context: &mut Context<'_>,
) -> Result<Markup> {
    let mut output = String::new();
    for block in values {
        let result = match &block.content {
            BlockContent::Markdown(text) => markdown(document, text, style, context),
            BlockContent::Component { directive, body } => {
                components::component(document, directive, body, context)
            }
        }
        .map_err(|error| Error::at(&document.path, block.line, error))?;
        output.push_str(&result.into_string());
    }
    // Each piece is compiler-generated markup; author text is escaped by its renderer.
    Ok(PreEscaped(output))
}

/// Extract a required leading Markdown intro from a section or index page.
fn intro(
    document: &Document,
    values: &[Block],
    style: Style,
    eyebrow: Option<&str>,
    context: &mut Context<'_>,
) -> Result<Markup> {
    let content = intro_content(document, values, style, eyebrow, context)?;
    Ok(html! { div class="section-header" { (content) } })
}

/// Render introductory text without a wrapper for the preview's flex header.
fn intro_content(
    document: &Document,
    values: &[Block],
    style: Style,
    eyebrow: Option<&str>,
    context: &mut Context<'_>,
) -> Result<Markup> {
    let Some(Block {
        content: BlockContent::Markdown(text),
        ..
    }) = values.first()
    else {
        return Err(Error::at(
            &document.path,
            1,
            "section requires a leading Markdown heading and description",
        ));
    };
    let expected = if style == Style::Blog {
        HeadingLevel::H1
    } else {
        HeadingLevel::H2
    };
    let first = Parser::new_ext(text, markdown::options()).next();
    if !matches!(first, Some(Event::Start(Tag::Heading { level, .. })) if level == expected) {
        return Err(Error::at(
            &document.path,
            1,
            "section begins with the wrong Markdown heading level",
        ));
    }
    let content = markdown(document, text, style, context)?;
    Ok(html! {
        @if let Some(label) = eyebrow { span class="section-tag" { (label) } }
        (content)
    })
}

/// Render metadata as inline Markdown with the same URL and escaping guarantees.
fn inline(document: &Document, text: &str, context: &mut Context<'_>) -> Result<Markup> {
    markdown(document, text, Style::Inline, context)
}

/// Produce the opening portion of an empty compiler-owned HTML element.
fn opening(markup: Markup) -> String {
    let text = markup.into_string();
    text[..text.rfind("</").unwrap()].to_owned()
}

/// Map Markdown events into the existing typography and resolve links before HTML
/// emission. Raw HTML is rejected; fenced and inline code remains escaped text.
fn markdown(
    document: &Document,
    text: &str,
    style: Style,
    context: &mut Context<'_>,
) -> Result<Markup> {
    let definitions = markdown::definitions(document);
    let mut callback = |broken: pulldown_cmark::BrokenLink<'_>| {
        definitions
            .get(&broken.reference.to_lowercase())
            .map(|(url, title)| (url.clone().into(), title.clone().into()))
    };
    let events: Vec<_> =
        Parser::new_with_broken_link_callback(text, markdown::options(), Some(&mut callback))
            .collect();
    let mut resolved = Vec::new();
    let mut in_heading = false;
    let mut tight_quotes = Vec::new();
    for (index, event) in events.iter().cloned().enumerate() {
        match event {
            Event::Html(_) | Event::InlineHtml(_) => return Err(Error::at(&document.path, 1, "raw HTML is unsupported; use Markdown or a semantic directive")),
            Event::Start(Tag::Link { dest_url, title, .. }) => {
                let url = resolve::link(context.site, document, context.route, &dest_url)?;
                let external = matches!(url::Url::parse(&url), Ok(value) if matches!(value.scheme(), "http" | "https"));
                resolved.push(Event::Html(opening(html! { a href=(url) title=[(!title.is_empty()).then_some(title.as_ref())]
                    target=[external.then_some("_blank")] rel=[external.then_some("noopener")] {} }).into()));
            }
            Event::Start(Tag::Image { link_type, dest_url, title, id }) => {
                resolved.push(Event::Start(Tag::Image { link_type, dest_url: resolve::link(context.site, document, context.route, &dest_url)?.into(), title, id }));
            }
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                let mut title = String::new();
                for item in events[index + 1..].iter() {
                    match item {
                        Event::End(TagEnd::Heading(_)) => break,
                        Event::Text(value) | Event::Code(value) => title.push_str(value),
                        _ => {}
                    }
                }
                let slug = slug(&title);
                let count = context.headings.entry(slug.clone()).or_default(); *count += 1;
                let id = if *count == 1 { slug } else { format!("{slug}-{count}") };
                let class = match style { Style::Hero => Some("hero-title"), Style::Section | Style::Blog => Some("section-title"), _ => None };
                let size = (style == Style::Blog).then_some("font-size: 2.4rem;");
                let tag = match level {
                    HeadingLevel::H1 => html! { h1 id=(id) class=[class] style=[size] {} },
                    HeadingLevel::H2 => html! { h2 id=(id) class=[class] {} },
                    HeadingLevel::H3 if style == Style::Guide => html! { h3 id=(id) style="font-size: 1.1rem; margin-bottom: 0.5rem; display: flex; align-items: center; gap: 0.5rem;" { (icon(Icon::Edit, Some(18), None)) } },
                    HeadingLevel::H3 => html! { h3 id=(id) {} },
                    HeadingLevel::H4 => html! { h4 id=(id) {} },
                    HeadingLevel::H5 => html! { h5 id=(id) {} },
                    HeadingLevel::H6 => html! { h6 id=(id) {} },
                };
                resolved.push(Event::Html(opening(tag).into()));
            }
            Event::Start(Tag::Paragraph) => {
                if style == Style::Inline || tight_quotes.last() == Some(&true) { continue; }
                let class = match style { Style::Hero => Some("hero-subtitle"), Style::Section | Style::Blog => Some("section-subtitle"), Style::Publication => Some("pub-details"), _ => None };
                let css = match style {
                    Style::Blog => Some("font-size: 1.15rem; max-width: 42rem;"),
                    Style::About => Some("color: var(--text-secondary); margin-bottom: 1.25rem;"),
                    Style::Guide => Some("color: var(--text-secondary); font-size: 0.9rem; line-height: 1.55;"),
                    _ => None,
                };
                resolved.push(Event::Html(opening(html! { p class=[class] style=[css] {} }).into()));
            }
            Event::End(TagEnd::Paragraph) if style == Style::Inline || tight_quotes.last() == Some(&true) => {}
            Event::Start(Tag::BlockQuote(kind)) => {
                let mut depth = 1;
                let mut paragraphs = 0;
                for item in &events[index + 1..] {
                    match item {
                        Event::Start(Tag::BlockQuote(_)) => depth += 1,
                        Event::End(TagEnd::BlockQuote(_)) => { depth -= 1; if depth == 0 { break; } }
                        Event::Start(Tag::Paragraph) if depth == 1 => paragraphs += 1,
                        _ => {}
                    }
                }
                tight_quotes.push(paragraphs == 1);
                resolved.push(Event::Start(Tag::BlockQuote(kind)));
            }
            Event::End(TagEnd::BlockQuote(kind)) => { tight_quotes.pop(); resolved.push(Event::End(TagEnd::BlockQuote(kind))); }
            Event::End(TagEnd::Heading(level)) => { in_heading = false; resolved.push(Event::End(TagEnd::Heading(level))); }
            Event::Start(Tag::Strong) if style == Style::Hero && in_heading => resolved.push(Event::Html("<span class=\"gradient-text\">".into())),
            Event::End(TagEnd::Strong) if style == Style::Hero && in_heading => resolved.push(Event::Html("</span>".into())),
            Event::Start(Tag::List(None)) if style == Style::About => resolved.push(Event::Html("<ul style=\"color: var(--text-secondary); margin-left: 1.25rem; margin-bottom: 1rem; display: flex; flex-direction: column; gap: 0.5rem;\">".into())),
            other => resolved.push(other),
        }
    }
    let mut output = String::new();
    pulldown_cmark::html::push_html(&mut output, resolved.into_iter());
    Ok(PreEscaped(output))
}

/// Generate stable Unicode-aware heading anchors; duplicates receive numeric suffixes.
fn slug(title: &str) -> String {
    let mut result = String::new();
    let mut separator = false;
    for character in title.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            if separator && !result.is_empty() {
                result.push('-');
            }
            result.push(character);
            separator = false;
        } else {
            separator = true;
        }
    }
    if result.is_empty() {
        "section".into()
    } else {
        result
    }
}
