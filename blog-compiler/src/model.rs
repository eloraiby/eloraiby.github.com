//! Typed authoring contracts. Metadata never stores HTML or arbitrary payloads.

use std::{collections::BTreeMap, path::PathBuf};

use serde::Deserialize;

/// Site-wide configuration; asset mounts retain their existing published paths.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Public origin used for Open Graph metadata.
    pub url: String,
    /// Default article author and site identity.
    pub author: String,
    /// Initials displayed in the navigation badge.
    pub badge: String,
    /// Explicit copyright year, independent of the build clock.
    pub copyright_year: u16,
    /// Optional explanatory line displayed on the homepage footer.
    pub footer_note: String,
    /// Asset paths relative to the site root, mounted at identical output paths.
    pub assets: Vec<String>,
    /// Navigation links in editorial order.
    pub navigation: Vec<NavLink>,
    /// Homepage social links in editorial order.
    pub socials: Vec<Action>,
    /// Reduced social links used by article and blog layouts.
    pub blog_socials: Vec<Action>,
    /// Filter identifiers and their visible labels, in editorial order.
    pub topics: Vec<Topic>,
}

/// A navigation destination expressed as a source Markdown link or an external URL.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavLink {
    /// Visible navigation text.
    pub label: String,
    /// Source link, resolved relative to the content directory.
    pub href: String,
    /// Navigation group selected on blog pages and posts.
    #[serde(default)]
    pub blog: bool,
}

/// Stable filter identity separated from its display spelling.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Topic {
    /// Lowercase identifier matched against article filter tags.
    pub id: String,
    /// Display label including any hash prefix.
    pub label: String,
}

/// Shared button and social-link data; structure and attributes belong to Rust.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    /// Visible link text, including an optional authored arrow.
    pub label: String,
    /// Source-relative Markdown/asset reference or supported external URL.
    pub href: String,
    /// Visual treatment, selected from a closed set.
    #[serde(default)]
    pub style: ButtonStyle,
    /// Optional built-in vector icon.
    pub icon: Option<Icon>,
    /// Whether to open a separate browser tab.
    #[serde(default)]
    pub new_tab: bool,
    /// Optional descriptive tooltip.
    pub title: Option<String>,
}

/// Button appearance supported by the existing stylesheet.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ButtonStyle {
    /// Neutral bordered action.
    #[default]
    Secondary,
    /// Gradient primary action.
    Primary,
}

/// Every interface SVG has one authoritative Rust implementation.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Icon {
    /// Theme control's sun glyph.
    Sun,
    /// Theme control's moon glyph.
    Moon,
    /// Mobile navigation's three bars.
    Menu,
    /// Location marker in the self introduction.
    Location,
    /// Right-pointing action arrow.
    Arrow,
    /// GitHub mark.
    Github,
    /// X/Twitter mark.
    Twitter,
    /// LinkedIn mark.
    Linkedin,
    /// Project star count.
    Star,
    /// Publication download action.
    Download,
    /// Article publication date.
    Calendar,
    /// Publishing-guide callout.
    Edit,
}

/// Discriminated metadata prevents treating a fragment as an article or a project.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Metadata {
    /// A standalone composition page.
    Page(Page),
    /// A dated article with a generated article shell.
    Post(Post),
    /// Reusable content rendered through an explicitly selected view.
    Fragment(Fragment),
    /// A project record rendered inside a project collection.
    Project(Project),
    /// An academic publication record.
    Publication(Publication),
}

/// Metadata common to standalone composition pages.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    /// Layout determines navigation state and page-width treatment.
    pub layout: Layout,
    /// Absolute output URL ending in `.html`.
    pub route: String,
    /// Additional full-page output routes rendered with rebased links.
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Plain-text document title.
    pub title: String,
    /// Search-engine description.
    pub description: String,
    /// Optional homepage search keywords.
    pub keywords: Option<String>,
    /// Optional Open Graph summary.
    pub social_description: Option<String>,
    /// Optional label above a blog index title.
    pub eyebrow: Option<String>,
}

