//! The note renderer: the blocks notes actually use, as a list the reader draws.
//!
//! ponytail: no reference links, no setext headings, no inline HTML — a note is text, not
//! markup, and `<b>` in a note is two angle brackets and a letter. Reach for a parser if
//! notes outgrow this.
//!
//! Nothing here produces markup of any kind, so there is nothing to escape and no way for
//! a note to reach past the panel it is drawn in. A link is either an `http(s)` URL or a
//! path that has to resolve to a note in this vault; there is no third kind.

use std::collections::HashMap;

/// The markers the renderer draws for list items. Named so the window's glyph test can
/// check them against the fonts that have to draw them.
pub const TASK_DONE: &str = "\u{2611}";
pub const TASK_TODO: &str = "\u{2610}";
pub const BULLET: &str = "\u{2022}";

struct Footnote {
    n: usize,
    text: String,
}

/// Where a link goes. Only `http`/`https` ever becomes a [`Target::Url`], so anything
/// else a note writes — `javascript:`, `file:`, a bare path — is looked up as a note and
/// greys out when there is none.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    /// Another note. `relative` is a markdown link, resolved against the note holding it;
    /// a wikilink names its target outright.
    Note {
        path: String,
        relative: bool,
    },
    Url(String),
}

/// One run of text and everything true about it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
    pub link: Option<Target>,
    /// A footnote reference carries its own text, so the note reads without a jump.
    pub hover: Option<String>,
}

impl Span {
    fn plain(text: impl Into<String>) -> Span {
        Span {
            text: text.into(),
            ..Span::default()
        }
    }
}

/// How a table column is set.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
}

/// A note, as the reader lays it out. Lists are flat: `indent` is the depth, so there is
/// no tree to walk and no way for one to be left open.
#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Heading(usize, Vec<Span>),
    Para(Vec<Span>),
    Quote(Vec<Span>),
    Code {
        lang: String,
        text: String,
    },
    Item {
        indent: usize,
        marker: String,
        body: Vec<Span>,
    },
    Rule,
    Table {
        align: Vec<Align>,
        head: Vec<Vec<Span>>,
        rows: Vec<Vec<Vec<Span>>>,
    },
}

/// One open list, and where its numbering has got to.
struct List {
    indent: usize,
    ordered: bool,
    n: usize,
}

pub fn render(source: &str) -> Vec<Block> {
    Parser::new(source).run()
}

/// A note being read, one line at a time.
///
/// The blocks being built, the paragraph being gathered, the lists that are open and
/// whether a quote is running were once six locals passed between five functions. They
/// are fields, and the functions are the methods that move them.
struct Parser {
    lines: Vec<String>,
    footnotes: HashMap<String, Footnote>,
    /// The definitions in the order they were written, which is the order they are
    /// numbered and listed in.
    defs: Vec<(String, String)>,
    out: Vec<Block>,
    lists: Vec<List>,
    /// The paragraph being gathered. Consecutive lines are one paragraph, the way
    /// markdown means them.
    para: Vec<Span>,
    quote: bool,
    at: usize,
}

impl Parser {
    fn new(source: &str) -> Parser {
        let body = strip_comments(&strip_frontmatter(source));
        let lines: Vec<String> = body
            .split('\n')
            .map(|l| l.trim_end().replace('\t', "  "))
            .collect();
        // Footnote definitions are collected first so a reference can carry its text,
        // whichever order they appear in.
        let mut footnotes = HashMap::new();
        let mut defs: Vec<(String, String)> = Vec::new();
        for line in &lines {
            if let Some((id, text)) = footnote_def(line) {
                footnotes.insert(
                    id.clone(),
                    Footnote {
                        n: defs.len() + 1,
                        text: text.clone(),
                    },
                );
                defs.push((id, text));
            }
        }
        Parser {
            lines,
            footnotes,
            defs,
            out: Vec::new(),
            lists: Vec::new(),
            para: Vec::new(),
            quote: false,
            at: 0,
        }
    }

