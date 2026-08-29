//! Merges a scanned vault, its layout and its links into the ordered graph the
//! browser draws, and writes it out as JSON.

use crate::layout::{Group, Layout};
use crate::links::Links;
use crate::node::NodeId;
use crate::vault::Vault;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub struct Node {
    pub id: NodeId,
    pub label: String,
    pub group: String,
}

pub struct Graph {
    pub vault_path: String,
    pub mode: &'static str,
    pub groups: Vec<Group>,
    pub nodes: Vec<Node>,
    pub links: Vec<(usize, usize)>,
}

pub fn build(root: &Path) -> Graph {
    let vault = Vault::scan(root);
    let layout = Layout::of(&vault);
    let links = Links::resolve(&vault);
    Graph::assemble(&vault, layout, links)
}

impl Graph {
    pub fn assemble(vault: &Vault, mut layout: Layout, links: Links) -> Graph {
        let mut ids: Vec<NodeId> = layout
            .group_of
            .keys()
            .filter(|id| !id.is_note())
            .cloned()
            .collect();
        ids.extend(vault.paths().map(|rel| NodeId::Note(rel.to_string())));

        if !links.external.is_empty() {
            layout.add_external_group();
            for id in &links.external {
                layout.group_of.insert(id.clone(), "external".into());
            }
            ids.extend(links.external.iter().cloned());
        }

        let mut edges: Vec<(NodeId, NodeId)> = layout.tree.clone();
        edges.extend(links.edges.iter().cloned());

        if !layout.keeps_isolates {
            let linked: HashSet<&NodeId> = edges.iter().flat_map(|(a, b)| [a, b]).collect();
            ids.retain(|id| linked.contains(id));
        }

        ids.sort_by_cached_key(|id| (layout.rank(layout.group_key(id)), id.sort_key()));
        let index: HashMap<&NodeId, usize> =
            ids.iter().enumerate().map(|(i, id)| (id, i)).collect();

        let mut links: Vec<(usize, usize)> = edges
            .iter()
            .filter_map(|(a, b)| Some((*index.get(a)?, *index.get(b)?)))
            .collect();
        links.sort_unstable();
        links.dedup();

        Graph {
            vault_path: vault.root.display().to_string(),
            mode: layout.mode,
            nodes: ids
                .iter()
                .map(|id| Node {
                    label: id.label(&vault.name),
                    group: layout.group_key(id).to_string(),
                    id: id.clone(),
                })
                .collect(),
            groups: layout.groups,
            links,
        }
    }

