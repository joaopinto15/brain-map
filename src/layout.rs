//! How concepts are grouped, coloured and paced, and what structure holds them together.
//!
//! A vault is an Open Knowledge Format bundle, so a concept's group is the `type` it
//! declares and nothing else — not its folder, not a keyword, not a guess. A concept that
//! declares none is still a concept (§11), and the reserved `index.md` and `log.md` are
//! not concepts at all, so each gets a group of its own rather than being dropped.

use crate::node::NodeId;
use crate::okf;
use crate::vault::Vault;
use std::collections::HashMap;

const PALETTE: [&str; 12] = [
    "#60a5fa", "#fbbf24", "#f472b6", "#a78bfa", "#fb923c", "#22d3ee", "#f87171", "#4ade80",
    "#e879f9", "#facc15", "#94a3b8", "#2dd4bf",
];

pub use brain_map_model::Group;

/// The vault and folder nodes: the tree that holds a bundle together when its concepts
/// link to nothing yet.
const STRUCTURE: &str = "__structure";

pub struct Layout {
    /// Ordered: position drives both the growth sequence and the legend.
    pub groups: Vec<Group>,
    pub group_of: HashMap<NodeId, String>,
    /// Vault → folder → concept edges, so a bundle with no links still forms a galaxy.
    pub tree: Vec<(NodeId, NodeId)>,
}

impl Layout {
    pub fn of(vault: &Vault) -> Layout {
        let mut groups = vec![group(
            STRUCTURE,
            "#34d399",
            11.0,
            30.0,
            &vault.name,
            0,
            0,
            true,
        )];
        let mut group_of = HashMap::new();

        // One group per concept type, smallest first so the growth starts tight. A type
        // says what a concept *is*; what it is about is its tags, which cut across types
        // and filter on their own axis.
        let mut types: Vec<(String, Vec<&str>)> = Vec::new();
        for note in &vault.notes {
            let key = okf::concept_type(&note.path, &note.front);
            match types.iter_mut().find(|(k, _)| k == key) {
                Some((_, members)) => members.push(note.path.as_str()),
                None => types.push((key.to_string(), vec![note.path.as_str()])),
            }
        }
        types.sort_by_key(|(_, members)| members.len());

        for (i, (key, members)) in types.iter().enumerate() {
            let big = members.len() > 60;
            groups.push(Group {
                key: key.clone(),
                color: PALETTE[i % PALETTE.len()].into(),
                radius: if big { 3.5 } else { 6.0 },
                glow: if big { 0.0 } else { 14.0 },
                name: key.clone(),
                pace: (2200 / members.len().max(1)).clamp(8, 400) as u32,
                pause: 450,
                major: !big,
                cluster: false,
            });
            for rel in members {
                group_of.insert(NodeId::Note((*rel).to_string()), key.clone());
            }
        }

        group_of.insert(NodeId::Vault, STRUCTURE.into());

        // The directories a bundle organizes its concepts into are structure, not type:
        // one folder holds several types and a type spreads over several folders.
        let mut tree = Vec::new();
        for rel in vault.paths() {
            let note = NodeId::Note(rel.to_string());
            let Some((top, _)) = rel.split_once('/') else {
                tree.push((NodeId::Vault, note));
                continue;
            };
            let folder = NodeId::Folder(top.to_string());
            if !group_of.contains_key(&folder) {
                group_of.insert(folder.clone(), STRUCTURE.into());
                tree.push((NodeId::Vault, folder.clone()));
            }
            tree.push((folder, note));
        }

        Layout {
            groups,
            group_of,
            tree,
        }
    }

    pub fn group_key(&self, id: &NodeId) -> &str {
        self.group_of.get(id).map_or(okf::UNTYPED, |g| g.as_str())
    }

    /// The structural tree first, then group order. Drives the growth sequence.
    pub fn rank(&self, key: &str) -> usize {
        if key == STRUCTURE {
            return 0;
        }
        self.groups.iter().position(|g| g.key == key).unwrap_or(99) + 1
    }

