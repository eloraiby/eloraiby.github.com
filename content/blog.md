+++
kind = "page"
layout = "blog"
route = "/blog.html"
aliases = ["/blog/index.html"]
title = "Blog & Technical Notes - Wael El Oraiby"
description = "Technical articles on computational geometry, systems architecture, high-performance computing, Rust, C/C++, and real-time rendering by Wael El Oraiby."
eyebrow = "Writings & Insights"
+++

# Blog & Technical Notes

Essays on algorithms, low-level systems programming in Rust and C/C++, graphics pipelines, and computational geometry.

:::posts view="list"
:::

:::guide
### Blog Compiler & Publishing Guide

Write an article in Markdown under `content/posts/`, then run `cargo run --manifest-path blog-compiler/Cargo.toml -- build`. The compiler updates the article listing automatically. See the [authoring example](article-template.md).
:::
