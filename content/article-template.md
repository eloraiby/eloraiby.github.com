+++
kind = "page"
layout = "blog"
route = "/blog/template.html"
title = "Markdown Authoring Example - Wael El Oraiby"
description = "An example article source for the Rust blog compiler."
+++

# Markdown Authoring Example

Create `content/posts/YYYY-MM-DD-your-topic.md` with metadata and Markdown:

````markdown
+++
kind = "post"
title = "Your title"
description = "Search summary"
date = 2026-11-05
excerpt = "A short listing summary."
tags = ["Rust"]
filter_tags = ["rust"]
+++

Your introduction.

## First section

:::callout
**Takeaway:** A styled Markdown callout.
:::

```rust
fn main() { println!("Hello!"); }
```
````

[Back to the blog](blog.md).
