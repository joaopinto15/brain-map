//! The legend and what clicking it does. A group holds every note in it, a tag only the
//! notes that declare it — and the lit set the filter produces is the same one the
//! click/hover focus uses, so there is only ever one way to dim the graph.
//!
//! The picker's search of the remembered vaults is here too: it narrows a list by what
//! was typed, which is the same job, and it is tested the same way — without a window.

use crate::sim::Node;

#[derive(Clone, Debug, PartialEq)]
pub enum Filter {
    Group(String),
    Tag(String),
}

impl Filter {
    pub fn key(&self) -> &str {
        match self {
            Filter::Group(key) | Filter::Tag(key) => key,
        }
    }
}

/// A bare word is a search of the vaults you have opened; anything carrying a separator
/// is a location to open, so typing a path is never hijacked by a vault whose name
/// happens to contain it.
pub fn is_search(typed: &str) -> bool {
    !typed.is_empty() && !typed.contains(['/', '\\', ':', '~'])
}

/// The remembered vaults a typed search names, most recent first and case blind. An empty
/// search names all of them, which is the list the picker shows before anything is typed.
pub fn vault_search<'a>(recent: &'a [String], typed: &str) -> Vec<&'a str> {
    let wanted = typed.trim().to_lowercase();
    recent
        .iter()
        .filter(|path| path.to_lowercase().contains(&wanted))
        .map(String::as_str)
        .collect()
}

pub fn matches(node: &Node, filter: Option<&Filter>) -> bool {
    match filter {
        None => true,
        Some(Filter::Group(key)) => node.group == *key,
        Some(Filter::Tag(tag)) => node.tags.iter().any(|t| t == tag),
    }
}

/// Clicking the row that is already on clears it, which is what makes one row a toggle.
pub fn toggled(current: Option<&Filter>, clicked: Filter) -> Option<Filter> {
    match current == Some(&clicked) {
        true => None,
        false => Some(clicked),
    }
}

/// How many notes each group holds, in the order the groups are listed.
pub fn group_counts(nodes: &[Node], keys: &[String]) -> Vec<usize> {
    keys.iter()
        .map(|key| nodes.iter().filter(|n| n.group == *key).count())
        .collect()
}

/// The busiest tags, most used first. The legend shows this many and no more.
pub fn top_tags(nodes: &[Node], limit: usize) -> Vec<(String, usize)> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for node in nodes {
        for tag in &node.tags {
            match counts.iter_mut().find(|(t, _)| t == tag) {
                Some((_, n)) => *n += 1,
                None => counts.push((tag.clone(), 1)),
            }
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    counts.truncate(limit);
    counts
}

/// The vault watcher reloads when the fingerprint moves. The first answer is only a
/// baseline — treating it as a change would reload the page forever.
pub fn changed<T: PartialEq + ?Sized>(mark: Option<&T>, now: &T) -> bool {
    mark.is_some_and(|mark| mark != now)
}

#[cfg(test)]
mod search_tests {
    use super::*;

    #[test]
    fn a_word_searches_and_a_path_opens() {
        let recent = vec![
            "/home/me/uni/notes".to_string(),
            "/home/me/notes".to_string(),
            "/home/me/work".to_string(),
        ];
        assert_eq!(
            vault_search(&recent, "UNI"),
            vec!["/home/me/uni/notes"],
            "case blind, and it searches the whole path"
        );
        assert_eq!(
            vault_search(&recent, "notes"),
            vec!["/home/me/uni/notes", "/home/me/notes"],
            "most recent first, which is the order the list is kept in"
        );
        assert_eq!(
            vault_search(&recent, "").len(),
            3,
            "everything, before anything is typed"
        );
        assert!(vault_search(&recent, "nothing here").is_empty());

        assert!(is_search("uni"), "a bare word");
        assert!(!is_search(""), "nothing typed is not a search");
        assert!(!is_search("~/notes"), "a path is a location");
        assert!(!is_search("/home/me/notes"));
        assert!(!is_search("gdrive:notes"), "a drive is a location");
        assert!(
            !is_search("https://github.com/you/notes"),
            "so is a URL, or its host would find a vault instead"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brain_map_model::{Graph, Group, Node as GraphNode};

    fn nodes() -> Vec<Node> {
        let mut graph = Graph {
            groups: vec![Group {
                key: "ideas".into(),
                color: "#60a5fa".into(),
                radius: 6.0,
                glow: 0.0,
                name: "ideas".into(),
                pace: 60,
                pause: 450,
                major: true,
                cluster: false,
            }],
            ..Graph::default()
        };
        graph.groups.push(Group {
            key: "daily".into(),
            ..graph.groups[0].clone()
        });
        for (id, group, tags) in [
            ("ideas/a.md", "ideas", vec!["research", "shared"]),
            ("ideas/b.md", "ideas", vec!["research"]),
            ("daily/c.md", "daily", vec!["shared"]),
            ("daily/d.md", "daily", vec![]),
        ] {
            graph.nodes.push(GraphNode {
                id: id.into(),
                label: id.into(),
                group: group.into(),
                tags: tags.into_iter().map(String::from).collect(),
                icon: String::new(),
            });
        }
        crate::sim::Sim::new(&graph, |_| 30.0, 1).nodes
    }

    #[test]
    fn a_group_lights_its_notes_and_a_tag_cuts_across_them() {
        let nodes = nodes();
        let lit = |f: Option<&Filter>| nodes.iter().filter(|n| matches(n, f)).count();
        assert_eq!(lit(Some(&Filter::Group("ideas".into()))), 2);
        assert_eq!(lit(Some(&Filter::Tag("research".into()))), 2);
        assert_eq!(
            lit(Some(&Filter::Tag("shared".into()))),
            2,
            "tags cross folders"
        );
        assert_eq!(lit(None), 4, "no filter lights everything");
    }

    #[test]
    fn clicking_the_same_row_again_clears_it() {
        let on = Filter::Tag("research".into());
        assert_eq!(toggled(Some(&on), on.clone()), None);
        assert_eq!(toggled(None, on.clone()), Some(on.clone()));
        assert_eq!(
            toggled(Some(&Filter::Group("ideas".into())), on.clone()),
            Some(on),
            "a different row replaces the filter"
        );
    }

    #[test]
    fn the_legend_counts_what_it_lists() {
        let nodes = nodes();
        assert_eq!(
            group_counts(&nodes, &["ideas".into(), "daily".into()]),
            [2, 2]
        );
        assert_eq!(
            top_tags(&nodes, 12),
            [("research".to_string(), 2), ("shared".to_string(), 2)]
        );
        assert_eq!(
            top_tags(&nodes, 1).len(),
            1,
            "the legend stops at its limit"
        );
    }

    #[test]
    fn the_watcher_reloads_only_after_it_has_a_baseline() {
        assert!(!changed(None, "7"), "the first answer is the baseline");
        assert!(!changed(Some("7"), "7"), "an unchanged vault stays put");
        assert!(changed(Some("7"), "9"), "a changed vault reloads");
        assert!(
            !changed(Some("0"), "0"),
            "a vault with no notes is still a vault"
        );
    }
}
