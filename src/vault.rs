//! The only module that reads the filesystem. Everything downstream is a pure
//! function over the `Vault` value it produces.

use std::fs;
use std::path::{Path, PathBuf};

const SKIP_DIRS: [&str; 11] = [
    ".git",
    ".obsidian",
    "node_modules",
    ".brain-map",
    "__pycache__",
    ".venv",
    "venv",
    "dist",
    "build",
    ".next",
    ".cache",
];

pub struct Note {
    pub path: String,
    pub text: String,
}

pub struct Vault {
    pub name: String,
    pub root: PathBuf,
    /// An AI Workshop OS vault: `CLAUDE.md` at the root next to a `wiki/` directory.
    pub is_aios: bool,
    pub notes: Vec<Note>,
}

impl Vault {
    pub fn scan(root: &Path) -> Vault {
        let mut notes = Vec::new();
        collect(root, root, &mut notes);
        notes.sort_by(|a, b| a.path.cmp(&b.path));
        Vault {
            name: root
                .canonicalize()
                .unwrap_or_else(|_| root.to_path_buf())
                .file_name()
                .map_or("Vault".to_string(), |n| n.to_string_lossy().into_owned()),
            is_aios: root.join("CLAUDE.md").is_file() && root.join("wiki").is_dir(),
            root: root.to_path_buf(),
            notes,
        }
    }

    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.notes.iter().map(|n| n.path.as_str())
    }

    pub fn has_note(&self, path: &str) -> bool {
        self.notes.iter().any(|n| n.path == path)
    }

    /// Markdown links can point at files the scan skipped. Answering that needs the
    /// disk, so the question lives here rather than in the link resolver.
    pub fn has_file(&self, rel: &str) -> bool {
        self.root.join(rel).is_file()
    }
}

fn collect(dir: &Path, root: &Path, out: &mut Vec<Note>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if path.is_dir() {
            if !name.starts_with('.') && !SKIP_DIRS.contains(&name.as_str()) {
                collect(&path, root, out);
            }
        } else if name.ends_with(".md") {
            out.push(Note {
                path: path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/"),
                text: fs::read_to_string(&path).unwrap_or_default(),
            });
        }
    }
}

#[cfg(test)]
pub fn fixture(is_aios: bool, notes: &[(&str, &str)]) -> Vault {
    Vault {
        name: "MyVault".into(),
        root: PathBuf::from("/nonexistent-brain-map-fixture"),
        is_aios,
        notes: notes
            .iter()
            .map(|(path, text)| Note {
                path: (*path).to_string(),
                text: (*text).to_string(),
            })
            .collect(),
    }
}
