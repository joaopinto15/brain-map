//! Following a link out of a note. Targets resolve the way `links.rs` resolves them on
//! the server, so a link that drew an edge in the graph is a link the reader can follow:
//! wikilinks by note name or by path, markdown links relative to the linking note, or
//! bundle-absolute from the vault root.

use std::collections::HashMap;

/// Every note under both the names it answers to: its bare stem, and its full path.
pub struct Index {
    by_key: HashMap<String, usize>,
}

impl Index {
    pub fn of(ids: impl IntoIterator<Item = String>) -> Index {
        let mut by_key = HashMap::new();
        for (i, id) in ids.into_iter().enumerate() {
            if brain_map_model::is_structural(&id) {
                continue;
            }
            let stem = id.rsplit('/').next().unwrap_or(&id);
            by_key.entry(strip_md(stem).to_lowercase()).or_insert(i);
            by_key.entry(key(&id)).or_insert(i);
        }
        Index { by_key }
    }

    /// The note a link points at. `relative` marks a markdown link, which resolves
    /// against the folder of the note it was written in; a wikilink does not.
    pub fn resolve(&self, target: &str, from: Option<&str>, relative: bool) -> Option<usize> {
        // The target reaches here exactly as the note wrote it: nothing escapes it on
        // the way in, so nothing has to unescape it here.
        let clean = target.split('#').next().unwrap_or("").trim();
        if clean.is_empty() {
            return None;
        }
        if relative && !clean.starts_with('/') {
            let dir = from
                .and_then(|f| f.rsplit_once('/').map(|(dir, _)| dir))
                .unwrap_or("");
            let joined = match dir.is_empty() {
                true => clean.to_string(),
                false => format!("{dir}/{clean}"),
            };
            return self.by_key.get(&key(&joined)).copied();
        }
        self.by_key.get(&key(clean)).copied()
    }
}

fn strip_md(path: &str) -> &str {
    match path.len() >= 3 && path[path.len() - 3..].eq_ignore_ascii_case(".md") {
        true => &path[..path.len() - 3],
        false => path,
    }
}

/// A path reduced to what identifies it: no `.` or `..` steps, no extension, no case.
fn key(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            ".." => {
                parts.pop();
            }
            "" | "." => {}
            part => parts.push(part),
        }
    }
    strip_md(&parts.join("/")).to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> Index {
        Index::of(
            [
                "__vault__",
                "__dir__ideas",
                "ideas/0.md",
                "ideas/1.md",
                "daily/0.md",
            ]
            .map(String::from),
        )
    }

    #[test]
    fn a_link_resolves_the_way_the_graph_drew_it() {
        let index = index();
        let target = Some(3);
        let cases: [(&str, Option<usize>, (&str, Option<&str>, bool)); 9] = [
            ("by note name", target, ("1", None, false)),
            ("by path", target, ("ideas/1.md", None, false)),
            (
                "by path without the extension",
                target,
                ("ideas/1", None, false),
            ),
            (
                "relative to the linking note",
                target,
                ("1.md", Some("ideas/0.md"), true),
            ),
            (
                "bundle-absolute",
                target,
                ("/ideas/1.md", Some("daily/0.md"), true),
            ),
            (
                "up and over",
                target,
                ("../ideas/1.md", Some("daily/0.md"), true),
            ),
            (
                "past an anchor",
                target,
                ("ideas/1.md#section", None, false),
            ),
            (
                "a target nobody wrote",
                None,
                ("nowhere.md", Some("ideas/0.md"), true),
            ),
            ("an empty target", None, ("", None, false)),
        ];
        let wrong: Vec<String> = cases
            .iter()
            .filter_map(|(what, want, (target, from, relative))| {
                let got = index.resolve(target, *from, *relative);
                (got != *want).then(|| format!("{what}: got {got:?} want {want:?}"))
            })
            .collect();
        assert!(wrong.is_empty(), "{}", wrong.join(" | "));
    }

    #[test]
    fn structural_nodes_are_not_link_targets() {
        assert_eq!(index().resolve("__vault__", None, false), None);
    }
}
