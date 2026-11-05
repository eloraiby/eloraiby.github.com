//! Semantic components extracted from the existing portfolio and article pages.

use maud::{Markup, html};

use super::{Context, Style, blocks, icon, inline, intro, intro_content};
use crate::{Error, Result, markdown, model::*, resolve};

/// Dispatch one typed directive. Every branch owns its structural HTML contract.
pub(super) fn component(
    document: &Document,
    directive: &Directive,
    body: &[Block],
    context: &mut Context<'_>,
) -> Result<Markup> {
    match directive {
        Directive::Include { src, view } => include(document, src, *view, context),
        Directive::Cards { kind } => {
            let records = resolve::card_sources(context.site, document, body, *kind)?;
            let mut cards = Vec::new();
            for record in records {
                if context.stack.contains(&record.path) {
                    return Err(Error(format!(
                        "card reference cycle: {}",
                        record.path.display()
                    )));
                }
                if context.stack.len() >= 64 {
                    return Err(Error(
                        "component reference nesting exceeds 64 documents".into(),
                    ));
                }
                context.stack.push(record.path.clone());
                let card = match &record.metadata {
                    Metadata::Project(value) => project(record, value, context),
                    Metadata::Publication(value) => publication(record, value, context),
                    _ => unreachable!("card record kinds were checked by resolution"),
                };
                context.stack.pop();
                cards.push(card?);
            }
            Ok(
                html! { div class=(match kind { CardKind::Project => "projects-grid", CardKind::Publication => "publications-list" }) { @for card in cards { (card) } } },
            )
        }
        Directive::Posts { view, eyebrow } => {
            posts(document, body, *view, eyebrow.as_deref(), context)
        }
        Directive::Callout => {
            let content = blocks(document, body, Style::Inline, context)?;
            Ok(html! { div class="post-callout" { (content) } })
        }
        Directive::Figure { src, alt, dark } => {
            if alt.trim().is_empty() {
                return Err(Error("figures require nonempty alternative text".into()));
            }
            let source = resolve::link(context.site, document, context.route, src)?;
            let variant = dark
                .as_ref()
                .map(|path| resolve::link(context.site, document, context.route, path))
                .transpose()?;
            let caption = blocks(document, body, Style::Inline, context)?;
            Ok(html! { figure style="margin: 2rem 0;" {
                img src=(source) alt=(alt) class=[variant.as_ref().map(|_| "theme-diagram-light")]
                    style="width: 100%; border-radius: var(--radius-md); border: 1px solid var(--border-color); box-shadow: var(--shadow-md);";
                @if let Some(source) = variant { img src=(source) alt=(alt) class="theme-diagram-dark"
                    style="width: 100%; border-radius: var(--radius-md); border: 1px solid var(--border-color); box-shadow: var(--shadow-md);"; }
                figcaption style="font-size: 0.85rem; color: var(--text-muted); text-align: center; margin-top: 0.75rem; font-family: var(--font-mono);" { (caption) }
            } })
        }
        Directive::AboutCard => {
            if !matches!(document.metadata, Metadata::Fragment(_)) {
                return Err(Error("about-card requires a fragment source".into()));
            }
            let content = blocks(document, body, Style::About, context)?;
            Ok(html! { div class="about-card" { (content) } })
        }
        Directive::Timeline => {
            let Metadata::Fragment(fragment) = &document.metadata else {
                return Err(Error("timeline requires a fragment with milestones".into()));
            };
            let mut milestones = Vec::new();
            for milestone in &fragment.milestones {
                let description = inline(document, &milestone.description, context)?;
                milestones.push(html! { div class="timeline-item" {
                    div class="timeline-role" { (&milestone.title) }
                    div class="timeline-meta" { (&milestone.meta) }
                    div class="timeline-desc" { (description) }
                } });
            }
            Ok(html! { div class="timeline" { @for milestone in milestones { (milestone) } } })
        }
        Directive::Guide => {
            let content = blocks(document, body, Style::Guide, context)?;
            Ok(
                html! { div style="margin-top: 3.5rem; background: var(--bg-surface); border: 1px dashed var(--border-color); border-radius: var(--radius-md); padding: 1.5rem;" { (content) } },
            )
        }
    }
}

