//! BBCode subset of the Godot UI (RichTextLabel) rendered to Bevy `TextSpan`s.
//!
//! Tags actually used by game/scripts/ui and the Codex: `[b] [i] [u] [s]`, `[color=#hex|name]`,
//! `[font_size=N]`, `[url=target]…[/url]` (or `[url]target[/url]`), `[img=WxH]path[/img]`
//! (`res://` stripped, relative to `game/assets`), `[center] [right]` (block alignment),
//! `[p]` (paragraph break), `[lb] [rb]` (escaped brackets). Unknown tags are kept as literal text.
//!
//! [`parse`] is pure and unit-tested; [`rich_text`] builds the Bevy bundle.

use bevy::ecs::relationship::RelatedSpawner;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{FontStyle, FontWeight, Strikethrough, Underline, UnderlineColor};

use super::UiTheme;
use super::style;

/// One run of uniformly styled text.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RichSpan {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub color: Option<Color>,
    pub size: Option<f32>,
    /// `[url=…]` target, if inside a link.
    pub link: Option<String>,
}

/// An inline image `[img=WxH]path[/img]`.
#[derive(Debug, Clone, PartialEq)]
pub struct RichImage {
    pub path: String,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub link: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RichItem {
    Span(RichSpan),
    Image(RichImage),
}

/// Block alignment given by a `[center]` / `[right]` tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RichAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RichDoc {
    pub items: Vec<RichItem>,
    pub align: RichAlign,
}

impl RichDoc {
    /// Plain text without markup (images skipped).
    pub fn plain(&self) -> String {
        self.items
            .iter()
            .filter_map(|item| match item {
                RichItem::Span(span) => Some(span.text.as_str()),
                RichItem::Image(_) => None,
            })
            .collect()
    }

    /// Link targets in order of appearance (deduplicated).
    pub fn links(&self) -> Vec<String> {
        let mut links: Vec<String> = Vec::new();
        for item in &self.items {
            let link = match item {
                RichItem::Span(span) => span.link.as_ref(),
                RichItem::Image(image) => image.link.as_ref(),
            };
            if let Some(link) = link
                && !links.contains(link)
            {
                links.push(link.clone());
            }
        }
        links
    }
}

/// Parses a colour: `#rgb`, `#rrggbb`, `#rrggbbaa` or a basic CSS name.
pub fn parse_color(text: &str) -> Option<Color> {
    let text = text.trim().trim_matches('"');
    if let Some(hex) = text.strip_prefix('#') {
        let value = |index: usize, width: usize| -> Option<f32> {
            let digits = hex.get(index..index + width)?;
            let raw = u8::from_str_radix(digits, 16).ok()?;
            Some(if width == 1 { raw * 17 } else { raw } as f32 / 255.0)
        };
        return match hex.len() {
            3 => Some(Color::srgb(value(0, 1)?, value(1, 1)?, value(2, 1)?)),
            6 => Some(Color::srgb(value(0, 2)?, value(2, 2)?, value(4, 2)?)),
            8 => Some(Color::srgba(
                value(0, 2)?,
                value(2, 2)?,
                value(4, 2)?,
                value(6, 2)?,
            )),
            _ => None,
        };
    }
    let named = match text.to_ascii_lowercase().as_str() {
        "red" => (0.78, 0.10, 0.10),
        "green" => (0.0, 0.50, 0.0),
        "blue" => (0.10, 0.20, 0.80),
        "white" => (1.0, 1.0, 1.0),
        "black" => (0.0, 0.0, 0.0),
        "yellow" => (0.90, 0.80, 0.10),
        "orange" => (0.90, 0.50, 0.10),
        "gray" | "grey" => (0.5, 0.5, 0.5),
        "gold" => (0.72, 0.56, 0.24),
        "purple" => (0.50, 0.15, 0.55),
        _ => return None,
    };
    Some(Color::srgb(named.0, named.1, named.2))
}

#[derive(Clone, Default)]
struct Format {
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
    color: Option<Color>,
    size: Option<f32>,
    link: Option<String>,
}

/// An open tag on the parser stack, with the format it replaced.
struct Open {
    name: String,
    previous: Format,
}

