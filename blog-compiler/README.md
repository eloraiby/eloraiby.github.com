# Blog compiler

This crate compiles the portfolio and technical blog from Markdown. All compiler
code, Cargo files, and compiler tests live under `blog-compiler/`. Content lives in
`../content/`; `../site.toml` configures the site. Rust owns document structure,
components, and interface SVG geometry. The existing CSS and JavaScript own
appearance and browser interactions.

## Build and check

Run from the repository root:

```sh
cargo run --locked --manifest-path blog-compiler/Cargo.toml -- check
cargo run --locked --manifest-path blog-compiler/Cargo.toml -- build
python3 -m http.server --directory dist 8000
```

The default source root is the compiler crate's parent, even when Cargo is run
from another working directory. Override it with `--root /path/to/site`.
`build --out preview` writes to `SITE_ROOT/preview`; an absolute output path is
also accepted. The output parent directory must already exist.

`check` runs the complete compilation and local-link validation in memory. It
writes nothing. `build` publishes that same validated snapshot into `dist/`.
The library exposes `Site::load`, `Site::compile`, `Build::files`, and
`Build::write` for consumers needing the same contracts without the CLI.

## Source contracts

Every Markdown source starts with TOML front matter between `+++` lines.
Unknown metadata fields and directive attributes are errors. The five document
kinds are `page`, `post`, `fragment`, `project`, and `publication`.

`content/index.md` is required and must be a page with route `/index.html`.
Page layouts are `home` and `blog`. Pages provide `title`, `description`, and a
route; `aliases` optionally publishes the same source at additional routes.
Posts derive `/blog/FILENAME.html` unless they specify `route` explicitly.
Fragments and records have no standalone output route.

Routes begin with `/`, end in `.html`, and contain normal path components.
Duplicate routes, aliases, asset mounts, and homepage preview orders are errors.
Sources and assets must be ordinary files/directories; traversal does not follow
symlinks. Builds do not fetch article content, star counts, fonts, or other remote
resources. Dependencies are recorded in `Cargo.lock`.

### Write an article

Create `content/posts/YYYY-MM-DD-topic.md`:

````markdown
+++
kind = "post"
title = "An article title"
description = "Search-engine summary."
date = 2026-11-05
excerpt = "A listing summary with **optional emphasis**."
tags = ["Rust", "Systems"]
filter_tags = ["rust", "systems"]
+++

The introductory paragraph.

## First section

```rust
fn main() {
    println!("Hello!");
}
```

:::callout
**Takeaway:** Markdown content inside a styled callout.
:::
````

The article shell supplies the title's H1, author, date, tags, navigation, and
footer. Start body sections at H2. A date is a TOML calendar date without a time.
`tags` preserves visible spellings; `filter_tags` contains lowercase identifiers
matched by `site.toml` topic filters. The listing includes every `post`, ordered
by descending date with source-path ordering for ties. There is no draft status
or implicit scheduling in this version.

`reading_minutes = 7` preserves an explicit reading time. Without it, the
compiler counts body text and code recursively at 200 words/minute, rounds up,
and displays at least one minute.

To feature a post on the homepage, append this table to its front matter:

```toml
[home]
order = 1
excerpt = "The homepage's editorial summary."
tags = ["Rust"]
```

Homepage selection uses positive, unique ordering keys. It deliberately supports
different excerpts and visible tags from the full listing. Metadata fields must
precede `[home]` unless they belong to that table, following normal TOML rules.

### Compose pages and fragments

The main page is Markdown, including references such as:

```markdown
:::include src="sections/projects.md" view="projects"
:::
```

Includes must reference `kind = "fragment"` sources. A fragment has an `id`
containing lowercase letters, digits, or hyphens. Its optional `eyebrow` appears
above the section heading. Supported views are:

| View | Content contract |
| --- | --- |
| `hero` | H1 and introduction; metadata supplies `status`, `location`, `tags`, and `actions`. Bold text in the heading receives the gradient treatment. |
| `highlights` | Metadata-only `[[highlights]]` entries with `title`, `description`, and optional `accent`. |
| `projects` | Leading H2 and description, followed by project cards. |
| `publications` | Leading H2 and description, followed by publication cards. |
| `about` | Leading H2 and description, followed by `about-card` containers. |
| `contact` | H2 and introduction; metadata supplies an `image` asset and `actions`. |
| `markdown` | Embed ordinary content without a section wrapper. |

An `about-card` contains Markdown with H3 headings, prose, and lists. An empty
`timeline` directive renders the containing fragment's `[[milestones]]` records,
each with `title`, `meta`, and `description`.

### Reuse project and publication records

```markdown
:::cards kind="project"
- [Delaunay](../projects/delaunay.md)
- [MicroUI](../projects/microui-rs.md)
:::
```

