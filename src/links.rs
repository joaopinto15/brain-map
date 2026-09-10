//! Connections read out of note text: `[[wikilinks]]` and markdown links, either
//! relative or Open Knowledge Format bundle-absolute (`/tables/customers.md`).

use crate::node::{normalize, parent_of, stem_of, NodeId};
use crate::vault::Vault;
use std::collections::HashMap;

pub struct Links {
    pub edges: Vec<(NodeId, NodeId)>,
    /// Notes reached by a markdown link that the scan itself skipped.
    pub external: Vec<NodeId>,
}

impl Links {
    pub fn resolve(vault: &Vault) -> Links {
        // Wikilinks resolve by note name, or by path when the link spells one out.
        let mut by_key: HashMap<String, NodeId> = HashMap::new();
        for rel in vault.paths() {
            let note = NodeId::Note(rel.to_string());
            by_key
                .entry(stem_of(rel).to_lowercase())
                .or_insert_with(|| note.clone());
            by_key
                .entry(normalize(rel).trim_end_matches(".md").to_lowercase())
                .or_insert(note);
        }

        let mut edges = Vec::new();
        let mut external = Vec::new();
        for note in &vault.notes {
            let from = NodeId::Note(note.path.clone());
            let (wiki, md) = extract_links(&note.text);

            for target in wiki {
                let key = normalize(target.trim().trim_end_matches(".md")).to_lowercase();
                match by_key.get(&key) {
                    Some(to) if *to != from => edges.push((from.clone(), to.clone())),
                    _ => {}
                }
            }

            for target in md {
                // OKF bundle-absolute links start at the root; everything else is relative.
                let resolved = match target.strip_prefix('/') {
                    Some(abs) => normalize(abs),
                    None => normalize(&format!("{}/{}", parent_of(&note.path), target)),
                };
                if resolved.is_empty() || resolved == note.path {
                    continue;
                }
                let to = NodeId::Note(resolved.clone());
                if !vault.has_note(&resolved) {
                    if !vault.has_file(&resolved) {
                        continue;
                    }
                    if !external.contains(&to) {
                        external.push(to.clone());
                    }
                }
                edges.push((from.clone(), to));
            }
        }
        Links { edges, external }
    }
}

fn find(hay: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    (from..hay.len().saturating_sub(needle.len() - 1))
        .find(|&i| &hay[i..i + needle.len()] == needle)
}

/// Wikilink targets `[[target|alias]]` and relative markdown links `[text](other.md)`.
pub fn extract_links(text: &str) -> (Vec<String>, Vec<String>) {
    let bytes = text.as_bytes();
    let (mut wiki, mut md) = (Vec::new(), Vec::new());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"[[") {
            if let Some(end) = find(bytes, i + 2, b"]]") {
                let target = text[i + 2..end]
                    .split(['|', '#'])
                    .next()
                    .unwrap_or("")
                    .trim();
                if !target.is_empty() {
                    wiki.push(target.to_string());
                }
                i = end + 2;
                continue;
            }
        }
        if bytes[i] == b'(' && i > 0 && bytes[i - 1] == b']' {
            if let Some(end) = find(bytes, i + 1, b")") {
                let target = &text[i + 1..end];
                if target.ends_with(".md")
                    && !target.contains(char::is_whitespace)
                    && !target.contains('#')
                {
                    md.push(target.to_string());
                }
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }
    (wiki, md)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::fixture;

    #[test]
    fn parses_both_link_syntaxes() {
        let text = "see [[Alpha|the first]] and [[notes/Beta#head]] and [x](../Gamma.md) \
                    but not [y](https://example.com) or [z](other.md#anchor)";
        let (wiki, md) = extract_links(text);
        assert_eq!(wiki, ["Alpha", "notes/Beta"]);
        assert_eq!(md, ["../Gamma.md"]);
    }

    fn edges(vault: &Vault) -> Vec<(String, String)> {
        Links::resolve(vault)
            .edges
            .iter()
            .map(|(a, b)| (a.node_id(), b.node_id()))
            .collect()
    }

    #[test]
    fn resolves_by_name_and_by_path() {
        let vault = fixture(&[
            ("Index.md", "[[Graphs]] and [[ideas/Graphs]]"),
            ("ideas/Graphs.md", ""),
        ]);
        assert_eq!(
            edges(&vault),
            [
                ("Index.md".to_string(), "ideas/Graphs.md".to_string()),
                ("Index.md".to_string(), "ideas/Graphs.md".to_string()),
            ]
        );
    }

    #[test]
    fn resolves_relative_markdown_links() {
        let vault = fixture(&[("ideas/Graphs.md", "[rust](../Rust.md)"), ("Rust.md", "")]);
        assert_eq!(
            edges(&vault),
            [("ideas/Graphs.md".to_string(), "Rust.md".to_string())]
        );
    }

    #[test]
    fn resolves_bundle_absolute_markdown_links() {
        let vault = fixture(&[
            ("metrics/revenue.md", "[orders](/tables/orders.md)"),
            ("tables/orders.md", ""),
        ]);
        assert_eq!(
            edges(&vault),
            [(
                "metrics/revenue.md".to_string(),
                "tables/orders.md".to_string()
            )]
        );
    }

    #[test]
    fn drops_self_links_and_unknown_targets() {
        let vault = fixture(&[(
            "A.md",
            "[[A]] [[Nowhere]] [gone](missing.md) [web](https://x.com)",
        )]);
        assert!(edges(&vault).is_empty());
        assert!(Links::resolve(&vault).external.is_empty());
    }

    #[test]
    fn alias_and_anchor_target_the_same_note() {
        let vault = fixture(&[("A.md", "[[B|call it what you like]]"), ("B.md", "")]);
        assert_eq!(edges(&vault), [("A.md".to_string(), "B.md".to_string())]);
    }
}