/// Parses `source` into a [`RichDoc`]. Never fails: malformed markup degrades to text.
pub fn parse(source: &str) -> RichDoc {
    let mut doc = RichDoc::default();
    let mut format = Format::default();
    let mut stack: Vec<Open> = Vec::new();
    let mut buffer = String::new();
    // Inside `[img]…[/img]`: (width, height) and the path being collected.
    let mut image: Option<(Option<f32>, Option<f32>, String)> = None;
    // Inside `[url]target[/url]` (no `=`): index of the first item of the link text.
    let mut bare_url_start: Option<usize> = None;

    fn flush(doc: &mut RichDoc, buffer: &mut String, format: &Format) {
        if buffer.is_empty() {
            return;
        }
        doc.items.push(RichItem::Span(RichSpan {
            text: std::mem::take(buffer),
            bold: format.bold,
            italic: format.italic,
            underline: format.underline,
            strike: format.strike,
            color: format.color,
            size: format.size,
            link: format.link.clone(),
        }));
    }

    let chars: Vec<char> = source.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        let current = chars[index];
        if current != '[' {
            if let Some((_, _, path)) = image.as_mut() {
                path.push(current);
            } else {
                buffer.push(current);
            }
            index += 1;
            continue;
        }
        // Find the closing bracket of a candidate tag (no nested `[`).
        let close = chars[index + 1..]
            .iter()
            .position(|c| *c == ']' || *c == '[')
            .filter(|offset| chars[index + 1 + offset] == ']')
            .map(|offset| index + 1 + offset);
        let Some(close) = close else {
            buffer.push('[');
            index += 1;
            continue;
        };
        let inside: String = chars[index + 1..close].iter().collect();
        let (is_close, body) = match inside.strip_prefix('/') {
            Some(rest) => (true, rest.to_owned()),
            None => (false, inside.clone()),
        };
        let (name, argument) = match body.split_once(['=', ' ']) {
            Some((name, argument)) => (name.to_owned(), Some(argument.trim().to_owned())),
            None => (body.clone(), None),
        };
        let known = matches!(
            name.as_str(),
            "b" | "i"
                | "u"
                | "s"
                | "color"
                | "font_size"
                | "url"
                | "img"
                | "center"
                | "right"
                | "p"
                | "lb"
                | "rb"
        );
        if !known {
            if let Some((_, _, path)) = image.as_mut() {
                path.push('[');
            } else {
                buffer.push('[');
            }
            index += 1;
            continue;
        }
        index = close + 1;
        // Inside an image only `[/img]` ends the path.
        if let Some((width, height, path)) = image.clone() {
            if is_close && name == "img" {
                flush(&mut doc, &mut buffer, &format);
                let path = path.trim().trim_start_matches("res://").to_owned();
                doc.items.push(RichItem::Image(RichImage {
                    path,
                    width,
                    height,
                    link: format.link.clone(),
                }));
                image = None;
            }
            continue;
        }
        match (is_close, name.as_str()) {
            (false, "lb") => buffer.push('['),
            (false, "rb") => buffer.push(']'),
            (false, "p") => {
                if !buffer.is_empty() || !doc.items.is_empty() {
                    buffer.push('\n');
                }
            }
            (false, "center") => doc.align = RichAlign::Center,
            (false, "right") => doc.align = RichAlign::Right,
            (true, "center" | "right" | "p" | "lb" | "rb") => {}
            (false, "img") => {
                let (width, height) = parse_image_size(argument.as_deref());
                flush(&mut doc, &mut buffer, &format);
                image = Some((width, height, String::new()));
            }
            (false, tag) => {
                flush(&mut doc, &mut buffer, &format);
                stack.push(Open {
                    name: tag.to_owned(),
                    previous: format.clone(),
                });
                match tag {
                    "b" => format.bold = true,
                    "i" => format.italic = true,
                    "u" => format.underline = true,
                    "s" => format.strike = true,
                    "color" => {
                        format.color = argument.as_deref().and_then(parse_color).or(format.color)
                    }
                    "font_size" => {
                        format.size = argument
                            .as_deref()
                            .and_then(|a| a.parse().ok())
                            .or(format.size)
                    }
                    "url" => {
                        format.underline = true;
                        match argument {
                            Some(target) => format.link = Some(target),
                            None => bare_url_start = Some(doc.items.len()),
                        }
                    }
                    _ => {}
                }
            }
            (true, tag) => {
                if let Some(position) = stack.iter().rposition(|open| open.name == tag) {
                    flush(&mut doc, &mut buffer, &format);
                    if tag == "url"
                        && let Some(start) = bare_url_start.take()
                    {
                        // `[url]target[/url]`: the visible text is also the link target.
                        let target: String = doc.items[start..]
                            .iter()
                            .filter_map(|item| match item {
                                RichItem::Span(span) => Some(span.text.as_str()),
                                RichItem::Image(_) => None,
                            })
                            .collect();
                        for item in &mut doc.items[start..] {
                            if let RichItem::Span(span) = item {
                                span.link = Some(target.clone());
                            }
                        }
                    }
                    format = stack[position].previous.clone();
                    stack.truncate(position);
                }
            }
        }
    }
    flush(&mut doc, &mut buffer, &format);
    doc
}