    fn run(mut self) -> Vec<Block> {
        while self.at < self.lines.len() {
            let line = self.lines[self.at].clone();
            if let Some(lang) = fence_open(&line) {
                self.close();
                self.fenced(lang);
                continue;
            }
            // A blank line ends a quote and a paragraph, but not a list: a loose list
            // keeps going.
            if line.trim().is_empty() {
                self.flush();
                self.quote = false;
            } else if footnote_def(&line).is_some() {
                // Collected already, and not shown where it was written.
            } else if self.table_here() {
                self.close();
                self.table();
                continue;
            } else if let Some((level, text)) = heading(&line) {
                self.close();
                let spans = self.inline(&text);
                self.out.push(Block::Heading(level, spans));
            } else if is_break(line.trim()) {
                self.close();
                self.out.push(Block::Rule);
            } else if let Some(item) = list_item(&line) {
                self.item(item);
            } else if let Some(text) = quoted(&line) {
                self.quoted_line(&text);
            } else if self.continues_an_item(&line) {
                self.continue_item(&line);
            } else {
                if !self.quote {
                    self.lists.clear();
                }
                let spans = self.inline(&line);
                self.join(spans);
            }
            self.at += 1;
        }
        self.close();
        self.footnote_list();
        self.out
    }

    /// A fenced block runs to its closing fence, or to the end of the note — an unclosed
    /// fence still shows what it holds rather than swallowing it.
    fn fenced(&mut self, lang: String) {
        self.at += 1;
        let mut held = String::new();
        while self.at < self.lines.len() {
            let line = &self.lines[self.at];
            if line.trim_start().starts_with("```") {
                self.at += 1;
                self.out.push(Block::Code { lang, text: held });
                return;
            }
            if !held.is_empty() {
                held.push('\n');
            }
            held.push_str(line);
            self.at += 1;
        }
        self.out.push(Block::Code { lang, text: held });
    }

    /// A table is a row whose next line is the ---|--- rule under it.
    fn table_here(&self) -> bool {
        let under = self.lines.get(self.at + 1).map_or("", |l| l.as_str());
        self.lines[self.at].contains('|') && is_rule(under)
    }

    fn table(&mut self) {
        let head: Vec<Vec<Span>> = cells(&self.lines[self.at].clone())
            .iter()
            .map(|h| self.inline(h))
            .collect();
        let align: Vec<Align> = cells(&self.lines[self.at + 1])
            .iter()
            .map(|d| align_of(d))
            .collect();
        self.at += 2;
        let mut rows: Vec<Vec<Vec<Span>>> = Vec::new();
        while self.at < self.lines.len()
            && self.lines[self.at].contains('|')
            && !self.lines[self.at].trim().is_empty()
        {
            let values = cells(&self.lines[self.at]);
            rows.push(
                (0..head.len())
                    .map(|j| self.inline(values.get(j).map_or("", |v| v.as_str())))
                    .collect(),
            );
            self.at += 1;
        }
        self.out.push(Block::Table { align, head, rows });
    }

    /// One list line. A shallower item closes everything opened deeper than it; a deeper
    /// one opens a list of its own; the same indent in a different kind starts again.
    fn item(&mut self, (indent, marker, text): (usize, String, String)) {
        let ordered = marker.starts_with(|c: char| c.is_ascii_digit());
        self.flush();
        self.quote = false;
        while self.lists.last().is_some_and(|l| l.indent > indent) {
            self.lists.pop();
        }
        match self.lists.last_mut() {
            Some(list) if list.indent == indent && list.ordered == ordered => list.n += 1,
            Some(list) if list.indent == indent => {
                *list = List {
                    indent,
                    ordered,
                    n: 1,
                }
            }
            _ => self.lists.push(List {
                indent,
                ordered,
                n: 1,
            }),
        }
        let depth = self.lists.len() - 1;
        let n = self.lists.last().expect("pushed above").n;
        let (marker, body) = match task_item(&text) {
            Some((done, rest)) => (
                match done {
                    true => TASK_DONE.to_string(),
                    false => TASK_TODO.to_string(),
                },
                self.inline(&rest),
            ),
            // An ordered list is numbered from where it started, not from what the note
            // happens to have typed on each line.
            None => match ordered {
                true => (format!("{n}."), self.inline(&text)),
                false => (BULLET.to_string(), self.inline(&text)),
            },
        };
        self.out.push(Block::Item {
            indent: depth,
            marker,
            body,
        });
    }

