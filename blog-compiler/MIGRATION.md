# Markdown migration

The migration uses the current working-tree website as its visual/content
baseline, including the user's existing contact and biography edits. Existing
HTML edits remain outside the implementation commits. Historical root/blog HTML
is retained as a reference, with Markdown now authoritative for generated output.

## Resulting ownership

- `content/index.md` composes seven homepage sections from fragments and a curated
  article collection.
- Twelve project records and six publication records preserve their authored
  ordering, descriptions, actions, and assets.
- Four article sources preserve body text, quotations, figures, captions, code,
  dates, visible tags, and reading-time overrides.
- `content/blog.md` produces `/blog.html` and `/blog/index.html` with links rebased
  for each route.
- `content/article-template.md` produces the existing `/blog/template.html` URL as
  a Markdown authoring example.
- `site.toml` owns identity, navigation, footer variants, topic filters, and asset
  mounts. The compiler reads no legacy HTML or `.body` files.
- Rust owns layout, component markup, interface labels, and SVG geometry. Existing
  CSS, JavaScript, screenshots, figures, PDFs, and email artwork remain assets.

The index's publishing note is intentionally updated to describe Markdown and
the compiler. Its text and authoring-example page supersede instructions to copy
HTML templates and insert listing cards manually. Other content and presentation
are preserved. Mathematical notation remains literal and code remains unhighlighted.

## Verification performed

The complete build contains eight HTML pages and 22 unchanged asset files.
The compiler checks all emitted local links and HTML anchors. Production
integration tests verify homepage composition, curated preview ordering,
article-body structure/text/code, and byte-identical assets. A separate isolated
build containing only Markdown, configuration, and declared assets proves the
compiler does not depend on historical HTML.

Chromium 156 comparisons covered the homepage, both index URLs, and all four
articles at widths 1440 and 390, in light and dark modes: 28 page/width/theme
combinations. CSS transitions were disabled in both comparison copies to avoid
capturing intermediate colors. Full-page screenshots matched pixel for pixel for
the homepage and four articles in all 20 corresponding combinations. The index
differences are the updated publishing note; on mobile the shorter note reduces
page height by 22 pixels. No generated page had broken images or horizontal
overflow in these checks.

Browser interaction checks exercised theme toggling and persistence after reload,
Rust-topic filtering, mobile-menu opening, aria-expanded state, and closing on an
outside click. All passed. These local checks do not establish remote deployment:
the Pages workflow must run in a repository configured to publish via GitHub Actions.

The compiler suite includes 22 isolated contract regressions and three production
migration regressions. Formatting and Clippy with warnings denied are also checked
by the Pages workflow before building the deployment artifact.
