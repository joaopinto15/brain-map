//! The graph, defined once for both halves of the program.
//!
//! The scanner builds a [`Graph`] out of a vault and the page draws it. Nothing here
//! knows about the filesystem or the window, so a field can only mean one thing on
//! either side of [`Source`].

use std::path::{Path, PathBuf};

/// A colour group: one OKF concept type, plus the structural tree and the external
/// files that are not concepts at all.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub key: String,
    pub color: String,
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

/// One note, folder or vault node. `id` is the node id: a note's path, or a
/// `__vault__` / `__dir__` prefixed key for the structural nodes.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub group: String,
    pub tags: Vec<String>,
    /// What the note's `icon:` said, or empty. Nothing derives this: an icon is written
    /// in the note or the node draws as a plain disc.
    pub icon: String,
    /// The OKF trust and lifecycle signals the concept declared: its trust tier, a status
    /// that is not the default, and `stale` once it is. Derived in the scanner, where the
    /// clock and the frontmatter both are, so the window only ever reads them.
    pub signals: Vec<String>,
    /// The rest of what the concept said about itself, for the reader to show.
    pub concept: Concept,
}

/// What a concept's frontmatter says about itself beyond its type, tags and icon
/// (OKF §4.1, §5): the prose, the asset it describes, who wrote and confirmed it, and
/// what it derives from. Every field is optional in the format and empty here when absent.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Concept {
    pub description: String,
    /// The canonical URI of the asset the concept describes, or empty for an idea.
    pub resource: String,
    pub generated: Option<Actor>,
    pub verified: Vec<Actor>,
    pub sources: Vec<Provenance>,
}

/// Who did something and when, in OKF's actor convention (§7): `human:<id>`,
/// `process:<id>` or `<producer>/<version>`, and an ISO 8601 instant, both as written.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Actor {
    pub by: String,
    pub at: String,
}

/// One `sources` entry (§5.1): a material the concept derives from, named by a URL, a
/// bundle path, or a scope descriptor a consumer cannot follow.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Provenance {
    pub id: String,
    pub title: String,
    pub resource: String,
}

/// An edge, by node index.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Link {
    pub s: usize,
    pub t: usize,
}

/// Everything the page needs to draw a vault.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Graph {
    /// The vault's path, or empty when no vault has been chosen yet.
    pub vault: String,
    /// In order: the legend lists groups this way and the palette indexes them by it.
    pub groups: Vec<Group>,
    pub nodes: Vec<Node>,
    pub links: Vec<Link>,
}

impl Graph {
    /// The group a node belongs to. Groups are few, so the scan costs less than a map.
    pub fn group(&self, key: &str) -> Option<&Group> {
        self.groups.iter().find(|g| g.key == key)
    }
}

/// A node the page draws no note for: the vault node and the folder nodes.
pub fn is_structural(id: &str) -> bool {
    id.starts_with("__")
}

/// Where the graph comes from, and the things the page cannot do for itself.
///
/// Six of these were once six HTTP routes and the page fetched them. The page is a window
/// now and calls them directly, but the split they draw is the same one: the page asks,
/// the scanner answers, and neither side can see the other's insides. Everything that
/// touches the disk or the desktop is on this side of the line, which is why the drives
/// joined it rather than the picker learning to run a program.
pub trait Source: Send + Sync {
    /// The whole vault, rescanned. `/graph.json` was this.
    fn scan(&self, vault: &Path) -> Graph;
    /// One number over every note's path, length and mtime. It moving means reload.
    fn fingerprint(&self, vault: &Path) -> u64;
    /// A note's source text, or `None` when it is not a note inside this vault.
    fn read_note(&self, vault: &Path, rel: &str) -> Option<String>;
    /// Hand a note to `$EDITOR`.
    fn edit(&self, vault: &Path, rel: &str);
    /// The desktop's folder chooser. `Ok(None)` means the dialog was cancelled.
    fn choose_folder(&self) -> Result<Option<String>, String>;
    /// A typed path, git URL or drive becomes a vault, or says why it does not.
    fn open_vault(&self, typed: &str) -> Result<PathBuf, String>;
    /// The drives that are configured to import from, `remote:` each. Empty when there
    /// is no rclone, which is what hides them from the picker.
    fn drives(&self) -> Vec<String>;
}
