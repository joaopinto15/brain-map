//! What a node in the graph is, and what a note path means.

/// The three kinds of node the graph holds. Folder and vault nodes only exist to
/// give link-free folders a structural tree to hang from.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum NodeId {
    Vault,
    Folder(String),
    Note(String),
}

impl NodeId {
    /// The id the browser sees. The `__` prefixes live here and nowhere else.
    pub fn node_id(&self) -> String {
        match self {
            NodeId::Vault => "__vault__".to_string(),
            NodeId::Folder(top) => format!("__dir__{top}"),
            NodeId::Note(path) => path.clone(),
        }
    }

    pub fn label(&self, vault_name: &str) -> String {
        match self {
            NodeId::Vault => vault_name.to_string(),
            NodeId::Folder(top) => top.clone(),
            NodeId::Note(path) => label_of(path),
        }
    }

    pub fn is_note(&self) -> bool {
        matches!(self, NodeId::Note(_))
    }

    /// Nodes sort by their group first, then by folder, then by id — so a folder node
    /// always lands just above the notes it holds, and the vault leads the structure it
    /// shares a group with.
    pub fn sort_key(&self) -> (String, String) {
        if matches!(self, NodeId::Vault) {
            return (String::new(), String::new());
        }
        let graph = self.node_id();
        (parent_of(&graph).to_string(), graph)
    }
}

pub fn parent_of(rel: &str) -> &str {
    rel.rsplit_once('/').map_or("", |(dir, _)| dir)
}

pub fn stem_of(rel: &str) -> &str {
    let base = rel.rsplit_once('/').map_or(rel, |(_, base)| base);
    base.strip_suffix(".md").unwrap_or(base)
}

/// Notes named SKILL/README/INDEX carry no meaning on their own — label them by folder.
pub fn label_of(rel: &str) -> String {
    let stem = stem_of(rel);
    if matches!(stem, "SKILL" | "README" | "INDEX" | "index") {
        let parent = parent_of(rel);
        let name = parent.rsplit_once('/').map_or(parent, |(_, base)| base);
        if !name.is_empty() {
            return name.to_string();
        }
    }
    stem.to_string()
}

pub fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_paths() {
        assert_eq!(label_of("wiki/skills/deploy/SKILL.md"), "deploy");
        assert_eq!(label_of("ideas/Graphs.md"), "Graphs");
        assert_eq!(normalize("ideas/../notes/./A.md"), "notes/A.md");
    }

    #[test]
    fn node_ids_and_labels() {
        assert_eq!(NodeId::Vault.node_id(), "__vault__");
        assert_eq!(NodeId::Folder("ideas".into()).node_id(), "__dir__ideas");
        assert_eq!(NodeId::Note("a/b.md".into()).node_id(), "a/b.md");
        assert_eq!(NodeId::Vault.label("MyVault"), "MyVault");
        assert_eq!(NodeId::Folder("ideas".into()).label("MyVault"), "ideas");
        assert_eq!(NodeId::Note("a/README.md".into()).label("MyVault"), "a");
    }

    #[test]
    fn folder_nodes_sort_above_their_notes() {
        assert!(
            NodeId::Folder("ideas".into()).sort_key()
                < NodeId::Note("ideas/a.md".into()).sort_key()
        );
    }
}
