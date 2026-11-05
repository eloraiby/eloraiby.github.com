//! Small command-line boundary; all compiler behavior is also available as a library.

use std::path::PathBuf;

use blog_compiler::{Error, Result, Site};

/// Parse commands explicitly so invalid flags never silently change publication.
fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".into());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        println!(
            "blog-compiler <check|build> [--root SITE_ROOT] [--out OUTPUT]\nDefault root: compiler's parent; default output: SITE_ROOT/dist"
        );
        return Ok(());
    }
    if !matches!(command.as_str(), "check" | "build") {
        return Err(Error(format!("unknown command {command:?}")));
    }
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned();
    let mut output = None;
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--root" => {
                root = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or_else(|| Error("--root requires a path".into()))?
            }
            "--out" if command == "build" => {
                output = Some(
                    args.next()
                        .map(PathBuf::from)
                        .ok_or_else(|| Error("--out requires a path".into()))?,
                )
            }
            _ => return Err(Error(format!("unsupported option {flag:?} for {command}"))),
        }
    }
    let site = Site::load(&root)?;
    let build = site.compile()?;
    if command == "build" {
        let output = output.unwrap_or_else(|| PathBuf::from("dist"));
        let output = if output.is_absolute() {
            output
        } else {
            root.join(output)
        };
        build.write(&output)?;
        println!(
            "Built {} files into {}",
            build.files().len(),
            output.display()
        );
    } else {
        println!(
            "Checked {} output files; all local links and anchors resolve",
            build.files().len()
        );
    }
    Ok(())
}

/// Exit nonzero on any parse, resolution, rendering, or publication error.
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
