use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;

const PORT: u16 = 4710;
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
const PALETTE: [&str; 12] = [
    "#60a5fa", "#fbbf24", "#f472b6", "#a78bfa", "#fb923c", "#22d3ee", "#f87171", "#4ade80",
    "#e879f9", "#facc15", "#94a3b8", "#2dd4bf",
];

#[derive(Clone)]
struct Group {
    key: String,
    color: &'static str,
    radius: f64,
    glow: f64,
    name: String,
    pace: u32,
    pause: u32,
    major: bool,
    cluster: bool,
}

#[allow(clippy::too_many_arguments)] // group style table, not an API
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

/// Group styles for an AI Workshop OS vault (CLAUDE.md + wiki/).
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

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    if !root.is_dir() {
        eprintln!("not a directory: {}", root.display());
        std::process::exit(1);
    }

    let addr = format!("127.0.0.1:{PORT}");
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("bind {addr}: {e}");
            std::process::exit(1);
        }
    };
    let url = format!("http://{addr}");
    println!("brain-map: serving {} at {url}", root.display());
    let _ = Command::new("xdg-open").arg(&url).spawn();

    // ponytail: single-threaded, rescan per request so a browser refresh picks up new notes
    for mut stream in listener.incoming().flatten() {
        let mut request = String::new();
        if BufReader::new(&stream).read_line(&mut request).is_err() {
            continue;
        }
        let response = if request.starts_with("GET / ") {
            let graph = build_graph(&root);
            println!(
                "  {} nodes · {} links · {}",
                graph.node_count, graph.link_count, graph.mode
            );
            let page = include_str!("index.html").replace("__GRAPH__", &graph.json);
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
                page.len()
            )
        } else {
            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
        };
        let _ = stream.write_all(response.as_bytes());
    }
}