fn parse_image_size(argument: Option<&str>) -> (Option<f32>, Option<f32>) {
    let Some(argument) = argument else {
        return (None, None);
    };
    // `18x18`, `18`, or `width=18 height=18`.
    let argument = argument.trim();
    if let Some((w, h)) = argument.split_once('x')
        && let (Ok(w), Ok(h)) = (w.trim().parse(), h.trim().parse())
    {
        return (Some(w), Some(h));
    }
    match argument.parse::<f32>() {
        Ok(side) => (Some(side), Some(side)),
        Err(_) => (None, None),
    }
}

/// A `[url]` span or image: hover target for tooltip chains.
#[derive(Component, Debug, Clone)]
pub struct RichLink(pub String);

/// Root of a rich text block: the parsed source, for dump and tooltip inspection.
#[derive(Component, Debug, Clone)]
pub struct RichText {
    pub source: String,
    pub links: usize,
}

/// Colours and size of a rich text block.
#[derive(Clone, Debug)]
pub struct RichStyle {
    pub size: f32,
    pub color: Color,
    pub link_color: Color,
}

impl RichStyle {
    pub fn body(theme: &UiTheme) -> Self {
        Self {
            size: style::UiType::Body.size(),
            color: theme.text,
            link_color: style::RUBRIC,
        }
    }

