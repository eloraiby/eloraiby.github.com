//! Shared document shell, navigation, controls, and context-specific footer.

use maud::{DOCTYPE, Markup, html};

use super::{Context, icon};
use crate::{Result, model::*, resolve};

/// Render complete HTML around already-validated component output.
pub(super) fn document(document: &Document, body: Markup, context: &Context<'_>) -> Result<Markup> {
    let config = &context.site.config;
    let (title, description, home) = match &document.metadata {
        Metadata::Page(page) => (
            page.title.clone(),
            &page.description,
            page.layout == Layout::Home,
        ),
        Metadata::Post(post) => (
            format!("{} - {}", post.title, config.author),
            &post.description,
            false,
        ),
        _ => unreachable!("only standalone documents reach layout rendering"),
    };
    let navigation = navigation(document, home, context)?;
    let footer = footer(document, home, context)?;
    Ok(html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { (title) }
                meta name="description" content=(description);
                meta name="author" content=(&config.author);
                @if let Metadata::Page(page) = &document.metadata {
                    @if let Some(keywords) = &page.keywords { meta name="keywords" content=(keywords); }
                    @if let Some(summary) = &page.social_description {
                        meta property="og:type" content="website";
                        meta property="og:title" content=(&page.title);
                        meta property="og:description" content=(summary);
                        meta property="og:url" content=(&config.url);
                    }
                }
                link rel="stylesheet" href=(resolve::relative(context.route, "/css/style.css"));
                script src=(resolve::relative(context.route, "/js/main.js")) defer {}
            }
            body { (navigation) main { (body) } (footer) }
        }
    })
}

/// Render relative navigation links and the exact selectors used by main.js.
fn navigation(document: &Document, home: bool, context: &Context<'_>) -> Result<Markup> {
    let config = &context.site.config;
    let home_url = resolve::relative(context.route, "/index.html");
    let mut items = Vec::new();
    // Configuration references are based at content/, independent of the current page.
    let config_origin = &context.site.documents[&context.site.root.join("content/index.md")];
    for (index, item) in config.navigation.iter().enumerate() {
        let mut url = resolve::link(context.site, config_origin, context.route, &item.href)?;
        if home && url.starts_with("index.html#") {
            url = url[10..].to_owned();
        }
        if !home && index == 0 {
            url = home_url.clone();
        }
        if item.blog && matches!(document.metadata, Metadata::Post(_)) {
            url = resolve::relative(context.route, "/blog/index.html");
        }
        let active = (home && index == 0) || (!home && item.blog);
        items.push(html! { li { a href=(url) class=(if active { "nav-link active" } else { "nav-link" }) { (&item.label) } } });
    }
    Ok(html! {
        header class="site-header" {
            div class="container nav-container" {
                a href=(home_url) class="brand-link" { span class="brand-badge" { (&config.badge) } span { (&config.author) } }
                nav { ul class="nav-menu" { @for item in items { (item) } } }
                div class="nav-actions" {
                    button class="theme-toggle-btn" aria-label=(if home { "Toggle light/dark theme" } else { "Toggle theme" }) title=[home.then_some("Toggle theme")] {
                        (icon(Icon::Sun, None, Some("theme-icon-dark")))
                        (icon(Icon::Moon, None, Some("theme-icon-light")))
                    }
                    button class="mobile-menu-btn" aria-label=(if home { "Toggle navigation menu" } else { "Toggle menu" }) aria-expanded="false" {
                        (icon(Icon::Menu, Some(20), None))
                    }
                }
            }
        }
    })
}

/// Render footer variants from configuration rather than duplicated page markup.
fn footer(_document: &Document, home: bool, context: &Context<'_>) -> Result<Markup> {
    let config = &context.site.config;
    let origin = &context.site.documents[&context.site.root.join("content/index.md")];
    let socials = if home {
        &config.socials
    } else {
        &config.blog_socials
    };
    let mut links = Vec::new();
    for action in socials {
        let mut href = resolve::link(context.site, origin, context.route, &action.href)?;
        if home && href == "index.html#contact" {
            href = "#contact".into();
        }
        links.push(html! { a href=(href) class="footer-social-link" title=[action.title.as_deref()]
            target=[action.new_tab.then_some("_blank")] rel=[action.new_tab.then_some("noopener")] { (&action.label) } });
    }
    let copyright = format!(
        "© {} {}. All rights reserved.",
        config.copyright_year, config.author
    );
    Ok(
        html! { footer class="site-footer" { div class="container footer-content" {
            @if home { div { p { (copyright) } p style="font-size: 0.8rem; margin-top: 0.25rem;" { (&config.footer_note) } } }
            @else { p { (copyright) } }
            div class="footer-socials" { @for link in links { (link) } }
        } } },
    )
}