    fn quoted_line(&mut self, text: &str) {
        if !self.quote {
            self.flush();
            self.lists.clear();
            self.quote = true;
        }
        let spans = self.inline(text);
        self.join(spans);
    }

    /// A wrapped line under a list item belongs to the item above it.
    fn continues_an_item(&self, line: &str) -> bool {
        !self.lists.is_empty()
            && line.starts_with(char::is_whitespace)
            && matches!(self.out.last(), Some(Block::Item { .. }))
    }

    fn continue_item(&mut self, line: &str) {
        let addition = self.inline(line.trim());
        if let Some(Block::Item { body, .. }) = self.out.last_mut() {
            body.push(Span::plain(" "));
            body.extend(addition);
        }
    }

    fn join(&mut self, line: Vec<Span>) {
        if !self.para.is_empty() {
            self.para.push(Span::plain(" "));
        }
        self.para.extend(line);
    }

    /// The gathered paragraph becomes a block — a quote when one is running.
    fn flush(&mut self) {
        if self.para.is_empty() {
            return;
        }
        let spans = std::mem::take(&mut self.para);
        self.out.push(match self.quote {
            true => Block::Quote(spans),
            false => Block::Para(spans),
        });
    }

    /// Everything open, closed: a block that ends a paragraph ends a quote and every list
    /// with it.
    fn close(&mut self) {
        self.flush();
        self.lists.clear();
        self.quote = false;
    }

    /// The definitions, listed under a rule at the foot of the note.
    fn footnote_list(&mut self) {
        if self.defs.is_empty() {
            return;
        }
        self.out.push(Block::Rule);
        for (n, (_, text)) in self.defs.clone().iter().enumerate() {
            let body = self.inline(text);
            self.out.push(Block::Item {
                indent: 0,
                marker: format!("{}.", n + 1),
                body,
            });
        }
    }

    fn inline(&self, text: &str) -> Vec<Span> {
        Inline::scan(text, &self.footnotes)
    }
}

fn strip_frontmatter(source: &str) -> String {
    let Some(rest) = source
        .strip_prefix("---\n")
        .or_else(|| source.strip_prefix("---\r\n"))
    else {
        return source.to_string();
    };
    let mut at = 0;
    for line in rest.split_inclusive('\n') {
        at += line.len();
        if line.trim_end() == "---" {
            return rest[at..].to_string();
        }
    }
    source.to_string()
}

fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        rest = match rest[start..].find("-->") {
            Some(end) => &rest[start + end + 3..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

fn footnote_def(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("[^")?;
    let close = rest.find(']')?;
    let text = rest[close + 1..].strip_prefix(':')?;
    Some((rest[..close].to_string(), text.trim_start().to_string()))
}

fn fence_open(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("```")?;
    let lang: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '+' || *c == '-')
        .collect();
    Some(lang)
}

fn heading(line: &str) -> Option<(usize, String)> {
    let level = line.chars().take_while(|c| *c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &line[level..];
    match rest.starts_with([' ', '\t']) {
        true => Some((level, rest.trim_start().to_string())),
        false => None,
    }
}

fn is_break(line: &str) -> bool {
    let repeated = |c: char| line.len() >= 3 && line.chars().all(|x| x == c);
    repeated('-') || repeated('*') || repeated('_')
}

/// `(indent, marker, text)` for a list line.
fn list_item(line: &str) -> Option<(usize, String, String)> {
    let indent = line.len() - line.trim_start().len();
    let rest = line.trim_start();
    let bullet = rest.starts_with(['-', '*', '+']) && rest[1..].starts_with(' ');
    if bullet {
        return Some((indent, rest[..1].to_string(), rest[2..].to_string()));
    }
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let after = &rest[digits..];
    let dotted = after.starts_with(['.', ')']) && after[1..].starts_with(' ');
    match dotted {
        true => Some((
            indent,
            rest[..digits + 1].to_string(),
            after[2..].to_string(),
        )),
        false => None,
    }
}

fn task_item(text: &str) -> Option<(bool, String)> {
    let rest = text.strip_prefix('[')?;
    let mark = rest.chars().next()?;
    let rest = rest[mark.len_utf8()..].strip_prefix("] ")?;
    match mark {
        ' ' => Some((false, rest.to_string())),
        'x' | 'X' => Some((true, rest.to_string())),
        _ => None,
    }
}

fn quoted(line: &str) -> Option<String> {
    let rest = line.strip_prefix('>')?;
    Some(rest.strip_prefix(' ').unwrap_or(rest).to_string())
}

fn is_rule(line: &str) -> bool {
    line.contains('-')
        && !line.trim().is_empty()
        && line
            .chars()
            .all(|c| matches!(c, '-' | ':' | '|' | ' ' | '\t'))
}

fn cells(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let inner = trimmed
        .strip_prefix('|')
        .unwrap_or(trimmed)
        .strip_suffix('|')
        .unwrap_or_else(|| trimmed.strip_prefix('|').unwrap_or(trimmed));
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

fn align_of(divider: &str) -> Align {
    match (divider.starts_with(':'), divider.ends_with(':')) {
        (true, true) => Align::Center,
        (false, true) => Align::End,
        _ => Align::Start,
    }
}

/// One pass over a line of text. Each rule consumes what it matched and says so; what no
/// rule matched is a character of the plain run being built.
struct Inline<'a> {
    chars: Vec<char>,
    footnotes: &'a HashMap<String, Footnote>,
    out: Vec<Span>,
    /// What has been typed since the last span closed.
    run: String,
    at: usize,
}

impl<'a> Inline<'a> {
    fn scan(text: &str, footnotes: &'a HashMap<String, Footnote>) -> Vec<Span> {
        let mut scanner = Inline {
            chars: text.chars().collect(),
            footnotes,
            out: Vec::new(),
            run: String::new(),
            at: 0,
        };
        while scanner.at < scanner.chars.len() {
            let matched = scanner.code()
                || scanner.wikilink()
                || scanner.image()
                || scanner.bracketed()
                || scanner.autolink()
                || scanner.emphasis();
            if !matched {
                scanner.run.push(scanner.chars[scanner.at]);
                scanner.at += 1;
            }
        }
        scanner.settle();
        scanner.out
    }

    /// The plain run so far becomes a span of its own.
    fn settle(&mut self) {
        if !self.run.is_empty() {
            self.out.push(Span::plain(std::mem::take(&mut self.run)));
        }
    }

    fn emit(&mut self, spans: impl IntoIterator<Item = Span>, next: usize) -> bool {
        self.settle();
        self.out.extend(spans);
        self.at = next;
        true
    }

    /// Code spans are never rewritten from the inside.
    fn code(&mut self) -> bool {
        if self.chars[self.at] != '`' {
            return false;
        }
        let Some(end) = find(&self.chars, self.at + 1, '`') else {
            return false;
        };
        let text = take(&self.chars, self.at + 1, end);
        self.emit(
            [Span {
                text,
                code: true,
                ..Span::default()
            }],
            end + 1,
        )
    }

    fn wikilink(&mut self) -> bool {
        let bang = starts(&self.chars, self.at, "![[");
        if !bang && !starts(&self.chars, self.at, "[[") {
            return false;
        }
        let from = self.at + if bang { 3 } else { 2 };
        let Some(end) = find_str(&self.chars, from, "]]") else {
            return false;
        };
        let inner = take(&self.chars, from, end);
        let (target, alias) = match inner.split_once('|') {
            Some((target, alias)) => (target.to_string(), alias.to_string()),
            None => (inner.clone(), inner.clone()),
        };
        let shown = alias.split('#').next().unwrap_or("").to_string();
        let label = Inline::scan(&shown, self.footnotes);
        let target = Target::Note {
            path: target,
            relative: false,
        };
        self.emit(linked(label, target), end + 2)
    }

    /// ponytail: an image is drawn as a link to itself, labelled with its alt text.
    /// Fetching and decoding one is a dependency and a cache; add them if notes ask.
    fn image(&mut self) -> bool {
        if self.chars[self.at] != '!' || self.chars.get(self.at + 1) != Some(&'[') {
            return false;
        }
        let Some((alt, target, next)) = link_at(&self.chars, self.at + 1) else {
            return false;
        };
        if !is_external(&target) {
            return false;
        }
        self.emit(linked(vec![Span::plain(alt)], Target::Url(target)), next)
    }

    /// `[` opens either a footnote reference or a link.
    fn bracketed(&mut self) -> bool {
        if self.chars[self.at] != '[' {
            return false;
        }
        // A footnote reference carries its own text, so the note reads without a jump.
        if self.chars.get(self.at + 1) == Some(&'^') {
            if let Some(end) = find(&self.chars, self.at + 2, ']') {
                let id = take(&self.chars, self.at + 2, end);
                if let Some(note) = self.footnotes.get(&id) {
                    let span = Span {
                        text: format!("[{}]", note.n),
                        hover: Some(note.text.clone()),
                        ..Span::default()
                    };
                    return self.emit([span], end + 1);
                }
            }
        }
        let Some((label, target, next)) = link_at(&self.chars, self.at) else {
            return false;
        };
        let inner = Inline::scan(&label, self.footnotes);
        let target = match is_external(&target) {
            true => Target::Url(target),
            false => Target::Note {
                path: target,
                relative: true,
            },
        };
        self.emit(linked(inner, target), next)
    }

    fn autolink(&mut self) -> bool {
        if self.chars[self.at] != '<' {
            return false;
        }
        let Some(end) = find(&self.chars, self.at + 1, '>') else {
            return false;
        };
        let url = take(&self.chars, self.at + 1, end);
        if !is_external(&url) || url.contains(char::is_whitespace) {
            return false;
        }
        self.emit(
            linked(vec![Span::plain(url.clone())], Target::Url(url)),
            end + 1,
        )
    }

    fn emphasis(&mut self) -> bool {
        let c = self.chars[self.at];
        let Some((wrap, style)) = (match c {
            '~' if starts(&self.chars, self.at, "~~") => Some(("~~", Style::Strike)),
            '*' if starts(&self.chars, self.at, "**") => Some(("**", Style::Bold)),
            '*' => Some(("*", Style::Italic)),
            _ => None,
        }) else {
            return false;
        };
        let from = self.at + wrap.len();
        let Some(end) = find_str(&self.chars, from, wrap) else {
            return false;
        };
        let inner = take(&self.chars, from, end);
        // The same rule the page always had: emphasis holds no star of its own.
        if inner.is_empty() || inner.contains(wrap.chars().next().expect("wrap")) {
            return false;
        }
        let spans = Inline::scan(&inner, self.footnotes)
            .into_iter()
            .map(|mut span| {
                match style {
                    Style::Bold => span.bold = true,
                    Style::Italic => span.italic = true,
                    Style::Strike => span.strike = true,
                }
                span
            });
        let spans: Vec<Span> = spans.collect();
        self.emit(spans, end + wrap.len())
    }
}

enum Style {
    Bold,
    Italic,
    Strike,
}

/// A link's label carries the target on every span it is made of, so a styled link is
/// still one link however many runs it took to write.
fn linked(label: Vec<Span>, target: Target) -> Vec<Span> {
    let label = match label.is_empty() {
        true => vec![Span::plain("")],
        false => label,
    };
    label
        .into_iter()
        .map(|mut span| {
            span.link = Some(target.clone());
            span
        })
        .collect()
}

/// `[label](target)` starting at `at`, and where it ends.
fn link_at(chars: &[char], at: usize) -> Option<(String, String, usize)> {
    if chars.get(at) != Some(&'[') {
        return None;
    }
    let label_end = find(chars, at + 1, ']')?;
    if chars.get(label_end + 1) != Some(&'(') {
        return None;
    }
    let target_end = find(chars, label_end + 2, ')')?;
    let target = take(chars, label_end + 2, target_end);
    match target.contains(char::is_whitespace) {
        true => None,
        false => Some((take(chars, at + 1, label_end), target, target_end + 1)),
    }
}

fn is_external(target: &str) -> bool {
    target.starts_with("http://") || target.starts_with("https://")
}

fn starts(chars: &[char], at: usize, pattern: &str) -> bool {
    pattern
        .chars()
        .enumerate()
        .all(|(offset, c)| chars.get(at + offset) == Some(&c))
}

fn find(chars: &[char], from: usize, c: char) -> Option<usize> {
    (from..chars.len()).find(|&i| chars[i] == c)
}

fn find_str(chars: &[char], from: usize, pattern: &str) -> Option<usize> {
    (from..chars.len()).find(|&i| starts(chars, i, pattern))
}

fn take(chars: &[char], from: usize, to: usize) -> String {
    chars[from..to].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Blocks written back out the way the note wrote them, so a table of cases stays
    /// readable. A round trip that looks like markdown means the parse kept the meaning.
    fn sketch(blocks: &[Block]) -> String {
        blocks
            .iter()
            .map(|block| match block {
                Block::Heading(level, spans) => format!("h{level}[{}]", runs(spans)),
                Block::Para(spans) => format!("p[{}]", runs(spans)),
                Block::Quote(spans) => format!("q[{}]", runs(spans)),
                Block::Code { lang, text } => format!("code:{lang}[{text}]"),
                Block::Item {
                    indent,
                    marker,
                    body,
                } => {
                    format!("li{indent}:{marker}[{}]", runs(body))
                }
                Block::Rule => "hr".to_string(),
                Block::Table { align, head, rows } => {
                    let row = |cells: &Vec<Vec<Span>>| {
                        cells.iter().map(|c| runs(c)).collect::<Vec<_>>().join("|")
                    };
                    let aligned: String =
                        align.iter().map(|a| format!("{a:?}").remove(0)).collect();
                    let body: Vec<String> = rows.iter().map(row).collect();
                    format!("table:{aligned}[{}]{{{}}}", row(head), body.join(";"))
                }
            })
            .collect()
    }

    fn runs(spans: &[Span]) -> String {
        spans
            .iter()
            .map(|span| {
                let mut text = span.text.clone();
                if span.code {
                    text = format!("`{text}`");
                }
                if span.bold {
                    text = format!("**{text}**");
                }
                if span.italic {
                    text = format!("*{text}*");
                }
                if span.strike {
                    text = format!("~~{text}~~");
                }
                if let Some(hover) = &span.hover {
                    text = format!("{text}({hover})");
                }
                match &span.link {
                    Some(Target::Url(url)) => format!("[{text}](url:{url})"),
                    Some(Target::Note {
                        path,
                        relative: true,
                    }) => format!("[{text}](rel:{path})"),
                    Some(Target::Note {
                        path,
                        relative: false,
                    }) => format!("[{text}](note:{path})"),
                    None => text,
                }
            })
            .collect()
    }

    /// The cases the page has always had to render, and what they have to come out as.
    #[test]
    fn notes_render_the_way_they_always_have() {
        let cases: &[(&str, &str)] = &[
            ("# Title", "h1[Title]"),
            (
                "- [ ] todo\n- [x] done",
                "li0:\u{2610}[todo]li0:\u{2611}[done]",
            ),
            ("1. one\n2. two", "li0:1.[one]li0:2.[two]"),
            ("1. one\n1. two", "li0:1.[one]li0:2.[two]"),
            ("> quoted", "q[quoted]"),
            ("see **bold** and `code`", "p[see **bold** and `code`]"),
            (
                "a [[Wiki Link|alias]] here",
                "p[a [alias](note:Wiki Link) here]",
            ),
            ("[next](../ideas/1.md)", "p[[next](rel:../ideas/1.md)]"),
            ("[docs](https://x.com)", "p[[docs](url:https://x.com)]"),
            ("```\nraw <b>x</b>\n```", "code:[raw <b>x</b>]"),
            ("<script>alert(1)</script>", "p[<script>alert(1)</script>]"),
            ("<!--toc:start-->\ntext", "p[text]"),
            ("---\ntype: Note\ntags: [a]\n---\n\n# Real", "h1[Real]"),
            ("one line\nand its rest", "p[one line and its rest]"),
            ("first para\n\nsecond para", "p[first para]p[second para]"),
            (
                "lead in\n> quoted one\n> quoted two\nafter",
                "p[lead in]q[quoted one quoted two after]",
            ),
            ("> quoted\n\nafter", "q[quoted]p[after]"),
            ("| a | b |\n|---|---|\n| 1 | 2 |", "table:SS[a|b]{1|2}"),
            ("| a | b |\n|:--|--:|\n| 1 |", "table:SE[a|b]{1|}"),
            (
                "- a\n  - b\n- c",
                "li0:\u{2022}[a]li1:\u{2022}[b]li0:\u{2022}[c]",
            ),
            ("- a\n  1. b\n  2. c", "li0:\u{2022}[a]li1:1.[b]li1:2.[c]"),
            ("- a\n\n- b", "li0:\u{2022}[a]li0:\u{2022}[b]"),
            ("- a\n  wrapped", "li0:\u{2022}[a wrapped]"),
            ("`a_*b*_c`", "p[`a_*b*_c`]"),
            ("~~gone~~", "p[~~gone~~]"),
            ("<https://x.com>", "p[[https://x.com](url:https://x.com)]"),
            (
                "![pic](https://x.com/a.png)",
                "p[[pic](url:https://x.com/a.png)]",
            ),
            ("```js\nlet x\n```", "code:js[let x]"),
            (
                "cited[^a]\n\n[^a]: the source",
                "p[cited[1](the source)]hrli0:1.[the source]",
            ),
        ];
        let wrong: Vec<String> = cases
            .iter()
            .filter(|(src, want)| sketch(&render(src)) != *want)
            .map(|(src, want)| format!("{src:?} gave {:?}, want {want:?}", sketch(&render(src))))
            .collect();
        assert!(wrong.is_empty(), "{}", wrong.join(" | "));
    }

    /// Nothing a note writes becomes markup, and nothing it writes becomes a URL the
    /// window would open. Anything that is not `http(s)` is a note path, which resolves
    /// inside the vault or greys out.
    #[test]
    fn a_note_can_only_ever_point_at_a_note_or_at_the_web() {
        let blocks = render("[x](javascript:alert(1)) <img onerror=alert(1)> &lt;b&gt;");
        let urls: Vec<&Target> = blocks
            .iter()
            .filter_map(|b| match b {
                Block::Para(spans) => Some(spans),
                _ => None,
            })
            .flatten()
            .filter_map(|s| s.link.as_ref())
            .collect();
        assert_eq!(
            urls,
            vec![&Target::Note {
                path: "javascript:alert(1".into(),
                relative: true
            }],
            "a target ends at its first `)`, and only http(s) is ever a URL"
        );
        // The angle brackets are text, because there is nowhere for them to be markup.
        // The `)` the target did not take is text, and so is everything after it.
        assert_eq!(
            sketch(&blocks),
            "p[[x](rel:javascript:alert(1)) <img onerror=alert(1)> &lt;b&gt;]"
        );
    }

    #[test]
    fn an_unclosed_fence_still_renders_what_it_holds() {
        assert_eq!(sketch(&render("```\nstuck")), "code:[stuck]");
    }
}
