# Portfolio and technical blog

The site is authored in [Markdown](content/index.md) and compiled by the Rust
[blog compiler](blog-compiler/README.md). All compiler code and Cargo files live
under `blog-compiler/`.

```sh
cargo run --locked --manifest-path blog-compiler/Cargo.toml -- check
cargo run --locked --manifest-path blog-compiler/Cargo.toml -- build
python3 -m http.server --directory dist 8000
```

Open `http://localhost:8000` to preview. The output is ordinary static HTML, CSS,
JavaScript, SVG, images, and PDFs.

- Edit `content/index.md` to compose the homepage from Markdown fragments.
- Edit `content/blog.md` for the blog index; both existing index URLs are generated.
- Add articles under `content/posts/`; the blog listing updates automatically.
- Edit project and publication records under `content/projects/` and
  `content/publications/`.
- Edit `site.toml` for identity, navigation, social links, filters, and asset mounts.

The current asset directories are the single source for CSS, JavaScript, images,
and documents. Root/blog HTML files and old `.body` fragments are historical
snapshots, retained to preserve existing local edits and migration references;
the compiler does not read them. Author new content in Markdown and deploy `dist/`.
The old copy-an-HTML-template publishing workflow is superseded.

The [Pages workflow](.github/workflows/pages.yml) checks and compiles the site,
uploads `dist/`, and deploys pushes to `master` or manual runs. In repository
Settings → Pages, select **GitHub Actions** as the build source. Local work does
not change hosting settings or deploy anything.

See the [compiler documentation](blog-compiler/README.md) for authoring examples,
metadata schemas, directives, reference resolution, validation, and publication
failure behavior.
