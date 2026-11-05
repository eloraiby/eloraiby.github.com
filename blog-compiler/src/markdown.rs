//! Parse front matter and semantic containers without interpreting examples as code.

use std::{collections::BTreeMap, path::Path};

use pulldown_cmark::{Event, Options, Parser, Tag};
use serde::{Deserialize, de::DeserializeOwned};

use crate::{Error, Result, model::*};

/// Markdown features shared by parsing, validation, word counts, and rendering.
/// Math remains literal text to preserve the current site's appearance.
pub fn options() -> Options {
    Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS
}

/// Parse an entire document and retain original line locations for every region.
pub fn parse(path: &Path, source: &str) -> Result<Document> {
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    if lines.first().map(|line| line.trim()) != Some("+++") {
        return Err(Error::at(
            path,
            1,
            "expected TOML front matter beginning with +++",
        ));
    }
    let end = lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, line)| line.trim() == "+++")
        .map(|(index, _)| index)
        .ok_or_else(|| Error::at(path, 1, "unclosed TOML front matter"))?;
    let header = lines[1..end].concat();
    let metadata = toml::from_str(&header).map_err(|error: toml::de::Error| {
        let line = error
            .span()
            .map(|range| {
                header[..range.start]
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count()
                    + 2
            })
            .unwrap_or(2);
        Error::at(path, line, error.message())
    })?;
    let body = lines[end + 1..].concat();
    let blocks = parse_blocks(path, &body, end + 2)?;
    Ok(Document {
        path: path.to_owned(),
        metadata,
        blocks,
    })
}

/// Parse nested directives only at column zero and outside Markdown code blocks.
pub fn parse_blocks(path: &Path, text: &str, first_line: usize) -> Result<Vec<Block>> {
    let code: Vec<_> = Parser::new_ext(text, options())
        .into_offset_iter()
        .filter_map(|(event, range)| {
            matches!(event, Event::Start(Tag::CodeBlock(_))).then_some(range)
        })
        .collect();
    let mut lines = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        lines.push((line, offset));
        offset += line.len();
    }
    let mut cursor = 0;
    let blocks = region(path, &lines, &code, &mut cursor, first_line, 0)?;
    Ok(blocks)
}

/// Consume one container body. The cursor always advances or returns an error.
fn region(
    path: &Path,
    lines: &[(&str, usize)],
    code: &[std::ops::Range<usize>],
    cursor: &mut usize,
    first_line: usize,
    depth: usize,
) -> Result<Vec<Block>> {
    if depth > 64 {
        return Err(Error::at(
            path,
            first_line + *cursor,
            "directive nesting exceeds 64 containers",
        ));
    }
    let mut blocks = Vec::new();
    let mut markdown = String::new();
    let mut markdown_line = first_line + *cursor;
    while *cursor < lines.len() {
        let (line, offset) = lines[*cursor];
        let special = line.starts_with(":::") && !code.iter().any(|range| range.contains(&offset));
        if !special {
            if markdown.is_empty() {
                markdown_line = first_line + *cursor;
            }
            markdown.push_str(line);
            *cursor += 1;
            continue;
        }
        flush(&mut blocks, &mut markdown, markdown_line);
        let number = first_line + *cursor;
        let value = line.trim_end();
        *cursor += 1;
        if value == ":::" {
            if depth > 0 {
                return Ok(blocks);
            }
            return Err(Error::at(
                path,
                number,
                "unexpected directive closing fence",
            ));
        }
        let directive = parse_directive(path, number, &value[3..])?;
        let body = region(path, lines, code, cursor, first_line, depth + 1)?;
        if matches!(directive, Directive::Include { .. } | Directive::Timeline) && !body.is_empty()
        {
            return Err(Error::at(
                path,
                number,
                "this directive requires an empty body",
            ));
        }
        blocks.push(Block {
            line: number,
            content: BlockContent::Component { directive, body },
        });
    }
    if depth > 0 {
        return Err(Error::at(
            path,
            first_line + lines.len(),
            "unclosed directive",
        ));
    }
    flush(&mut blocks, &mut markdown, markdown_line);
    Ok(blocks)
}