fn collect_md(dir: &Path, root: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if path.is_dir() {
            if !name.starts_with('.') && !SKIP_DIRS.contains(&name.as_str()) {
                collect_md(&path, root, out);
            }
        } else if name.ends_with(".md") {
            out.push(
                path.strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
}

fn parent_of(rel: &str) -> &str {
    rel.rsplit_once('/').map_or("", |(dir, _)| dir)
}

fn stem_of(rel: &str) -> &str {
    let base = rel.rsplit_once('/').map_or(rel, |(_, base)| base);
    base.strip_suffix(".md").unwrap_or(base)
}

/// Notes named SKILL/README/INDEX carry no meaning on their own — label them by folder.
fn label_of(rel: &str) -> String {
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

fn find(hay: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    (from..hay.len().saturating_sub(needle.len() - 1))
        .find(|&i| &hay[i..i + needle.len()] == needle)
}

/// Wikilink targets `[[target|alias]]` and relative markdown links `[text](other.md)`.
fn extract_links(text: &str) -> (Vec<String>, Vec<String>) {
    let bytes = text.as_bytes();
    let (mut wiki, mut md) = (Vec::new(), Vec::new());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"[[") {
            if let Some(end) = find(bytes, i + 2, b"]]") {
                let target = text[i + 2..end]
                    .split(['|', '#'])
                    .next()
                    .unwrap_or("")
                    .trim();
                if !target.is_empty() {
                    wiki.push(target.to_string());
                }
                i = end + 2;
                continue;
            }
        }
        if bytes[i] == b'(' && i > 0 && bytes[i - 1] == b']' {
            if let Some(end) = find(bytes, i + 1, b")") {
                let target = &text[i + 1..end];
                if target.ends_with(".md")
                    && !target.contains(char::is_whitespace)
                    && !target.contains('#')
                {
                    md.push(target.to_string());
                }
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }
    (wiki, md)
}

fn normalize(path: &str) -> String {
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

struct Graph {
    json: String,
    node_count: usize,
    link_count: usize,
    mode: &'static str,
}

fn build_graph(root: &Path) -> Graph {
    let mut files = Vec::new();
    collect_md(root, root, &mut files);
    files.sort();

    let aios = root.join("CLAUDE.md").is_file() && root.join("wiki").is_dir();
    let vault_name = root
        .canonicalize()
        .unwrap_or_else(|_| root.to_path_buf())
        .file_name()
        .map_or("Vault".to_string(), |n| n.to_string_lossy().into_owned());

    let mut groups: Vec<Group> = Vec::new();
    let mut node_group: HashMap<String, String> = HashMap::new();
    let mut tree_links: HashSet<(String, String)> = HashSet::new();

    if aios {
        groups = aios_groups();
        for rel in &files {
            node_group.insert(rel.clone(), aios_group_of(rel).to_string());
        }
    } else {
        // One group per top-level folder, smallest first so the growth starts tight.
        let mut tops: Vec<(String, Vec<String>)> = Vec::new();
        for rel in &files {
            let top = rel
                .split_once('/')
                .map_or("_root", |(top, _)| top)
                .to_string();
            match tops.iter_mut().find(|(name, _)| *name == top) {
                Some((_, members)) => members.push(rel.clone()),
                None => tops.push((top, vec![rel.clone()])),
            }
        }
        tops.sort_by_key(|(_, members)| members.len());

        let mut router = aios_groups().remove(0);
        router.name = vault_name.clone();
        groups.push(router);
        for (i, (top, members)) in tops.iter().enumerate() {
            let key = if top == "_root" { "root" } else { top.as_str() };
            let big = members.len() > 60;
            groups.push(Group {
                key: key.to_string(),
                color: PALETTE[i % PALETTE.len()],
                radius: if big { 3.5 } else { 6.0 },
                glow: if big { 0.0 } else { 14.0 },
                name: if key == "root" {
                    "Loose notes".into()
                } else {
                    top.clone()
                },
                pace: (2200 / members.len().max(1)).clamp(8, 400) as u32,
                pause: 450,
                major: !big,
                cluster: false,
            });
            for rel in members {
                node_group.insert(rel.clone(), key.to_string());
            }
        }
        if node_group.contains_key("CLAUDE.md") {
            node_group.insert("CLAUDE.md".into(), "router".into());
        }

        // Structural tree (vault → folder → note) so link-free folders still form a galaxy.
        node_group.insert("__vault__".into(), "router".into());
        for rel in &files {
            match rel.split_once('/') {
                Some((top, _)) => {
                    let dir_id = format!("__dir__{top}");
                    if !node_group.contains_key(&dir_id) {
                        let key = node_group
                            .get(rel)
                            .cloned()
                            .unwrap_or_else(|| "root".into());
                        if let Some(g) = groups.iter_mut().find(|g| g.key == key) {
                            g.cluster = true;
                        }
                        node_group.insert(dir_id.clone(), key);
                        tree_links.insert(("__vault__".into(), dir_id.clone()));
                    }
                    tree_links.insert((dir_id, rel.clone()));
                }
                None => {
                    tree_links.insert(("__vault__".into(), rel.clone()));
                }
            }
        }
    }

    // Wikilinks resolve by note name, or by path when the link spells one out.
    let mut by_key: HashMap<String, String> = HashMap::new();
    for rel in &files {
        by_key
            .entry(stem_of(rel).to_lowercase())
            .or_insert_with(|| rel.clone());
        by_key
            .entry(normalize(rel).trim_end_matches(".md").to_lowercase())
            .or_insert_with(|| rel.clone());
    }

    let mut nodes: Vec<(String, String)> = Vec::new();
    let mut seen_nodes: HashSet<String> = HashSet::new();
    for (id, grp) in node_group.iter() {
        if id.starts_with("__") {
            nodes.push((id.clone(), grp.clone()));
            seen_nodes.insert(id.clone());
        }
    }
    for rel in &files {
        let grp = node_group
            .get(rel)
            .cloned()
            .unwrap_or_else(|| "note".into());
        nodes.push((rel.clone(), grp));
        seen_nodes.insert(rel.clone());
    }

    let mut links: HashSet<(String, String)> = tree_links;
    let mut external: Vec<String> = Vec::new();
    for rel in &files {
        let text = fs::read_to_string(root.join(rel)).unwrap_or_default();
        let (wiki, md) = extract_links(&text);
        for target in wiki {
            let key = normalize(target.trim().trim_end_matches(".md")).to_lowercase();
            if let Some(hit) = by_key.get(&key) {
                if hit != rel {
                    links.insert((rel.clone(), hit.clone()));
                }
            }
        }
        for target in md {
            let resolved = normalize(&format!("{}/{}", parent_of(rel), target));
            if resolved.is_empty() || !root.join(&resolved).is_file() {
                continue;
            }
            if resolved == *rel {
                continue;
            }
            if !seen_nodes.contains(&resolved) && !external.contains(&resolved) {
                external.push(resolved.clone());
            }
            links.insert((rel.clone(), resolved));
        }
    }

    if !external.is_empty() {
        if !groups.iter().any(|g| g.key == "external") {
            groups.push(aios_groups().pop().unwrap());
        }
        for id in &external {
            nodes.push((id.clone(), "external".into()));
        }
    }

    // AIOS vaults have no structural tree, so unlinked notes would float alone.
    if aios {
        let mut degree: HashMap<&String, usize> = HashMap::new();
        for (a, b) in &links {
            *degree.entry(a).or_default() += 1;
            *degree.entry(b).or_default() += 1;
        }
        nodes.retain(|(id, _)| degree.contains_key(id));
    }

    let order_of = |key: &str| groups.iter().position(|g| g.key == key).unwrap_or(99);
    nodes.sort_by(|(a_id, a_g), (b_id, b_g)| {
        let rank = |g: &str| if g == "router" { 0 } else { order_of(g) + 1 };
        (rank(a_g), parent_of(a_id), a_id).cmp(&(rank(b_g), parent_of(b_id), b_id))
    });

    let index: HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (id.as_str(), i))
        .collect();

    let node_json: Vec<String> = nodes
        .iter()
        .map(|(id, grp)| {
            let label = if id == "__vault__" {
                vault_name.clone()
            } else if let Some(dir) = id.strip_prefix("__dir__") {
                dir.to_string()
            } else {
                label_of(id)
            };
            format!(
                r#"{{"id":{},"label":{},"g":{}}}"#,
                esc(id),
                esc(&label),
                esc(grp)
            )
        })
        .collect();

    let mut edges: Vec<(usize, usize)> = links
        .iter()
        .filter_map(|(a, b)| Some((*index.get(a.as_str())?, *index.get(b.as_str())?)))
        .collect();
    edges.sort_unstable();
    edges.dedup();
    let link_json: Vec<String> = edges
        .iter()
        .map(|(s, t)| format!(r#"{{"s":{s},"t":{t}}}"#))
        .collect();

    let group_json: Vec<String> = groups
        .iter()
        .map(|g| {
            format!(
                r#"{}:{{"c":{},"r":{},"glow":{},"name":{},"pace":{},"pause":{},"major":{},"cluster":{}}}"#,
                esc(&g.key), esc(g.color), g.radius, g.glow, esc(&g.name), g.pace, g.pause, g.major, g.cluster
            )
        })
        .collect();

    Graph {
        node_count: node_json.len(),
        link_count: link_json.len(),
        mode: if aios {
            "AI Workshop OS layout"
        } else {
            "generic folder grouping"
        },
        json: format!(
            r#"{{"vault":{},"groups":{{{}}},"nodes":[{}],"links":[{}]}}"#,
            esc(&root.display().to_string()),
            group_json.join(","),
            node_json.join(","),
            link_json.join(",")
        ),
    }
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '<' => out.push_str("\\u003c"),
            c if (c as u32) < 0x20 || c == '\u{2028}' || c == '\u{2029}' => {
                out.push_str(&format!("\\u{:04x}", c as u32))
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_links() {
        let text = "see [[Alpha|the first]] and [[notes/Beta#head]] and [x](../Gamma.md) \
                    but not [y](https://example.com) or [z](other.md#anchor)";
        let (wiki, md) = extract_links(text);
        assert_eq!(wiki, ["Alpha", "notes/Beta"]);
        assert_eq!(md, ["../Gamma.md"]);
    }

    #[test]
    fn labels_and_paths() {
        assert_eq!(label_of("wiki/skills/deploy/SKILL.md"), "deploy");
        assert_eq!(label_of("ideas/Graphs.md"), "Graphs");
        assert_eq!(normalize("ideas/../notes/./A.md"), "notes/A.md");
        assert_eq!(esc("a\"b\n<c"), "\"a\\\"b\\n\\u003cc\"");
    }

    #[test]
    fn aios_grouping() {
        assert_eq!(aios_group_of("CLAUDE.md"), "router");
        assert_eq!(aios_group_of("wiki/skills/deploy.md"), "hub");
        assert_eq!(aios_group_of("wiki/skills/deploy/SKILL.md"), "skill");
        assert_eq!(aios_group_of("wiki/concepts/x.md"), "concept");
        assert_eq!(aios_group_of("loose.md"), "note");
        assert_eq!(aios_group_of("some/repo/readme.md"), "external");
    }
}