/// Expand a fragment with an active-stack cycle check, preserving its source base.
fn include(
    origin: &Document,
    reference: &str,
    view: View,
    context: &mut Context<'_>,
) -> Result<Markup> {
    let document = resolve::source(context.site, origin, reference)?;
    let Metadata::Fragment(fragment) = &document.metadata else {
        return Err(Error("include requires kind = \"fragment\"".into()));
    };
    if context.stack.contains(&document.path) {
        let chain = context
            .stack
            .iter()
            .chain(std::iter::once(&document.path))
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        return Err(Error(format!("include cycle: {chain}")));
    }
    if context.stack.len() >= 64 {
        return Err(Error("include nesting exceeds 64 documents".into()));
    }
    context.stack.push(document.path.clone());
    let rendered = fragment_view(document, fragment, view, context);
    context.stack.pop();
    rendered
}

/// Render one of the explicitly supported fragment presentations.
fn fragment_view(
    document: &Document,
    fragment: &Fragment,
    view: View,
    context: &mut Context<'_>,
) -> Result<Markup> {
    match view {
        View::Markdown => blocks(document, &document.blocks, Style::Normal, context),
        View::Hero => {
            let status = fragment
                .status
                .as_deref()
                .ok_or_else(|| Error("hero requires status metadata".into()))?;
            let location = fragment
                .location
                .as_deref()
                .ok_or_else(|| Error("hero requires location metadata".into()))?;
            let content = blocks(document, &document.blocks, Style::Hero, context)?;
            let actions = actions(document, &fragment.actions, false, context)?;
            Ok(
                html! { section id=(&fragment.id) class="hero" { div class="container" { div class="hero-content" { div {
                    div class="hero-badge-row" {
                        span class="pill-badge status-badge" { span class="status-dot" {} (status) }
                        span class="pill-badge" { (icon(Icon::Location, Some(14), None)) (location) }
                    }
                    (content)
                    div class="hero-tags" { (tags(&fragment.tags, false)) }
                    div class="hero-actions" { @for action in actions { (action) } }
                } } } } },
            )
        }
        View::Highlights => {
            if fragment.highlights.is_empty() {
                return Err(Error("highlights view requires highlights metadata".into()));
            }
            if !document.blocks.is_empty() {
                return Err(Error("highlights view accepts metadata only".into()));
            }
            Ok(
                html! { section id=(&fragment.id) style="padding: 2.5rem 0; background-color: var(--bg-surface);" { div class="container" {
                    div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 1.5rem; text-align: center;" {
                        @for highlight in &fragment.highlights { div style="padding: 1rem;" {
                            div style=(format!("font-size: 1.5rem; font-weight: 800; color: var(--{}); margin-bottom: 0.25rem;", if highlight.accent { "accent" } else { "text-primary" })) { (&highlight.title) }
                            div style="font-size: 0.85rem; color: var(--text-muted);" { (&highlight.description) }
                        } }
                    }
                } } },
            )
        }
        View::Projects | View::Publications | View::About => {
            let header = intro(
                document,
                &document.blocks,
                Style::Section,
                fragment.eyebrow.as_deref(),
                context,
            )?;
            let content = blocks(document, &document.blocks[1..], Style::Normal, context)?;
            Ok(html! { section id=(&fragment.id) { div class="container" {
                (header)
                @if matches!(view, View::About) { div class="about-grid" { (content) } }
                @else { (content) }
            } } })
        }
        View::Contact => {
            let content = blocks(document, &document.blocks, Style::Normal, context)?;
            let image = fragment
                .image
                .as_ref()
                .ok_or_else(|| Error("contact requires image metadata".into()))?;
            let image = resolve::link(context.site, document, context.route, image)?;
            let actions = actions(document, &fragment.actions, false, context)?;
            Ok(
                html! { section id=(&fragment.id) { div class="container" { div class="contact-card" {
                    @if let Some(label) = &fragment.eyebrow { span class="section-tag" { (label) } }
                    (content)
                    div style="margin: 1.5rem auto 2.5rem; display: inline-flex; padding: 0.75rem 1.5rem; background: var(--bg-surface-elevated); border: 1px solid var(--border-color); border-radius: var(--radius-md);" {
                        img src=(image) alt="Contact Email" class="theme-adaptive-diagram" style="height: 28px; display: block;";
                    }
                    div class="contact-links-grid" { @for action in actions { (action) } }
                } } } },
            )
        }
    }
}