    pub fn to_json(&self) -> String {
        let groups: Vec<String> = self
            .groups
            .iter()
            .map(|g| {
                format!(
                    r#"{}:{{"c":{},"r":{},"glow":{},"name":{},"pace":{},"pause":{},"major":{},"cluster":{}}}"#,
                    esc(&g.key), esc(g.color), g.radius, g.glow, esc(&g.name),
                    g.pace, g.pause, g.major, g.cluster
                )
            })
            .collect();
        let nodes: Vec<String> = self
            .nodes
            .iter()
            .map(|n| {
                format!(
                    r#"{{"id":{},"label":{},"g":{}}}"#,
                    esc(&n.id.wire_id()),
                    esc(&n.label),
                    esc(&n.group)
                )
            })
            .collect();
        let links: Vec<String> = self
            .links
            .iter()
            .map(|(s, t)| format!(r#"{{"s":{s},"t":{t}}}"#))
            .collect();
        format!(
            r#"{{"vault":{},"groups":{{{}}},"nodes":[{}],"links":[{}]}}"#,
            esc(&self.vault_path),
            groups.join(","),
            nodes.join(","),
            links.join(",")
        )
    }
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '<' => out.push_str("\\u003c"),
            c if (c as u32) < 0x20 || c == '\u{2028}' || c == '\u{2029}' => {
                out.push_str(&format!("\\u{:04x}", c as u32))
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::fixture;

    fn graph_of(vault: &Vault) -> Graph {
        Graph::assemble(vault, Layout::of(vault), Links::resolve(vault))
    }

    fn wire(graph: &Graph) -> Vec<String> {
        graph.nodes.iter().map(|n| n.id.wire_id()).collect()
    }

    #[test]
    fn escapes_json_strings() {
        assert_eq!(esc("a\"b\n<c"), "\"a\\\"b\\n\\u003cc\"");
    }

    #[test]
    fn folders_lead_their_notes_and_the_vault_leads_everything() {
        let vault = fixture(
            false,
            &[("loose.md", ""), ("ideas/a.md", ""), ("ideas/b.md", "")],
        );
        assert_eq!(
            wire(&graph_of(&vault)),
            [
                "__vault__",
                "loose.md",
                "__dir__ideas",
                "ideas/a.md",
                "ideas/b.md"
            ]
        );
    }

    #[test]
    fn aios_drops_notes_nothing_links_to() {
        let vault = fixture(
            true,
            &[
                ("CLAUDE.md", "[[gh]]"),
                ("wiki/tools/gh.md", ""),
                ("wiki/orphan.md", ""),
            ],
        );
        assert_eq!(wire(&graph_of(&vault)), ["CLAUDE.md", "wiki/tools/gh.md"]);
    }

    #[test]
    fn a_generic_vault_keeps_isolates_because_the_tree_holds_them() {
        let vault = fixture(false, &[("orphan.md", "")]);
        assert_eq!(wire(&graph_of(&vault)), ["__vault__", "orphan.md"]);
    }

    #[test]
    fn link_indices_point_at_the_sorted_nodes() {
        let vault = fixture(false, &[("a.md", "[[b]]"), ("b.md", "")]);
        let graph = graph_of(&vault);
        let ids = wire(&graph);
        let named: Vec<(&str, &str)> = graph
            .links
            .iter()
            .map(|(s, t)| (ids[*s].as_str(), ids[*t].as_str()))
            .collect();
        assert!(named.contains(&("a.md", "b.md")));
        assert!(named.contains(&("__vault__", "a.md")));
    }

    /// The one test that touches the disk: `Vault::scan` is the only stage that can,
    /// and skipping and external discovery are invisible from an in-memory fixture.
    #[test]
    fn scans_a_real_folder_end_to_end() {
        let dir = std::env::temp_dir().join(format!("brain-map-e2e-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for sub in ["ideas", ".obsidian", "node_modules/pkg"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        std::fs::write(
            dir.join("Index.md"),
            "[[Graphs]] [vendored](node_modules/pkg/README.md)",
        )
        .unwrap();
        std::fs::write(dir.join("ideas/Graphs.md"), "").unwrap();
        std::fs::write(dir.join(".obsidian/skip.md"), "").unwrap();
        std::fs::write(dir.join("node_modules/pkg/README.md"), "").unwrap();

        let graph = build(&dir);
        let ids = wire(&graph);
        std::fs::remove_dir_all(&dir).unwrap();

        assert!(
            !ids.iter().any(|id| id.contains(".obsidian")),
            "dot folders are skipped"
        );
        assert!(ids.contains(&"ideas/Graphs.md".to_string()));
        let external = graph
            .nodes
            .iter()
            .find(|n| n.group == "external")
            .expect("external node");
        assert_eq!(external.id.wire_id(), "node_modules/pkg/README.md");
        assert_eq!(external.label, "pkg", "a README is labelled by its folder");
        assert_eq!(graph.mode, "generic folder grouping");
    }

    #[test]
    fn json_carries_every_group_node_and_link() {
        let vault = fixture(false, &[("a.md", "[[b]]"), ("b.md", "")]);
        let json = graph_of(&vault).to_json();
        assert!(json.contains(r#""id":"__vault__","label":"MyVault""#));
        assert!(json.contains(r#""root":{"c":"#));
        assert!(json.contains(r#""links":[{"s":"#));
    }
}
