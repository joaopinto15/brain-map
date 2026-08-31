//! How notes are grouped, coloured and paced, and what structure holds them together.
//! Two adapters satisfy this interface: a generic folder layout and an AI Workshop OS
//! layout. Everything that differs between them is decided here, once. The generic
//! layout groups by frontmatter `type` instead of by folder when the notes declare one.

use crate::node::NodeId;
use crate::vault::{Note, Vault};
use std::collections::HashMap;

const PALETTE: [&str; 12] = [
    "#60a5fa", "#fbbf24", "#f472b6", "#a78bfa", "#fb923c", "#22d3ee", "#f87171", "#4ade80",
    "#e879f9", "#facc15", "#94a3b8", "#2dd4bf",
];

#[derive(Clone)]
pub struct Group {
    pub key: String,
    pub color: &'static str,
    pub radius: f64,
    pub glow: f64,
    pub name: String,
    /// Milliseconds between two notes of this group appearing during the growth animation.
    pub pace: u32,
    /// Milliseconds of silence before the group starts appearing.
    pub pause: u32,
    pub major: bool,
    pub cluster: bool,
}

pub struct Layout {
    /// Ordered: position drives both the growth sequence and the legend.
    pub groups: Vec<Group>,
    pub group_of: HashMap<NodeId, String>,
    /// Vault → folder → note edges, so link-free folders still form a galaxy.
    pub tree: Vec<(NodeId, NodeId)>,
    /// A layout without a structural tree has to drop unlinked notes, or they float alone.
    pub keeps_isolates: bool,
    pub mode: &'static str,
}

impl Layout {
    pub fn of(vault: &Vault) -> Layout {
        if vault.is_aios {
            Layout::aios(vault)
        } else {
            Layout::generic(vault)
        }
    }

    pub fn aios(vault: &Vault) -> Layout {
        Layout {
            groups: aios_groups(),
            group_of: vault
                .paths()
                .map(|rel| {
                    (
                        NodeId::Note(rel.to_string()),
                        aios_group_of(rel).to_string(),
                    )
                })
                .collect(),
            tree: Vec::new(),
            keeps_isolates: false,
            mode: "AI Workshop OS layout",
        }
    }

    pub fn generic(vault: &Vault) -> Layout {
        let mut groups = vec![Group {
            name: vault.name.clone(),
            ..aios_groups().remove(0)
        }];
        let mut group_of = HashMap::new();

        // One group per top-level folder, smallest first so the growth starts tight. A
        // note's section is where it lives; what it is about is its tags, which cut
        // across folders and filter separately.
        let mut tops: Vec<(String, String, Vec<&str>)> = Vec::new();
        for note in &vault.notes {
            let (key, name) = bucket(note);
            match tops.iter_mut().find(|(k, _, _)| *k == key) {
                Some((_, _, members)) => members.push(note.path.as_str()),
                None => tops.push((key, name, vec![note.path.as_str()])),
            }
        }
        tops.sort_by_key(|(_, _, members)| members.len());

        for (i, (key, name, members)) in tops.iter().enumerate() {
            let big = members.len() > 60;
            groups.push(Group {
                key: key.clone(),
                color: PALETTE[i % PALETTE.len()],
                radius: if big { 3.5 } else { 6.0 },
                glow: if big { 0.0 } else { 14.0 },
                name: name.clone(),
                pace: (2200 / members.len().max(1)).clamp(8, 400) as u32,
                pause: 450,
                major: !big,
                cluster: false,
            });
            for rel in members {
                group_of.insert(NodeId::Note((*rel).to_string()), key.clone());
            }
        }

        let claude = NodeId::Note("CLAUDE.md".into());
        if group_of.contains_key(&claude) {
            group_of.insert(claude, "router".into());
        }
        group_of.insert(NodeId::Vault, "router".into());

        let mut tree = Vec::new();
        for rel in vault.paths() {
            let note = NodeId::Note(rel.to_string());
            let Some((top, _)) = rel.split_once('/') else {
                tree.push((NodeId::Vault, note));
                continue;
            };
            let folder = NodeId::Folder(top.to_string());
            if !group_of.contains_key(&folder) {
                let key = group_of
                    .get(&note)
                    .cloned()
                    .unwrap_or_else(|| "root".into());
                if let Some(g) = groups.iter_mut().find(|g| g.key == key) {
                    g.cluster = true;
                }
                group_of.insert(folder.clone(), key);
                tree.push((NodeId::Vault, folder.clone()));
            }
            tree.push((folder, note));
        }

        Layout {
            groups,
            group_of,
            tree,
            keeps_isolates: true,
            mode: "generic folder grouping",
        }
    }

