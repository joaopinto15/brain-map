//! The vault as a folder tree: what the explorer shows down the left.

/// A folder. Sub-folders keep the order the notes arrived in, which is the order the
/// graph itself is in.
#[derive(Default)]
pub struct Dir {
    pub dirs: Vec<(String, Dir)>,
    /// Node indices, so a row can open the note and fly the camera to it.
    pub files: Vec<usize>,
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
    fn a_row_points_back_at_its_node() {
        let ids = ids();
        let root = build(&ids);
        assert_eq!(ids[root.dirs[0].1.files[0]], "ideas/a.md");
    }
}