/// Preserve nonempty Markdown regions without losing their source location.
fn flush(blocks: &mut Vec<Block>, markdown: &mut String, line: usize) {
    if !markdown.trim().is_empty() {
        blocks.push(Block {
            line,
            content: BlockContent::Markdown(std::mem::take(markdown)),
        });
    } else {
        markdown.clear();
    }
}

/// Decode whitespace-separated TOML assignments while respecting quoted values.
fn attributes(path: &Path, line: usize, text: &str) -> Result<toml::Table> {
    let mut normalized = String::new();
    let mut quote = None;
    let mut escaped = false;
    for character in text.trim().chars() {
        if let Some(delimiter) = quote {
            normalized.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' && delimiter == '"' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
        } else if character == '\'' || character == '"' {
            quote = Some(character);
            normalized.push(character);
        } else if character.is_whitespace() {
            normalized.push('\n');
        } else {
            normalized.push(character);
        }
    }
    normalized
        .parse()
        .map_err(|error: toml::de::Error| Error::at(path, line, error.message()))
}

/// Parse a closed attribute schema; typos are diagnostics rather than ignored data.
fn decode<T: DeserializeOwned>(path: &Path, line: usize, table: toml::Table) -> Result<T> {
    table
        .try_into()
        .map_err(|error: toml::de::Error| Error::at(path, line, error.message()))
}

/// Turn a directive name and typed attributes into a semantic syntax node.
fn parse_directive(path: &Path, line: usize, text: &str) -> Result<Directive> {
    /// Include attributes keep source ownership distinct from rendering choice.
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Include {
        /// Referenced fragment path.
        src: String,
        /// Closed renderer selection.
        view: View,
    }
    /// Card collections select a concrete record kind.
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Cards {
        /// Expected metadata discriminant.
        kind: CardKind,
    }
    /// Post collections have view-specific, fixed ordering semantics.
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Posts {
        /// Listing or curated previews.
        view: PostView,
        /// Optional small label above the preview section heading.
        eyebrow: Option<String>,
    }
    /// Figures carry asset references and mandatory accessible alternative text.
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Figure {
        /// Default image.
        src: String,
        /// Accessible description.
        alt: String,
        /// Optional dark-theme variant.
        dark: Option<String>,
    }
    let (name, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
    let table = attributes(path, line, rest)?;
    Ok(match name {
        "include" => {
            let value: Include = decode(path, line, table)?;
            Directive::Include {
                src: value.src,
                view: value.view,
            }
        }
        "cards" => {
            let value: Cards = decode(path, line, table)?;
            Directive::Cards { kind: value.kind }
        }
        "posts" => {
            let value: Posts = decode(path, line, table)?;
            Directive::Posts {
                view: value.view,
                eyebrow: value.eyebrow,
            }
        }
        "figure" => {
            let value: Figure = decode(path, line, table)?;
            Directive::Figure {
                src: value.src,
                alt: value.alt,
                dark: value.dark,
            }
        }
        "callout" | "about-card" | "timeline" | "guide" => {
            if !table.is_empty() {
                return Err(Error::at(
                    path,
                    line,
                    "this directive accepts no attributes",
                ));
            }
            match name {
                "callout" => Directive::Callout,
                "about-card" => Directive::AboutCard,
                "timeline" => Directive::Timeline,
                _ => Directive::Guide,
            }
        }
        _ => return Err(Error::at(path, line, format!("unknown directive {name:?}"))),
    })
}

/// Collect source-level link definitions for use across directive boundaries.
pub fn definitions(document: &Document) -> BTreeMap<String, (String, String)> {
    /// Walk all Markdown regions, preserving one shared definition namespace.
    fn collect(blocks: &[Block], result: &mut BTreeMap<String, (String, String)>) {
        for block in blocks {
            match &block.content {
                BlockContent::Markdown(text) => {
                    let parser = Parser::new_ext(text, options());
                    for (label, definition) in parser.reference_definitions().iter() {
                        result.insert(
                            label.to_lowercase(),
                            (
                                definition.dest.to_string(),
                                definition
                                    .title
                                    .as_ref()
                                    .map(ToString::to_string)
                                    .unwrap_or_default(),
                            ),
                        );
                    }
                }
                BlockContent::Component { body, .. } => collect(body, result),
            }
        }
    }
    let mut result = BTreeMap::new();
    collect(&document.blocks, &mut result);
    result
}