    pub fn group_key(&self, id: &NodeId) -> &str {
        self.group_of.get(id).map_or("note", |g| g.as_str())
    }

    /// Router first, then group order. Drives the growth sequence.
    pub fn rank(&self, key: &str) -> usize {
        if key == "router" {
            return 0;
        }
        self.groups.iter().position(|g| g.key == key).unwrap_or(99) + 1
    }

    pub fn add_external_group(&mut self) {
        if !self.groups.iter().any(|g| g.key == "external") {
            self.groups.push(aios_groups().pop().unwrap());
        }
    }
}

/// The group a note belongs to: its top-level folder, or the loose pile at the root.
fn bucket(note: &Note) -> (String, String) {
    match note.path.split_once('/') {
        Some((top, _)) => (top.to_string(), top.to_string()),
        None => ("root".into(), "Loose notes".into()),
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
        color,
        radius,
        glow,
        name: name.into(),
        pace,
        pause,
        major,
        cluster: false,
    }
}

fn aios_groups() -> Vec<Group> {
    let mut groups = vec![
        group("router", "#34d399", 11.0, 30.0, "Router", 0, 0, true),
        group("core", "#e7e5e4", 7.0, 16.0, "Wiki", 480, 900, true),
        group("concept", "#fbbf24", 7.5, 20.0, "Concepts", 230, 700, true),
        group("hub", "#a78bfa", 7.0, 18.0, "Suites", 270, 800, true),
        group("skill", "#60a5fa", 3.2, 0.0, "Skills", 16, 500, false),
        group("tool", "#f472b6", 5.5, 12.0, "Tools", 150, 700, true),
        group("world", "#fb923c", 5.5, 12.0, "Worlds", 150, 400, true),
        group("note", "#34d399", 5.5, 12.0, "Notes", 220, 400, true),
        group("external", "#3e4c63", 2.1, 0.0, "Files", 5, 600, false),
    ];
    groups[3].cluster = true;
    groups
}