Card bodies are Markdown lists of links whose targets must match the requested
kind. The list determines ordering. Use `kind="publication"` for publications.

A project provides `title`, `tags`, and an `[action]` table. It requires exactly
one of `image` (with nonempty `image_alt`) or `icon_text`. Optional `repository`
makes the title a link; `stars` is a fixed integer. The body is its Markdown
description. See `content/projects/delaunay.md` for a complete record.

A publication provides `title` (inline Markdown), `year`, `venue`, `href`, and an
`[action]` table. Optional `new_tab` controls the title link; `featured` selects
the thesis treatment. Its body is the description. See
`content/publications/centroid-triangulations.md`.

Actions provide `label` and `href`. Optional fields are `style` (`primary` or
`secondary`), `icon`, `new_tab`, and `title`. Built-in icon names are `sun`,
`moon`, `menu`, `location`, `arrow`, `github`, `twitter`, `linkedin`, `star`,
`download`, `calendar`, and `edit`. SVG paths live in `src/render/icons.rs`.

### Listings, figures, and notes

An empty `:::posts view="list"` container generates all article cards and topic
filters. `:::posts view="preview"` takes an H2 and description in its body and
generates the curated homepage section. The preview's section anchor is
`blog-preview`; optional `eyebrow="Technical Writings"` supplies its small label.
`:::guide` wraps a publishing note in the index's existing style.

```markdown
:::figure src="../../images/actor-pacer.png" alt="Actor architecture"
Figure 1: A caption with **Markdown emphasis**.
:::
```

Optional `dark="../../images/actor-pacer-dark.png"` supplies an image selected by
the existing theme stylesheet. Images require nonempty alternative text.

All directives begin at column zero and end with a separate `:::` line. Nested
containers are allowed. Attributes are TOML `key=value` assignments separated by
whitespace, without spaces around `=`; quote string values containing spaces.
Fenced and indented code blocks are protected from directive interpretation.
Directive nesting and document-reference nesting are limited to 64 levels;
include and card-reference cycles are diagnosed.

### Links and assets

`[Another article](../posts/example.md#heading)` links to its generated page;
an include embeds a fragment instead. Links and image references resolve relative
to their original source, including inside nested includes. Markdown reference
definitions share a namespace across containers in the same document. Duplicate
definitions across separate regions currently use the last definition.

Headings receive lowercase Unicode-aware anchors, with punctuation converted to
separators; repeated headings receive `-2`, `-3`, and so forth. Generated pages
must have exactly one H1 and one main element, and all IDs must be unique.

`site.toml` mounts the existing `css/`, `js/`, `images/`, `docs/`, and `email.svg`
at identical output paths. Asset bytes are copied unchanged. Root-relative
published URLs are also supported and rebased for aliases. Local href/src targets
and HTML fragment anchors are checked after rendering. Asset-internal SVG/PDF/CSS
references and external URLs are not fetched or validated. Supported external
schemes are HTTP, HTTPS, mailto, and tel.

Raw HTML in Markdown is rejected. Use semantic components for structure; code
examples remain escaped. Tables, strikethrough, and task lists are supported.
Mathematical notation and code blocks retain the existing site's appearance;
this version does not introduce mathematical typesetting or syntax highlighting.

## Ownership and publication

`markdown.rs` parses syntax and source locations. `model.rs` defines owned typed
contracts. `resolve.rs` assigns routes and resolves references. `render/` owns
all structural HTML and interface SVG. `build.rs` validates and publishes bytes.
No parser or renderer mutates source documents.

Build publication follows `compile -> validate -> stage -> replace -> cleanup`.
Compilation or staging failures leave prior output intact. Replacement keeps the
previous directory as a backup until the new directory is installed; an install
failure attempts to restore it and reports its location if recovery fails.
There is a short rename window, so serve builds as static deployment artifacts
rather than replacing the directory of a live production server in place.
This is recoverable directory replacement, not a power-loss-durable transaction.

The compiler replaces only empty directories or directories bearing its
`.blog-compiler-output` marker. It refuses output paths overlapping sources,
assets, or compiler code. Successful rebuilds remove stale generated files.
Do not edit generated output or use a marked output directory for unrelated work.

## Verification

```sh
cargo fmt --manifest-path blog-compiler/Cargo.toml -- --check
cargo clippy --locked --manifest-path blog-compiler/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path blog-compiler/Cargo.toml
```

Regression tests cover aliases, cross-file references, protected code examples,
metadata and directive failures, cycles, route conflicts, escaping, curated
previews, reading times, determinism, filesystem boundaries, stale-file removal,
and failed-build preservation through the real CLI.

Production-site tests inspect the generated article sections, verbatim code,
figures, captions, and callouts directly. They also build an isolated tree of
declared inputs and check asset bytes; no test requires root/blog HTML snapshots.
