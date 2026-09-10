//! The frontmatter, as a value tree. The subset of YAML that OKF bundles are written in:
//! block mappings and lists, flow `{ k: v }` mappings and `[a, b]` lists, quoted and plain
//! scalars, and a plain scalar folded over indented continuation lines. Every value is a
//! string — a timestamp stays the text the author wrote, which is what the YAML 1.2 core
//! schema does and what a round-trip needs.
//!
//! ponytail: no anchors, no block scalars (`|`, `>`), no tags, no multi-document streams.
//! None of the sample bundles use them; a real YAML crate is a dependency the scanner does
//! not take.

#[derive(Clone, Debug, PartialEq)]
pub enum Yaml {
    Str(String),
    List(Vec<Yaml>),
    Map(Vec<(String, Yaml)>),
}

impl Yaml {
    pub fn get(&self, key: &str) -> Option<&Yaml> {
        match self {
            Yaml::Map(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn str(&self) -> Option<&str> {
        match self {
            Yaml::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// A field's text, or empty — the shape a frontmatter reader wants for a scalar key.
    pub fn text(&self, key: &str) -> &str {
        self.get(key).and_then(Yaml::str).unwrap_or("")
    }

    /// A list, or a lone value read as a one-element list — which is how OKF says to read
    /// a bare `verified` mapping, and how a `tags: word` is meant.
    pub fn items(&self) -> Vec<&Yaml> {
        match self {
            Yaml::List(items) => items.iter().collect(),
            other => vec![other],
        }
    }
}

struct Line<'a> {
    indent: usize,
    text: &'a str,
}

/// The value a document holds. A document with no mapping at the top is a string.
pub fn parse(source: &str) -> Yaml {
    let lines: Vec<Line> = source
        .lines()
        .map(strip_comment)
        .filter(|l| !l.trim().is_empty())
        .map(|l| Line {
            indent: l.len() - l.trim_start().len(),
            text: l.trim(),
        })
        .collect();
    let mut at = 0;
    block(&lines, &mut at, 0)
}

/// The value starting at `lines[at]`, which must sit at `indent` or deeper.
fn block(lines: &[Line], at: &mut usize, indent: usize) -> Yaml {
    let Some(first) = lines.get(*at) else {
        return Yaml::Str(String::new());
    };
    if first.text.starts_with("- ") || first.text == "-" {
        return list(lines, at, first.indent);
    }
    if split_key(first.text).is_some() {
        return map(lines, at, first.indent, None);
    }
    *at += 1;
    let mut text = first.text.to_string();
    fold(lines, at, indent, &mut text);
    scalar(&text)
}

/// A block mapping whose entries sit at `indent`. `head` is an entry already split off
/// the `- ` of a list item, which the rest of the mapping continues under.
fn map(lines: &[Line], at: &mut usize, indent: usize, head: Option<(&str, &str)>) -> Yaml {
    let mut entries = Vec::new();
    if let Some((key, value)) = head {
        entries.push((key.to_string(), value_of(lines, at, indent, value)));
    }
    while let Some(line) = lines.get(*at) {
        if line.indent != indent {
            break;
        }
        let Some((key, value)) = split_key(line.text) else {
            break;
        };
        *at += 1;
        entries.push((key.to_string(), value_of(lines, at, indent, value)));
    }
    Yaml::Map(entries)
}

/// What follows `key:` — inline, or on the lines below it.
fn value_of(lines: &[Line], at: &mut usize, indent: usize, inline: &str) -> Yaml {
    if !inline.is_empty() {
        let mut text = inline.to_string();
        fold(lines, at, indent, &mut text);
        return scalar(&text);
    }
    match lines.get(*at) {
        // A list may sit level with its key, which is where PyYAML dumps it.
        Some(next) if next.indent >= indent && next.text.starts_with("- ") => {
            list(lines, at, next.indent)
        }
        Some(next) if next.indent > indent => block(lines, at, next.indent),
        _ => Yaml::Str(String::new()),
    }
}

/// A block list whose dashes sit at `indent`.
fn list(lines: &[Line], at: &mut usize, indent: usize) -> Yaml {
    let mut items = Vec::new();
    while let Some(line) = lines.get(*at) {
        if line.indent != indent || !(line.text.starts_with("- ") || line.text == "-") {
            break;
        }
        *at += 1;
        let rest = line.text[1..].trim_start();
        // Whatever follows the dash sits in the column after it, and so does the rest
        // of a mapping that started there.
        let inner = indent + (line.text.len() - rest.len());
        items.push(match rest {
            "" => block(lines, at, inner),
            _ => match split_key(rest) {
                Some((key, value)) => map(lines, at, inner, Some((key, value))),
                None => {
                    let mut text = rest.to_string();
                    fold(lines, at, indent, &mut text);
                    scalar(&text)
                }
            },
        });
    }
    Yaml::List(items)
}

/// A plain scalar continues onto lines indented deeper than its key, joined by spaces.
fn fold(lines: &[Line], at: &mut usize, indent: usize, text: &mut String) {
    if text.starts_with(['{', '[', '"', '\'']) {
        return;
    }
    while let Some(line) = lines.get(*at) {
        if line.indent <= indent || line.text.starts_with("- ") || split_key(line.text).is_some() {
            break;
        }
        text.push(' ');
        text.push_str(line.text);
        *at += 1;
    }
}

/// `key: value` or `key:` — the colon has to be followed by a space or end the line, so
/// `human:jsmith` and `https://…` are values, not keys.
fn split_key(text: &str) -> Option<(&str, &str)> {
    if text.starts_with(['{', '[', '"', '\'', '-', '#']) {
        return None;
    }
    let colon = text.find(':')?;
    let (key, rest) = text.split_at(colon);
    let rest = &rest[1..];
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    let key = key.trim();
    match key.is_empty() {
        true => None,
        false => Some((key, rest.trim())),
    }
}

/// An inline value: a flow mapping, a flow list, or a scalar with its quotes taken off.
fn scalar(text: &str) -> Yaml {
    let text = text.trim();
    if let Some(inner) = text.strip_prefix('{').and_then(|t| t.strip_suffix('}')) {
        return Yaml::Map(
            split_flow(inner)
                .iter()
                .filter_map(|entry| split_key(entry).or_else(|| entry.split_once(':')))
                .map(|(k, v)| (k.trim().to_string(), scalar(v)))
                .collect(),
        );
    }
    if let Some(inner) = text.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
        return Yaml::List(split_flow(inner).iter().map(|item| scalar(item)).collect());
    }
    Yaml::Str(unquote(text).to_string())
}

/// The comma-separated entries of a flow collection, with nested brackets and quotes
/// left whole.
fn split_flow(inner: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let (mut depth, mut quote, mut start) = (0, None, 0);
    for (i, c) in inner.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '{' | '[') => depth += 1,
            (None, '}' | ']') => depth -= 1,
            (None, ',') if depth == 0 => {
                parts.push(&inner[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&inner[start..]);
    parts
        .into_iter()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect()
}

/// A ` #` outside quotes starts a comment; a `#` glued to text, as in a URL, does not.
fn strip_comment(line: &str) -> &str {
    let mut quote = None;
    let mut prev = ' ';
    for (i, c) in line.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (None, '"' | '\'') if prev == ' ' || prev == ':' || i == 0 => quote = Some(c),
            (None, '#') if prev == ' ' || prev == '\t' || i == 0 => return &line[..i],
            _ => {}
        }
        prev = c;
    }
    line
}

pub fn unquote(s: &str) -> &str {
    let s = s.trim();
    for quote in ['"', '\''] {
        if let Some(inner) = s.strip_prefix(quote).and_then(|v| v.strip_suffix(quote)) {
            return inner;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> Yaml {
        Yaml::Str(text.into())
    }

    #[test]
    fn scalars_lists_and_both_shapes_of_each() {
        let doc = parse(
            "type: Metric\ntitle: \"Weekly revenue\"\ntags: [sales, 'q3']\nmore:\n  - ops\n  - oncall\nflat:\n- a\n- b\n",
        );
        assert_eq!(doc.text("type"), "Metric");
        assert_eq!(doc.text("title"), "Weekly revenue", "quotes come off");
        assert_eq!(
            doc.get("tags"),
            Some(&Yaml::List(vec![s("sales"), s("q3")]))
        );
        assert_eq!(
            doc.get("more"),
            Some(&Yaml::List(vec![s("ops"), s("oncall")]))
        );
        assert_eq!(
            doc.get("flat"),
            Some(&Yaml::List(vec![s("a"), s("b")])),
            "a list level with its key, the way PyYAML writes one"
        );
    }

    #[test]
    fn a_mapping_in_flow_form_and_in_block_form_read_the_same() {
        let flow = parse("generated: { by: human:jp, at: 2026-06-25T09:00:00Z }\n");
        let block = parse("generated:\n  by: human:jp\n  at: '2026-06-25T09:00:00Z'\n");
        assert_eq!(flow.get("generated"), block.get("generated"));
        assert_eq!(flow.get("generated").unwrap().text("by"), "human:jp");
    }

    #[test]
    fn a_list_of_mappings_in_every_way_the_samples_write_one() {
        let doc = parse(
            "verified:\n  - { by: process:nightly, at: 2026-06-26T02:00:00Z }\n  - { by: human:jp, at: 2026-06-25T09:00:00Z }\n\
             sources:\n  - id: policy\n    resource: policies/revenue.md\n    title: Revenue policy\n  - id: orders\n    resource: tables/orders.md\n\
             level:\n- id: a\n  title: A\n- id: b\n",
        );
        let by: Vec<&str> = doc
            .get("verified")
            .unwrap()
            .items()
            .iter()
            .map(|v| v.text("by"))
            .collect();
        assert_eq!(by, ["process:nightly", "human:jp"]);
        let sources = doc.get("sources").unwrap().items();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].text("title"), "Revenue policy");
        assert_eq!(sources[1].text("resource"), "tables/orders.md");
        let level = doc.get("level").unwrap().items();
        assert_eq!((level[0].text("id"), level[0].text("title")), ("a", "A"));
        assert_eq!(level[1].text("id"), "b");
    }

    #[test]
    fn a_plain_scalar_folds_and_a_colon_inside_a_value_is_not_a_key() {
        let doc = parse(
            "description: Computes the count of users who have\n  completed a purchase.\nresource: https://support.google.com/analytics/answer/9037342#zippy\nnext: x\n",
        );
        assert_eq!(
            doc.text("description"),
            "Computes the count of users who have completed a purchase."
        );
        assert_eq!(
            doc.text("resource"),
            "https://support.google.com/analytics/answer/9037342#zippy",
            "a URL keeps its scheme colon and its fragment hash"
        );
        assert_eq!(doc.text("next"), "x");
    }

    #[test]
    fn comments_and_nested_flow_values_are_handled() {
        let doc = parse(
            "status: stable # the default\nexecutor:\n  resource: skills/run.md\n  receipt: [job_id, executed_sql, result]\nparameters:\n  - { name: year, type: integer, required: true }\n",
        );
        assert_eq!(doc.text("status"), "stable");
        let executor = doc.get("executor").unwrap();
        assert_eq!(executor.text("resource"), "skills/run.md");
        assert_eq!(
            executor.get("receipt"),
            Some(&Yaml::List(vec![
                s("job_id"),
                s("executed_sql"),
                s("result")
            ]))
        );
        assert_eq!(
            doc.get("parameters").unwrap().items()[0].text("name"),
            "year"
        );
    }

    #[test]
    fn what_is_absent_reads_as_empty_rather_than_failing() {
        let doc = parse("type: Metric\nempty:\n");
        assert_eq!(doc.text("missing"), "");
        assert_eq!(doc.text("empty"), "");
        assert!(
            doc.get("type").unwrap().items().len() == 1,
            "a lone value is a one-item list"
        );
        assert_eq!(parse(""), s(""));
        assert_eq!(parse("just words"), s("just words"));
    }
}