/// Render a project card with one icon representation and optional star count.
pub(super) fn project(
    document: &Document,
    project: &Project,
    context: &mut Context<'_>,
) -> Result<Markup> {
    let repository = project
        .repository
        .as_ref()
        .map(|path| resolve::link(context.site, document, context.route, path))
        .transpose()?;
    let image = project
        .image
        .as_ref()
        .map(|path| resolve::link(context.site, document, context.route, path))
        .transpose()?;
    let description = blocks(document, &document.blocks, Style::Inline, context)?;
    let destination = resolve::link(context.site, document, context.route, &project.action.href)?;
    Ok(html! { div class="card" {
        div class="card-header" {
            div class="card-icon-title" {
                @if let Some(image) = image { img src=(image) alt=[project.image_alt.as_deref()] class="card-icon"; }
                @else if let Some(text) = &project.icon_text { div class="card-icon" style="display:flex;align-items:center;justify-content:center;font-weight:800;color:var(--accent);" { (text) } }
                h3 class="card-title" {
                    @if let Some(url) = repository { a href=(url) target="_blank" rel="noopener" { (&project.title) } }
                    @else { (&project.title) }
                }
            }
            @if let Some(stars) = project.stars { div class="card-stars" { (icon(Icon::Star, None, None)) (stars) } }
        }
        div class="card-body" { (description) }
        div class="card-footer" {
            div class="tags-list" { (tags(&project.tags, false)) }
            a href=(destination) class="card-link-arrow" target=[project.action.new_tab.then_some("_blank")] rel=[project.action.new_tab.then_some("noopener")] { (&project.action.label) }
        }
    } })
}

/// Render publication typography and optional dissertation highlighting.
pub(super) fn publication(
    document: &Document,
    publication: &Publication,
    context: &mut Context<'_>,
) -> Result<Markup> {
    let title = inline(document, &publication.title, context)?;
    let destination = resolve::link(context.site, document, context.route, &publication.href)?;
    let description = blocks(document, &document.blocks, Style::Publication, context)?;
    let action = actions(
        document,
        std::slice::from_ref(&publication.action),
        true,
        context,
    )?;
    Ok(
        html! { div class="pub-item" style=[publication.featured.then_some("border-left: 3px solid var(--accent);")] {
            div class="pub-meta" { span class="pub-year" { (publication.year) } span { (&publication.venue) } }
            h3 class="pub-title" { a href=(destination) target=[publication.new_tab.then_some("_blank")] rel=[publication.new_tab.then_some("noopener")] { (title) } }
            (description)
            div class="pub-actions" { @for action in action { (action) } }
        } },
    )
}

/// Render shared action data, including the hero's trailing arrow placement.
fn actions(
    document: &Document,
    values: &[Action],
    small: bool,
    context: &mut Context<'_>,
) -> Result<Vec<Markup>> {
    let mut rendered = Vec::new();
    for action in values {
        let destination = resolve::link(context.site, document, context.route, &action.href)?;
        let class = format!(
            "btn btn-{}{}",
            match action.style {
                ButtonStyle::Primary => "primary",
                ButtonStyle::Secondary => "secondary",
            },
            if small { " btn-sm" } else { "" }
        );
        rendered.push(html! { a href=(destination) class=(class) target=[action.new_tab.then_some("_blank")] rel=[action.new_tab.then_some("noopener")] title=[action.title.as_deref()] {
            @if let Some(kind) = action.icon { @if !matches!(kind, Icon::Arrow) { (icon(kind, Some(if small { 14 } else { 16 }), None)) } }
            (&action.label)
            @if matches!(action.icon, Some(Icon::Arrow)) { (icon(Icon::Arrow, Some(16), None)) }
        } });
    }
    Ok(rendered)
}

/// Generate visible tags with optional hash prefixes; filtering uses separate IDs.
fn tags(values: &[String], hashes: bool) -> Markup {
    html! { @for tag in values { span class="tag" { @if hashes { "#" } (tag) } } }
}

/// Reading-time overrides preserve current labels; otherwise count authored text,
/// code, and nested components at 200 whitespace-separated words per minute.
fn reading_minutes(document: &Document, post: &Post) -> u32 {
    /// Count source text recursively without including metadata or directive syntax.
    fn words(blocks: &[Block]) -> usize {
        blocks
            .iter()
            .map(|block| match &block.content {
                BlockContent::Markdown(text) => {
                    pulldown_cmark::Parser::new_ext(text, markdown::options())
                        .map(|event| match event {
                            pulldown_cmark::Event::Text(text)
                            | pulldown_cmark::Event::Code(text) => text.split_whitespace().count(),
                            _ => 0,
                        })
                        .sum()
                }
                BlockContent::Component { body, .. } => words(body),
            })
            .sum()
    }
    post.reading_minutes
        .unwrap_or_else(|| words(&document.blocks).div_ceil(200).max(1) as u32)
}

