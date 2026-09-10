//! Merges a scanned vault, its layout and its links into the ordered graph the
//! browser draws, and writes it out as JSON.

use crate::layout::{Group, Layout};
use crate::links::Links;
use crate::node::NodeId;
use crate::okf;
use crate::vault::Vault;
use brain_map_model::Graph;
use std::collections::HashMap;
use std::path::Path;

pub struct Node {
    pub id: NodeId,
    pub label: String,
    pub group: String,
    pub tags: Vec<String>,
    /// What the note's `icon:` said. A folder or the vault has no note, so it has none.
    pub icon: String,
    pub signals: Vec<String>,
    pub concept: brain_map_model::Concept,
}

pub struct Scan {
    pub vault_path: String,
    pub groups: Vec<Group>,
    pub nodes: Vec<Node>,
    pub links: Vec<(usize, usize)>,
}

pub fn build(root: &Path) -> Scan {
    let vault = Vault::scan(root);
    let layout = Layout::of(&vault);
    let links = Links::resolve(&vault);
    Scan::assemble(&vault, layout, links)
}

impl Scan {
    pub fn assemble(vault: &Vault, mut layout: Layout, links: Links) -> Scan {
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

        ids.sort_by_cached_key(|id| (layout.rank(layout.group_key(id)), id.sort_key()));
        let index: HashMap<&NodeId, usize> =
            ids.iter().enumerate().map(|(i, id)| (id, i)).collect();

        let mut links: Vec<(usize, usize)> = edges
            .iter()
            .filter_map(|(a, b)| Some((*index.get(a)?, *index.get(b)?)))
            .collect();
        links.sort_unstable();
        links.dedup();

        let now = okf::now();
        Scan {
            vault_path: vault.root.display().to_string(),
            nodes: ids
                .iter()
                .map(|id| {
                    // A note's frontmatter `title` names it better than its filename does.
                    let note = match id {
                        NodeId::Note(path) => vault.note(path),
                        _ => None,
                    };
                    Node {
                        label: note
                            .and_then(|n| n.front.title.clone())
                            .unwrap_or_else(|| id.label(&vault.name)),
                        group: layout.group_key(id).to_string(),
                        tags: note.map(|n| n.front.tags.clone()).unwrap_or_default(),
                        icon: note.and_then(|n| n.front.icon.clone()).unwrap_or_default(),
                        signals: note
                            .map(|n| okf::signals(&n.front, now))
                            .unwrap_or_default(),
                        concept: note.map(|n| n.front.about.clone()).unwrap_or_default(),
                        id: id.clone(),
                    }
                })
                .collect(),
            groups: layout.groups,
            links,
        }
    }

    /// The graph as the page receives it. Both ends share these types, so a field
    /// cannot mean one thing here and another there.
    pub fn into_graph(&self) -> Graph {
        Graph {
            vault: self.vault_path.clone(),
            groups: self.groups.clone(),
            nodes: self
                .nodes
                .iter()
                .map(|n| brain_map_model::Node {
                    id: n.id.node_id(),
                    label: n.label.clone(),
                    group: n.group.clone(),
                    tags: n.tags.clone(),
                    icon: n.icon.clone(),
                    signals: n.signals.clone(),
                    concept: n.concept.clone(),
                })
                .collect(),
            links: self
                .links
                .iter()
                .map(|&(s, t)| brain_map_model::Link { s, t })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::fixture;

    fn scan_of(vault: &Vault) -> Scan {
        Scan::assemble(vault, Layout::of(vault), Links::resolve(vault))
    }

    fn ids(scan: &Scan) -> Vec<String> {
        scan.nodes.iter().map(|n| n.id.node_id()).collect()
    }

    #[test]
    fn the_structure_leads_and_the_concepts_follow_it() {
        let vault = fixture(&[("loose.md", ""), ("ideas/a.md", ""), ("ideas/b.md", "")]);
        assert_eq!(
            ids(&scan_of(&vault)),
            [
                "__vault__",
                "__dir__ideas",
                "loose.md",
                "ideas/a.md",
                "ideas/b.md"
            ],
            "the tree grows first, then the concepts type by type"
        );
    }

    #[test]
    fn a_bundle_keeps_isolates_because_the_tree_holds_them() {
        let vault = fixture(&[("orphan.md", "")]);
        assert_eq!(ids(&scan_of(&vault)), ["__vault__", "orphan.md"]);
    }

    #[test]
    fn link_indices_point_at_the_sorted_nodes() {
        let vault = fixture(&[("a.md", "[[b]]"), ("b.md", "")]);
        let graph = scan_of(&vault);
        let ids = ids(&graph);
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
        let ids = ids(&graph);
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
        assert_eq!(external.id.node_id(), "node_modules/pkg/README.md");
        assert_eq!(external.label, "pkg", "a README is labelled by its folder");
    }

    #[test]
    fn frontmatter_titles_tags_and_signals_reach_the_graph() {
        let vault = fixture(&[
            (
                "m/rev.md",
                "---\ntype: Metric\ntitle: Weekly revenue\ntags: [sales, finance]\n\
                 verified: { by: human:jp, at: 2026-06-25T09:00:00Z }\nstale_after: 2020-01-01T00:00:00Z\n---\n",
            ),
            ("m/plain.md", ""),
        ]);
        let graph = scan_of(&vault).into_graph();
        let on = |id: &str| {
            graph
                .nodes
                .iter()
                .find(|n| n.id == id)
                .expect("in the graph")
        };
        let titled = on("m/rev.md");
        assert_eq!(
            (titled.label.as_str(), titled.group.as_str()),
            ("Weekly revenue", "Metric"),
            "a concept is named by its title and grouped by its type"
        );
        assert_eq!(titled.tags, ["sales", "finance"]);
        assert_eq!(titled.signals, ["human-reviewed", "stale"]);
        assert!(
            on("m/plain.md").tags.is_empty(),
            "an untagged note carries no tags"
        );
        assert_eq!(
            on("m/plain.md").signals,
            ["unverified"],
            "and one that declared no trust is unverified, not signal-less"
        );
        assert!(
            on("__dir__m").signals.is_empty(),
            "a folder declares nothing, so it carries nothing"
        );
    }

    #[test]
    fn a_note_carries_the_icon_it_declares_and_no_other() {
        let vault = fixture(&[
            (
                "ideas/drawn.md",
                "---\nicon: \u{1f427}\ntags: [linux]\n---\n",
            ),
            ("ideas/quiet.md", "---\ntags: [linux]\n---\n"),
        ]);
        let graph = scan_of(&vault).into_graph();
        let on = |id: &str| {
            graph
                .nodes
                .iter()
                .find(|n| n.id == id)
                .expect("in the graph")
        };
        assert_eq!(on("ideas/drawn.md").icon, "\u{1f427}", "the note said so");
        assert_eq!(
            on("ideas/quiet.md").icon,
            "",
            "and a note that said nothing gets none"
        );
        assert_eq!(
            on("__dir__ideas").icon,
            "",
            "a folder has no note to say it"
        );
    }

    #[test]
    fn the_graph_carries_every_group_node_and_link() {
        let vault = fixture(&[("a.md", "[[b]]"), ("b.md", "")]);
        let graph = scan_of(&vault).into_graph();
        assert_eq!(graph.nodes[0].id, "__vault__");
        assert_eq!(graph.nodes[0].label, "MyVault");
        assert!(graph.group("Untyped").is_some());
        assert!(!graph.links.is_empty());
    }
}