    pub fn caption(theme: &UiTheme) -> Self {
        Self {
            size: style::UiType::Caption.size(),
            color: style::INK,
            ..Self::body(theme)
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

/// Text block rendering `source` (BBCode). The block is a `Text` with one child `TextSpan`
/// per run (and `InlineImage` children for `[img]`); link runs carry [`RichLink`] + `Hovered`.
pub fn rich_text(
    theme: &UiTheme,
    assets: &AssetServer,
    source: &str,
    style_of: &RichStyle,
) -> impl Bundle {
    let doc = parse(source);
    let justify = match doc.align {
        RichAlign::Left => Justify::Left,
        RichAlign::Center => Justify::Center,
        RichAlign::Right => Justify::Right,
    };
    let root = RichText {
        source: source.to_owned(),
        links: doc.links().len(),
    };
    let theme = theme.clone();
    let assets = assets.clone();
    let style_of = style_of.clone();
    (
        Name::new("RichText"),
        root,
        Text::new(""),
        theme.text_font(false, false, style_of.size),
        TextColor(style_of.color),
        TextLayout::justify(justify),
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            for item in doc.items {
                match item {
                    RichItem::Span(span) => {
                        let size = span.size.unwrap_or(style_of.size);
                        let linked = span.link.is_some();
                        let color = span.color.unwrap_or(if linked {
                            style_of.link_color
                        } else {
                            style_of.color
                        });
                        let mut font = theme.text_font(span.bold, span.italic, size);
                        if span.bold {
                            font.weight = FontWeight(700);
                        }
                        if span.italic {
                            font.style = FontStyle::Italic;
                        }
                        let mut entity =
                            parent.spawn((TextSpan::new(span.text), font, TextColor(color)));
                        if span.underline {
                            entity.insert((Underline, UnderlineColor(color)));
                        }
                        if span.strike {
                            entity.insert(Strikethrough);
                        }
                        if let Some(target) = span.link {
                            entity.insert((RichLink(target), Hovered::default()));
                        }
                    }
                    RichItem::Image(image) => {
                        let mut entity = parent.spawn(InlineImage {
                            image: assets.load(image.path),
                            width: image.width,
                            height: image.height.or(Some(style_of.size)),
                            ..default()
                        });
                        if let Some(target) = image.link {
                            entity.insert((RichLink(target), Hovered::default()));
                        }
                    }
                }
            }
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(doc: &RichDoc) -> Vec<&RichSpan> {
        doc.items
            .iter()
            .filter_map(|item| match item {
                RichItem::Span(span) => Some(span),
                RichItem::Image(_) => None,
            })
            .collect()
    }

    #[test]
    fn plain_text_is_one_span() {
        let doc = parse("Bonjour le monde");
        assert_eq!(spans(&doc).len(), 1);
        assert_eq!(doc.plain(), "Bonjour le monde");
    }

    #[test]
    fn bold_italic_nest_and_close() {
        let doc = parse("a [b]gras [i]et italique[/i][/b] fin");
        let spans = spans(&doc);
        assert_eq!(spans.len(), 4);
        assert!(!spans[0].bold);
        assert!(spans[1].bold && !spans[1].italic);
        assert!(spans[2].bold && spans[2].italic);
        assert!(!spans[3].bold && !spans[3].italic);
        assert_eq!(doc.plain(), "a gras et italique fin");
    }

    #[test]
    fn color_hex_and_name() {
        let doc = parse("[color=#8b1a1a]rouge[/color][color=green]vert[/color]x");
        let spans = spans(&doc);
        assert_eq!(
            spans[0].color,
            Some(Color::srgb(139.0 / 255.0, 26.0 / 255.0, 26.0 / 255.0))
        );
        assert!(spans[1].color.is_some());
        assert_eq!(spans[2].color, None);
        assert_eq!(parse_color("#fff"), Some(Color::srgb(1.0, 1.0, 1.0)));
        assert_eq!(parse_color("nonsense"), None);
    }

    #[test]
    fn url_links_are_underlined_and_listed() {
        let doc = parse(
            "voir [url=ib:rule:morale]moral[/url] et [url=cdx:foo]Foo[/url] puis [url=ib:rule:morale]encore[/url]",
        );
        let linked: Vec<_> = spans(&doc)
            .into_iter()
            .filter(|s| s.link.is_some())
            .collect();
        assert_eq!(linked.len(), 3);
        assert!(linked.iter().all(|s| s.underline));
        assert_eq!(doc.links(), vec!["ib:rule:morale", "cdx:foo"]);
    }

    #[test]
    fn bare_url_uses_its_text_as_target() {
        let doc = parse("[url]cdx:foo[/url]");
        assert_eq!(doc.links(), vec!["cdx:foo"]);
    }

    #[test]
    fn bold_link_like_the_codex_title() {
        let doc = parse("[b][url=cdx:archer]Archer[/url][/b]\nCorps");
        let first = spans(&doc)[0];
        assert!(first.bold);
        assert_eq!(first.link.as_deref(), Some("cdx:archer"));
        assert_eq!(doc.plain(), "Archer\nCorps");
    }

    #[test]
    fn image_with_size_and_res_prefix() {
        let doc = parse("x [img=18x18]res://assets/icons/gold.png[/img] y");
        let image = doc.items.iter().find_map(|item| match item {
            RichItem::Image(image) => Some(image),
            _ => None,
        });
        let image = image.expect("image");
        assert_eq!(image.path, "assets/icons/gold.png");
        assert_eq!((image.width, image.height), (Some(18.0), Some(18.0)));
        assert_eq!(doc.plain(), "x  y");
    }

    #[test]
    fn font_size_and_alignment() {
        let doc = parse("[center][font_size=30]Titre[/font_size][/center]");
        assert_eq!(doc.align, RichAlign::Center);
        assert_eq!(spans(&doc)[0].size, Some(30.0));
    }

    #[test]
    fn escapes_and_unknown_tags() {
        let doc = parse("[lb]note[rb] et [sic] et 5 [ non fermé");
        assert_eq!(doc.plain(), "[note] et [sic] et 5 [ non fermé");
    }

    #[test]
    fn unbalanced_close_is_ignored() {
        let doc = parse("a[/b]b[b]c");
        assert_eq!(doc.plain(), "abc");
        assert!(spans(&doc).last().unwrap().bold);
    }

    #[test]
    fn strike_and_underline() {
        let doc = parse("[s]barré[/s][u]souligné[/u]");
        let spans = spans(&doc);
        assert!(spans[0].strike && !spans[0].underline);
        assert!(spans[1].underline);
    }
}