/// Standalone layouts supported by the current site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Layout {
    /// Full-width portfolio sections.
    Home,
    /// Constrained index with topic filters.
    Blog,
}

/// An article's authoritative metadata and optional homepage presentation.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Post {
    /// Visible article title, also used in listings.
    pub title: String,
    /// Search-engine description.
    pub description: String,
    /// TOML calendar date; time and timezone components are rejected.
    pub date: toml::value::Datetime,
    /// Optional explicit route; otherwise derived from the Markdown filename.
    pub route: Option<String>,
    /// Listing excerpt, supporting inline Markdown.
    pub excerpt: String,
    /// Visible article tags, preserving authored spelling.
    pub tags: Vec<String>,
    /// Stable filter identifiers, separate from display tags.
    pub filter_tags: Vec<String>,
    /// Optional reading-time override; otherwise calculated at 200 words/minute.
    pub reading_minutes: Option<u32>,
    /// Optional curated homepage preview.
    pub home: Option<Preview>,
}

/// Homepage-specific presentation; order is explicit and unique across posts.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preview {
    /// Positive editorial ordering key.
    pub order: u32,
    /// Homepage excerpt, which may differ from the full listing.
    pub excerpt: String,
    /// Visible homepage tags.
    pub tags: Vec<String>,
}

/// Reusable section metadata; content itself remains Markdown blocks.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fragment {
    /// Stable section anchor for navigation.
    pub id: String,
    /// Optional small label above the main section heading.
    pub eyebrow: Option<String>,
    /// Self introduction availability badge text.
    pub status: Option<String>,
    /// Self introduction location badge text.
    pub location: Option<String>,
    /// Self introduction expertise tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Self introduction or contact action links.
    #[serde(default)]
    pub actions: Vec<Action>,
    /// Career highlights shown in the compact statistics strip.
    #[serde(default)]
    pub highlights: Vec<Highlight>,
    /// About-section timeline records.
    #[serde(default)]
    pub milestones: Vec<Milestone>,
    /// Optional contact artwork, resolved as an asset reference.
    pub image: Option<String>,
}

/// One compact career-highlight entry.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Highlight {
    /// Prominent organization or qualification.
    pub title: String,
    /// Supporting role text.
    pub description: String,
    /// Whether to use the existing accent color.
    #[serde(default)]
    pub accent: bool,
}

/// One authored milestone; descriptions support inline Markdown.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Milestone {
    /// Role or organization.
    pub title: String,
    /// Supporting date or role label.
    pub meta: String,
    /// Description of the work or qualification.
    pub description: String,
}

/// A project card independent of its placement in a page.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    /// Visible project name.
    pub title: String,
    /// Optional title destination; some cards are plain headings.
    pub repository: Option<String>,
    /// Optional raster icon asset.
    pub image: Option<String>,
    /// Alternative textual badge when no raster icon is supplied.
    pub icon_text: Option<String>,
    /// Authored image alternative text.
    pub image_alt: Option<String>,
    /// Optional fixed star count; builds never fetch remote counts.
    pub stars: Option<u32>,
    /// Optional full-width project screenshot, separate from the small card icon.
    pub screenshot: Option<ProjectScreenshot>,
    /// Visible technology labels.
    pub tags: Vec<String>,
    /// Footer destination and label.
    pub action: Action,
}

/// A project preview image and optional authored caption, rendered as a figure.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectScreenshot {
    /// Source-relative image reference or supported external image URL.
    pub src: String,
    /// Required accessible description of the actual screenshot content.
    pub alt: String,
    /// Optional inline Markdown caption, including source or artwork credits.
    pub caption: Option<String>,
}

