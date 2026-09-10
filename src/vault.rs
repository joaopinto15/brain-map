//! The only module that reads the filesystem. Everything downstream is a pure
//! function over the `Vault` value it produces.

use crate::okf::Front;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

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
    /// What the frontmatter said, typed. Reading it is [`crate::okf::Front::read`]'s job.
    pub front: Front,
}

impl Note {
    fn new(path: String, text: String) -> Note {
        let front = Front::read(&crate::yaml::parse(frontmatter(&text)));
        Note { path, text, front }
    }
}

pub struct Vault {
    pub name: String,
    pub root: PathBuf,
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
            root: root.to_path_buf(),
            notes,
        }
    }

    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.notes.iter().map(|n| n.path.as_str())
    }

    pub fn note(&self, path: &str) -> Option<&Note> {
        self.notes.iter().find(|n| n.path == path)
    }

    pub fn has_note(&self, path: &str) -> bool {
        self.note(path).is_some()
    }

    /// Markdown links can point at files the scan skipped. Answering that needs the
    /// disk, so the question lives here rather than in the link resolver.
    pub fn has_file(&self, rel: &str) -> bool {
        self.root.join(rel).is_file()
    }
}

/// A note's source, for the reader panel. The path comes off the graph, so it is
/// normalized, forced to `.md`, and required to canonicalize to a file inside the
/// vault — a symlink or a `..` that climbs out gets nothing.
pub fn note_path(root: &Path, rel: &str) -> Option<PathBuf> {
    let rel = crate::node::normalize(rel);
    if !rel.ends_with(".md") {
        return None;
    }
    let file = root.join(&rel).canonicalize().ok()?;
    if !file.starts_with(root.canonicalize().ok()?) || !file.is_file() {
        return None;
    }
    Some(file)
}

pub fn read_note(root: &Path, rel: &str) -> Option<String> {
    fs::read_to_string(note_path(root, rel)?).ok()
}

/// The `---` block at the head of a note, or nothing. Only its position is decided here:
/// what is in it is YAML, and [`crate::yaml`] reads that.
fn frontmatter(text: &str) -> &str {
    let Some(rest) = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))
    else {
        return "";
    };
    let mut end = 0;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim_end();
        if trimmed == "---" || trimmed == "..." {
            return &rest[..end];
        }
        end += line.len();
    }
    ""
}

/// Every note under `dir`, with the skip rules in one place: the scan reads the files,
/// the fingerprint only stats them, and neither can drift from the other's idea of what
/// counts as part of the vault.
fn walk(dir: &Path, root: &Path, visit: &mut impl FnMut(&Path, String)) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if path.is_dir() {
            if !name.starts_with('.') && !SKIP_DIRS.contains(&name.as_str()) {
                walk(&path, root, visit);
            }
        } else if name.ends_with(".md") {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            visit(&path, rel);
        }
    }
}

/// What the vault looks like from outside: one number over every note's path, length and
/// modified time. An edit, an add, a delete and a rename all move it, and no file is read
/// to find that out, so the page can ask often.
///
/// ponytail: polled, not watched — the standard library has no filesystem notifier and
/// this is a stat walk. Reach for `notify` only if a vault ever grows big enough to feel it.
pub fn fingerprint(root: &Path) -> u64 {
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash: u64 = 0xcbf29ce484222325;
    walk(root, root, &mut |path, rel| {
        let meta = path.metadata().ok();
        let modified = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos() as u64);
        let len = meta.map_or(0, |m| m.len());
        for chunk in [
            rel.as_bytes(),
            &modified.to_le_bytes()[..],
            &len.to_le_bytes()[..],
        ] {
            for byte in chunk {
                hash ^= *byte as u64;
                hash = hash.wrapping_mul(FNV_PRIME);
            }
        }
    });
    hash
}

fn collect(dir: &Path, root: &Path, out: &mut Vec<Note>) {
    walk(dir, root, &mut |path, rel| {
        out.push(Note::new(rel, fs::read_to_string(path).unwrap_or_default()));
    });
}

