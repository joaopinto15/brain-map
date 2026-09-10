//! The vault as a folder tree: what the explorer shows down the left.
//!
//! [`rows`] is the same tree flattened to what is actually on screen — the folders that
//! are open and the notes under them, in draw order. The cursor the vim motions move is
//! an index into that list, so where `j` lands is decided here and checked without a
//! window.

/// A folder. Sub-folders keep the order the notes arrived in, which is the order the
/// graph itself is in.
#[derive(Default)]
pub struct Dir {
    pub dirs: Vec<(String, Dir)>,
    /// Node indices, so a row can open the note and fly the camera to it.
    pub files: Vec<usize>,
}

/// One line in the explorer: a folder, or a note. `depth` is how far it is indented,
/// which is what `h` walks back out through.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub depth: usize,
    pub key: Key,
}

/// What a row is. A folder is named by its path so it keeps its place when the vault is
/// rescanned; a note is a node index, the way every other part of the window names one.
#[derive(Clone, Debug, PartialEq)]
pub enum Key {
    Dir(String),
    File(usize),
}

impl Row {
    pub fn dir(&self) -> Option<&str> {
        match &self.key {
            Key::Dir(path) => Some(path),
            Key::File(_) => None,
        }
    }

    pub fn file(&self) -> Option<usize> {
        match self.key {
            Key::File(node) => Some(node),
            Key::Dir(_) => None,
        }
    }
}

/// The tree as the explorer draws it: every folder, the notes under the open ones, and
/// nothing under the closed ones. `closed` holds the folders that are shut, so a vault
/// opens with its tree open the way the explorer always has.
pub fn rows(dir: &Dir, closed: &dyn Fn(&str) -> bool) -> Vec<Row> {
    let mut out = Vec::new();
    walk(dir, "", 0, closed, &mut out);
    out
}

fn walk(dir: &Dir, path: &str, depth: usize, closed: &dyn Fn(&str) -> bool, out: &mut Vec<Row>) {
    for (name, below) in &dir.dirs {
        let here = format!("{path}/{name}");
        out.push(Row {
            depth,
            key: Key::Dir(here.clone()),
        });
        if !closed(&here) {
            walk(below, &here, depth + 1, closed, out);
        }
    }
    for &node in &dir.files {
        out.push(Row {
            depth,
            key: Key::File(node),
        });
    }
}

/// The folder a row is filed under: the nearest row above it that is less indented.
/// `h` walks out through these.
pub fn parent(rows: &[Row], at: usize) -> Option<usize> {
    let depth = rows.get(at)?.depth;
    rows[..at].iter().rposition(|row| row.depth < depth)
}

/// Every note filed under the folders of its path. Structural nodes are not files.
pub fn build(ids: &[String]) -> Dir {
    let mut root = Dir::default();
    for (i, id) in ids.iter().enumerate() {
        if brain_map_model::is_structural(id) {
            continue;
        }
        let mut dir = &mut root;
        let parts: Vec<&str> = id.split('/').collect();
        for name in &parts[..parts.len() - 1] {
            let at = match dir.dirs.iter().position(|(n, _)| n == name) {
                Some(at) => at,
                None => {
                    dir.dirs.push(((*name).to_string(), Dir::default()));
                    dir.dirs.len() - 1
                }
            };
            dir = &mut dir.dirs[at].1;
        }
        dir.files.push(i);
    }
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> Vec<String> {
        [
            "__vault__",
            "__dir__ideas",
            "loose.md",
            "ideas/a.md",
            "ideas/deep/b.md",
        ]
        .map(String::from)
        .to_vec()
    }

    #[test]
    fn the_tree_mirrors_the_vault() {
        let root = build(&ids());
        assert_eq!(root.files, [2], "a note at the root is a row of its own");
        assert_eq!(root.dirs.len(), 1);
        let (name, ideas) = &root.dirs[0];
        assert_eq!(name, "ideas");
        assert_eq!(ideas.files, [3]);
        assert_eq!(ideas.dirs[0].0, "deep");
        assert_eq!(
            ideas.dirs[0].1.files,
            [4],
            "nesting goes as deep as the path"
        );
    }

    #[test]
    fn the_rows_are_what_the_explorer_draws() {
        let root = build(&ids());
        let open = rows(&root, &|_| false);
        assert_eq!(
            open,
            vec![
                Row {
                    depth: 0,
                    key: Key::Dir("/ideas".into())
                },
                Row {
                    depth: 1,
                    key: Key::Dir("/ideas/deep".into())
                },
                Row {
                    depth: 2,
                    key: Key::File(4)
                },
                Row {
                    depth: 1,
                    key: Key::File(3)
                },
                Row {
                    depth: 0,
                    key: Key::File(2)
                },
            ],
            "folders first, then the notes beside them"
        );
        let shut = rows(&root, &|path| path == "/ideas");
        assert_eq!(
            shut,
            vec![
                Row {
                    depth: 0,
                    key: Key::Dir("/ideas".into())
                },
                Row {
                    depth: 0,
                    key: Key::File(2)
                },
            ],
            "a closed folder hides everything under it"
        );
    }

    #[test]
    fn h_walks_out_to_the_folder_a_row_is_in() {
        let rows = rows(&build(&ids()), &|_| false);
        assert_eq!(parent(&rows, 2), Some(1), "the note is in deep");
        assert_eq!(parent(&rows, 1), Some(0), "deep is in ideas");
        assert_eq!(parent(&rows, 0), None, "ideas is at the root");
        assert_eq!(parent(&rows, 4), None, "so is a loose note");
    }

    #[test]
    fn a_row_points_back_at_its_node() {
        let ids = ids();
        let root = build(&ids);
        assert_eq!(ids[root.dirs[0].1.files[0]], "ideas/a.md");
    }
}