    pub fn add_external_group(&mut self) {
        if !self.groups.iter().any(|g| g.key == "external") {
            self.groups.push(group(
                "external", "#3e4c63", 2.1, 0.0, "Files", 5, 600, false,
            ));
        }
    }
}

#[allow(clippy::too_many_arguments)] // group style table, not an interface
fn group(
    key: &str,
    color: &'static str,
    radius: f64,
    glow: f64,
    name: &str,
    pace: u32,
    pause: u32,
    major: bool,
) -> Group {
    Group {
        key: key.into(),
        color: color.into(),
        radius,
        glow,
        name: name.into(),
        pace,
        pause,
        major,
        cluster: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::fixture;

    fn keys(layout: &Layout) -> Vec<&str> {
        layout.groups.iter().map(|g| g.key.as_str()).collect()
    }

    #[test]
    fn a_concept_groups_by_the_type_it_declares() {
        let vault = fixture(&[
            ("tables/orders.md", "---\ntype: BigQuery Table\n---\n"),
            ("metrics/revenue.md", "---\ntype: Metric\n---\n"),
            ("metrics/churn.md", "---\ntype: Metric\n---\n"),
            ("scratch.md", "no frontmatter"),
        ]);
        let layout = Layout::of(&vault);
        assert_eq!(
            keys(&layout),
            ["__structure", "BigQuery Table", "Untyped", "Metric"],
            "smallest type first, so the growth starts tight"
        );
        assert_eq!(
            layout.group_key(&NodeId::Note("metrics/churn.md".into())),
            "Metric",
            "a type cuts across the folders its concepts sit in"
        );
        assert_eq!(
            layout.group_key(&NodeId::Note("tables/orders.md".into())),
            "BigQuery Table"
        );
        assert_eq!(
            layout.group_key(&NodeId::Note("scratch.md".into())),
            "Untyped",
            "a concept with no type is still a concept"
        );
    }

    #[test]
    fn the_reserved_files_are_not_concepts() {
        let vault = fixture(&[
            ("index.md", "# Bundle\n"),
            ("log.md", "# Log\n"),
            ("metrics/index.md", "# Metrics\n"),
            ("metrics/revenue.md", "---\ntype: Metric\n---\n"),
        ]);
        let layout = Layout::of(&vault);
        assert_eq!(keys(&layout), ["__structure", "Metric", "Index & log"]);
        assert_eq!(
            layout.group_key(&NodeId::Note("metrics/index.md".into())),
            "Index & log",
            "at any level of the bundle"
        );
    }

    #[test]
    fn folders_are_structure_and_hold_the_bundle_together() {
        let vault = fixture(&[("a.md", ""), ("tables/orders.md", "")]);
        let layout = Layout::of(&vault);
        assert!(layout
            .tree
            .contains(&(NodeId::Vault, NodeId::Note("a.md".into()))));
        assert!(layout
            .tree
            .contains(&(NodeId::Vault, NodeId::Folder("tables".into()))));
        assert!(layout.tree.contains(&(
            NodeId::Folder("tables".into()),
            NodeId::Note("tables/orders.md".into())
        )));
        assert_eq!(
            layout.group_key(&NodeId::Folder("tables".into())),
            STRUCTURE,
            "a folder holds several types, so it is one of none"
        );
        assert_eq!(layout.groups[0].name, "MyVault");
        assert_eq!(layout.rank(STRUCTURE), 0, "the tree grows before the types");
    }

    #[test]
    fn pace_shrinks_as_a_type_grows() {
        let notes: Vec<(String, String)> = (0..100)
            .map(|i| (format!("m/{i}.md"), "---\ntype: Metric\n---\n".to_string()))
            .collect();
        let borrowed: Vec<(&str, &str)> = notes
            .iter()
            .map(|(p, t)| (p.as_str(), t.as_str()))
            .collect();
        let layout = Layout::of(&fixture(&borrowed));
        let metric = layout.groups.iter().find(|g| g.key == "Metric").unwrap();
        assert_eq!(metric.pace, 22);
        assert!(
            !metric.major,
            "a type over 60 concepts renders small and fast"
        );
    }
}