/// Render an article shell around Markdown, maintaining existing classes and links.
pub(super) fn article(
    document: &Document,
    post: &Post,
    context: &mut Context<'_>,
) -> Result<Markup> {
    let date = resolve::date(post)?;
    let content = blocks(document, &document.blocks, Style::Normal, context)?;
    let blog = resolve::relative(context.route, "/blog/index.html");
    let contact = format!(
        "{}#contact",
        resolve::relative(context.route, "/index.html")
    );
    Ok(html! { article class="content-container" {
        header class="post-header" {
            a href=(&blog) class="post-back-link" { "← Back to all blog entries" }
            h1 class="post-title" { (&post.title) }
            div class="post-meta-row" {
                time datetime=(date.format("%Y-%m-%d")) class="blog-date" { (icon(Icon::Calendar, Some(16), None)) (date.format("%B %-d, %Y")) }
                span class="reading-time" { "• " (reading_minutes(document, post)) " min read" }
                span class="author-name" { "• By " (&context.site.config.author) }
            }
            div class="tags-list" { (tags(&post.tags, true)) }
        }
        div class="post-content" { (content) }
        footer class="post-footer-nav" {
            a href=(blog) class="btn btn-secondary btn-sm" { "← Back to Blog Index" }
            a href=(contact) class="btn btn-secondary btn-sm" { "Discuss this post →" }
        }
    } })
}

/// Render one article listing card with page-appropriate heading level and excerpt.
fn post_card(
    document: &Document,
    post: &Post,
    preview: bool,
    context: &mut Context<'_>,
) -> Result<Markup> {
    let date = resolve::date(post)?;
    let destination = resolve::relative(context.route, &resolve::post_route(document, post)?);
    let (excerpt, visible_tags) = if preview {
        let home = post
            .home
            .as_ref()
            .expect("preview selection requires homepage metadata");
        (&home.excerpt, &home.tags)
    } else {
        (&post.excerpt, &post.tags)
    };
    let excerpt = inline(document, excerpt, context)?;
    Ok(
        html! { article class="blog-entry-card" data-tags=(post.filter_tags.join(",")) {
            div class="blog-entry-header" {
                time datetime=(date.format("%Y-%m-%d")) class="blog-date" { (icon(Icon::Calendar, Some(16), None)) (date.format("%B %-d, %Y")) }
                span class="reading-time" { (reading_minutes(document, post)) " min read" }
            }
            @if preview { h3 class="blog-entry-title" { a href=(&destination) { (&post.title) } } }
            @else { h2 class="blog-entry-title" { a href=(&destination) { (&post.title) } } }
            p class="blog-entry-excerpt" { (excerpt) }
            div class="blog-entry-footer" {
                div class="tags-list" { (tags(visible_tags, true)) }
                a href=(destination) class="card-link-arrow" { @if preview { "Read Post →" } @else { "Read Article →" } }
            }
        } },
    )
}

/// Generate deterministic post collections with explicit editorial preview ordering.
fn posts(
    document: &Document,
    body: &[Block],
    view: PostView,
    eyebrow: Option<&str>,
    context: &mut Context<'_>,
) -> Result<Markup> {
    let preview = matches!(view, PostView::Preview);
    let mut entries = context
        .site
        .documents
        .values()
        .filter_map(|document| match &document.metadata {
            Metadata::Post(post) if !preview || post.home.is_some() => Some((document, post)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if preview {
        entries.sort_by_key(|(_, post)| post.home.as_ref().unwrap().order);
    } else {
        entries.sort_by(|(a, pa), (b, pb)| {
            pb.date
                .to_string()
                .cmp(&pa.date.to_string())
                .then_with(|| a.path.cmp(&b.path))
        });
    }
    let mut cards = Vec::new();
    for (document, post) in entries {
        cards.push(post_card(document, post, preview, context)?);
    }
    let list = html! { div class="blog-list" { @for card in cards { (card) } } };
    if preview {
        let header = intro_content(document, body, Style::Section, eyebrow, context)?;
        if body.len() != 1 {
            return Err(Error(
                "post preview requires only a heading and description".into(),
            ));
        }
        let blog = resolve::relative(context.route, "/blog.html");
        Ok(html! { section id="blog-preview" { div class="container" {
            div class="section-header" style="display:flex; justify-content:space-between; align-items:flex-end; flex-wrap:wrap; gap:1rem;" {
                div { (header) } a href=(blog) class="btn btn-secondary" { "View All Entries →" }
            }
            (list)
        } } })
    } else {
        if eyebrow.is_some() {
            return Err(Error("post list does not accept a preview eyebrow".into()));
        }
        if !body.is_empty() {
            return Err(Error("post list requires an empty body".into()));
        }
        Ok(html! {
            div class="filter-tabs" {
                button class="filter-btn active" data-filter="all" { "All Topics" }
                @for topic in &context.site.config.topics { button class="filter-btn" data-filter=(&topic.id) { (&topic.label) } }
            }
            (list)
        })
    }
}