fn aios_group_of(rel: &str) -> &'static str {
    match rel {
        "CLAUDE.md" => "router",
        _ if rel.starts_with("wiki/concepts/") => "concept",
        _ if rel.starts_with("wiki/skills/") => {
            if rel.matches('/').count() == 2 {
                "hub"
            } else {
                "skill"
            }
        }
        _ if rel.starts_with("wiki/tools/") => "tool",
        _ if rel.starts_with("wiki/worlds/") => "world",
        _ if rel.starts_with("wiki/") || rel.starts_with("cadence/") => "core",
        _ if rel.starts_with("projects/") || rel.starts_with("brainstorm/") => "note",
        _ if !rel.contains('/') => "note",
        _ => "external",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::fixture;

    #[test]
    fn aios_roles() {
        assert_eq!(aios_group_of("CLAUDE.md"), "router");
        assert_eq!(aios_group_of("wiki/skills/deploy.md"), "hub");
        assert_eq!(aios_group_of("wiki/skills/deploy/SKILL.md"), "skill");
        assert_eq!(aios_group_of("wiki/concepts/x.md"), "concept");
        assert_eq!(aios_group_of("loose.md"), "note");
        assert_eq!(aios_group_of("some/repo/readme.md"), "external");
    }

    #[test]
    fn generic_groups_folders_smallest_first() {
        let vault = fixture(
            false,
            &[
                ("a.md", ""),
                ("big/1.md", ""),
                ("big/2.md", ""),
                ("small/1.md", ""),
            ],
        );
        let layout = Layout::generic(&vault);
        let keys: Vec<&str> = layout.groups.iter().map(|g| g.key.as_str()).collect();
        assert_eq!(keys, ["router", "root", "small", "big"]);
        assert_eq!(layout.groups[1].name, "Loose notes");
        assert_eq!(layout.group_key(&NodeId::Note("big/1.md".into())), "big");
        assert_eq!(layout.group_key(&NodeId::Note("a.md".into())), "root");
        assert!(layout.keeps_isolates);
    }

    /// Groups are folders even where every note declares a `type`. An OKF bundle does,
    /// and its concepts still group by the directory they sit in; the types themselves
    /// belong in `tags`, which is the other, cross-cutting filter.
    #[test]
    fn folders_group_a_vault_that_declares_types() {
        let vault = fixture(
            false,
            &[
                ("tables/orders.md", "---\ntype: BigQuery Table\n---\n"),
                ("metrics/revenue.md", "---\ntype: Metric\n---\n"),
                ("metrics/churn.md", "---\ntype: Metric\n---\n"),
                ("scratch.md", "no frontmatter"),
            ],
        );
        let layout = Layout::generic(&vault);
        let keys: Vec<&str> = layout.groups.iter().map(|g| g.key.as_str()).collect();
        assert_eq!(keys, ["router", "tables", "root", "metrics"]);
        assert_eq!(
            layout.group_key(&NodeId::Note("metrics/churn.md".into())),
            "metrics"
        );
        assert_eq!(
            layout.group_key(&NodeId::Note("scratch.md".into())),
            "root"
        );
        assert_eq!(layout.mode, "generic folder grouping");
    }

    #[test]
    fn generic_builds_a_tree_and_marks_clusters() {
        let vault = fixture(false, &[("a.md", ""), ("ideas/1.md", "")]);
        let layout = Layout::generic(&vault);
        assert!(layout
            .tree
            .contains(&(NodeId::Vault, NodeId::Note("a.md".into()))));
        assert!(layout
            .tree
            .contains(&(NodeId::Vault, NodeId::Folder("ideas".into()))));
        assert!(layout.tree.contains(&(
            NodeId::Folder("ideas".into()),
            NodeId::Note("ideas/1.md".into())
        )));
        assert!(
            layout
                .groups
                .iter()
                .find(|g| g.key == "ideas")
                .unwrap()
                .cluster
        );
    }

    #[test]
    fn aios_has_no_tree_and_drops_isolates() {
        let vault = fixture(true, &[("CLAUDE.md", ""), ("wiki/tools/gh.md", "")]);
        let layout = Layout::aios(&vault);
        assert!(layout.tree.is_empty());
        assert!(!layout.keeps_isolates);
        assert_eq!(
            layout.group_key(&NodeId::Note("wiki/tools/gh.md".into())),
            "tool"
        );
    }

    #[test]
    fn pace_shrinks_as_a_folder_grows() {
        let notes: Vec<(String, String)> = (0..100)
            .map(|i| (format!("big/{i}.md"), String::new()))
            .collect();
        let borrowed: Vec<(&str, &str)> = notes
            .iter()
            .map(|(p, t)| (p.as_str(), t.as_str()))
            .collect();
        let layout = Layout::generic(&fixture(false, &borrowed));
        let big = layout.groups.iter().find(|g| g.key == "big").unwrap();
        assert_eq!(big.pace, 22);
        assert!(!big.major, "a folder over 60 notes renders small and fast");
    }
}