/// A publication record with an inline-Markdown title and Markdown description.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Publication {
    /// Title may emphasize mathematical variables using Markdown.
    pub title: String,
    /// Publication year.
    pub year: u16,
    /// Journal, publisher, or institution.
    pub venue: String,
    /// Title destination.
    pub href: String,
    /// Whether the title opens another browser tab.
    #[serde(default)]
    pub new_tab: bool,
    /// Highlighted thesis treatment.
    #[serde(default)]
    pub featured: bool,
    /// Publication action button.
    pub action: Action,
}

/// Supported fragment presentations, each implemented in Rust.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum View {
    /// Introductory heading, badges, tags, and actions.
    #[serde(rename = "self")]
    SelfIntro,
    /// Compact career highlights.
    Highlights,
    /// Project section with an ordered card list.
    Projects,
    /// Publication section with an ordered record list.
    Publications,
    /// Biography cards and timeline.
    About,
    /// Contact artwork and social actions.
    Contact,
    /// Include ordinary Markdown without adding a section wrapper.
    Markdown,
}

/// Parsed semantic directives; every attribute set is checked during parsing.
#[derive(Debug)]
pub enum Directive {
    /// Embed a fragment, resolving references relative to its own source file.
    Include {
        /// Source-relative fragment reference.
        src: String,
        /// Rust-owned presentation selected by the parent document.
        view: View,
    },
    /// Render project or publication records referenced by a Markdown list.
    Cards {
        /// Expected kind of every record referenced by the body.
        kind: CardKind,
    },
    /// Generate the full post listing or curated homepage previews.
    Posts {
        /// Collection membership, ordering, and presentation contract.
        view: PostView,
        /// Optional authored label above the preview heading.
        eyebrow: Option<String>,
    },
    /// Render an article callout containing Markdown.
    Callout,
    /// Render an image with a Markdown caption and optional dark-theme variant.
    Figure {
        /// Default source-relative image reference.
        src: String,
        /// Required accessible description.
        alt: String,
        /// Optional source-relative dark-theme image.
        dark: Option<String>,
    },
    /// One biography card containing a heading, prose, or a timeline directive.
    AboutCard,
    /// Render milestones owned by the containing fragment.
    Timeline,
    /// Styled publishing-guide note, containing Markdown.
    Guide,
}

/// Records accepted by a card collection.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CardKind {
    /// Project cards.
    Project,
    /// Publication records.
    Publication,
}

/// Article collection presentation; ordering follows the selected view's contract.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PostView {
    /// All posts ordered by descending date, with source path breaking ties.
    List,
    /// Only posts with homepage metadata, ordered by its explicit key.
    Preview,
}

/// A source block retains its original line for diagnostics after inclusion.
#[derive(Debug)]
pub struct Block {
    /// One-based location of the block in its original file.
    pub line: usize,
    /// Parsed block content.
    pub content: BlockContent,
}

/// Syntax preserves Markdown text until references have been resolved.
#[derive(Debug)]
pub enum BlockContent {
    /// An uninterrupted Markdown region.
    Markdown(String),
    /// A semantic container and its recursively parsed body.
    Component {
        directive: Directive,
        body: Vec<Block>,
    },
}

/// An owned source document; its path identifies the base for relative references.
#[derive(Debug)]
pub struct Document {
    /// Canonical path within the content directory.
    pub path: PathBuf,
    /// Typed front matter.
    pub metadata: Metadata,
    /// Parsed Markdown and semantic containers.
    pub blocks: Vec<Block>,
}

/// Complete in-memory site. Routes and source documents each have one owner.
#[derive(Debug)]
pub struct SiteData {
    /// Canonical filesystem root containing configuration, content, and assets.
    pub root: PathBuf,
    /// Validated site-wide settings.
    pub config: Config,
    /// Documents indexed by canonical source path.
    pub documents: BTreeMap<PathBuf, Document>,
    /// Output routes mapped to their owning source document, including aliases.
    pub routes: BTreeMap<String, PathBuf>,
    /// Canonical asset source paths mapped to their output URL.
    pub assets: BTreeMap<PathBuf, String>,
}
