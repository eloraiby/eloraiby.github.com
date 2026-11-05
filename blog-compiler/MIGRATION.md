# Markdown migration

The initial migration used the working-tree website as its visual/content
baseline, including the user's contact and biography edits. Those edits are
represented in the Markdown content and Rust-generated layout. After verifying
the generated output, redundant root/blog HTML and obsolete `.body` fragments
were removed. Markdown is authoritative, and generated pages live in `dist/`.

## Resulting ownership

- `content/index.md` composes seven homepage sections from fragments and a curated
  article collection.
- Fifteen project records and six publication records preserve their authored
  ordering, descriptions, actions, and assets. The three NeoCogi project cards
  precede the original twelve records.
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

The initial migration build contained eight HTML pages and 22 unchanged asset files.
The compiler checks all emitted local links and HTML anchors. Production
integration tests verify homepage composition, curated preview ordering,
article sections, verbatim code, figures, captions, callouts, quotations, and
byte-identical assets. A separate isolated
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

The compiler suite includes 24 isolated contract regressions and four production
migration regressions. Formatting and Clippy with warnings denied are also checked
by the Pages workflow before building the deployment artifact.

Timeline coverage follows authored milestone titles and roles rather than the
original migration's fixed entry count. Adding, removing, or reordering entries
is normal authoring work, with separate regression coverage for those operations.

## Removal of generated source copies

The eight historical HTML files and three unused `.body` fragments are no longer
tracked sources. Production tests inspect generated output directly instead of
reading historical article HTML; no replacement HTML snapshots are stored as
fixtures. The isolated-input test continues to compile using only configuration,
Markdown, and declared assets.

All 30 generated files were compared byte for byte before and after the removal.
The output, including every HTML page and asset, is unchanged. The original
contact/biography edits remain in the authoritative sources. Old published paths
are generated under `dist/`; they do not require source-tree HTML files. Git
ignores the retired root/index and root/blog output locations to prevent accidental
reintroduction of duplicate pages. The build and Pages deployment commands are
unchanged.

## Profile formatting follow-up

The authored profile now includes Gameloft, bringing both highlights and milestones
to six entries, along with the owner's corrected descriptions of the other roles. The highlights
layout uses three desktop columns, two tablet columns, and one mobile column;
its responsive rules live in CSS rather than an inline grid declaration.
Gameloft copy and the Technical Focus introduction were corrected for spelling,
capitalization, and grammar while retaining the supplied career information.
Timeline tests follow authored records and no longer enforce the initial five
milestones.

## NeoCogi project previews

The project section now begins with `neocogi-ui`, `neocogi-platform`, and
`neocogi-com`, in that order, with short descriptions and linked full-size
screenshots. UI and Platform use their repository demo captures. COM uses a
capture of its real calculator-plugin example output; it is labeled as a
walkthrough rather than an application UI. Sources, revisions, and image/artwork
credits are retained in [the image notes](../images/README.md).

These records use `kind = "project"`; no posts, article routes, or blog-list entries
are added. Project-list regression checks follow authored record ordering, and
dedicated coverage verifies the requested three cards and their screenshot links.