#[cfg(test)]
pub fn fixture(notes: &[(&str, &str)]) -> Vault {
    Vault {
        name: "MyVault".into(),
        root: PathBuf::from("/nonexistent-brain-map-fixture"),
        notes: notes
            .iter()
            .map(|(path, text)| Note::new((*path).to_string(), (*text).to_string()))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_block_between_the_markers_is_the_frontmatter_and_nothing_else_is() {
        assert_eq!(
            frontmatter("---\ntitle: Weekly revenue\ntags: [sales]\n---\nbody"),
            "title: Weekly revenue\ntags: [sales]\n"
        );
        assert_eq!(
            frontmatter("---\r\ntitle: A\r\n...\r\nbody"),
            "title: A\r\n"
        );
        assert_eq!(
            frontmatter("# Note\n\n---\ntitle: Revenue\n---\n"),
            "",
            "a rule further down is a rule, not frontmatter"
        );
        assert_eq!(
            frontmatter("---\ntitle: never closed\n"),
            "",
            "an open block is none"
        );
        assert_eq!(frontmatter("no frontmatter"), "");
    }

    #[test]
    fn a_note_carries_what_it_declared() {
        let vault = fixture(&[(
            "m/rev.md",
            "---\ntype: Metric\ntitle: Revenue\nicon: \u{1f4ca}\ntags:\n  - sales\n  - q3\nstatus: draft\n---\n# Body\n",
        )]);
        let rev = &vault.note("m/rev.md").unwrap().front;
        assert_eq!(rev.concept.as_deref(), Some("Metric"));
        assert_eq!(rev.title.as_deref(), Some("Revenue"));
        assert_eq!(rev.icon.as_deref(), Some("\u{1f4ca}"));
        assert_eq!(rev.tags, ["sales", "q3"]);
        assert_eq!(rev.status.as_deref(), Some("draft"));
    }

    #[test]
    fn the_fingerprint_moves_for_every_kind_of_change() {
        let dir = std::env::temp_dir().join(format!("brain-map-fp-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("sub/a.md"), "one").unwrap();
        fs::write(dir.join("b.md"), "two").unwrap();

        let start = fingerprint(&dir);
        assert_eq!(
            start,
            fingerprint(&dir),
            "an untouched vault keeps its number"
        );

        fs::write(dir.join("c.md"), "three").unwrap();
        let added = fingerprint(&dir);
        assert_ne!(start, added, "a new note moves it");

        // Renamed, not rewritten: same content, same length, same mtime — only the path
        // changed, which is why the path is in the hash.
        fs::rename(dir.join("c.md"), dir.join("d.md")).unwrap();
        let renamed = fingerprint(&dir);
        assert_ne!(added, renamed, "a rename moves it");

        fs::write(dir.join("d.md"), "three but longer").unwrap();
        assert_ne!(renamed, fingerprint(&dir), "an edit moves it");

        fs::remove_file(dir.join("d.md")).unwrap();
        assert_eq!(
            start,
            fingerprint(&dir),
            "and undoing every change brings it back"
        );

        // What the scan ignores, the fingerprint ignores, or the page reloads forever.
        fs::create_dir_all(dir.join(".git")).unwrap();
        fs::write(dir.join(".git/HEAD.md"), "noise").unwrap();
        fs::write(dir.join("notes.txt"), "not markdown").unwrap();
        assert_eq!(
            start,
            fingerprint(&dir),
            "skipped directories and non-markdown do not count"
        );

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn read_note_refuses_to_leave_the_vault() {
        let dir = std::env::temp_dir().join(format!("brain-map-read-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("sub/note.md"), "hello").unwrap();
        fs::write(dir.parent().unwrap().join("brain-map-outside.md"), "secret").unwrap();

        assert_eq!(read_note(&dir, "sub/note.md").as_deref(), Some("hello"));
        assert!(read_note(&dir, "../brain-map-outside.md").is_none());
        assert!(read_note(&dir, "sub/../../brain-map-outside.md").is_none());
        assert!(read_note(&dir, "sub/note.txt").is_none());
        assert!(read_note(&dir, "sub").is_none());
        assert!(read_note(&dir, "sub/missing.md").is_none());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_plain_note_carries_no_frontmatter() {
        let note = Note::new("a.md".into(), "just text".into());
        assert_eq!(note.front, Front::default());
    }
}
