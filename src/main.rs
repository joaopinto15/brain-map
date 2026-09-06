//! brain-map: a folder of markdown notes as a graph you can walk.
//!
//! This half reads the disk and nothing else. `vault.rs` → `layout.rs` / `links.rs` →
//! `graph.rs` builds a `Graph`, and [`Disk`] is the whole of what the window may ask for:
//! six methods that were six HTTP routes when the page ran in a browser.

mod graph;
mod layout;
mod links;
mod node;
mod vault;

use brain_map_model::{Graph, Source};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The vault, as it is on disk.
struct Disk;

impl Source for Disk {
    fn scan(&self, vault: &Path) -> Graph {
        let graph = graph::build(vault);
        println!(
            "brain-map: {} nodes · {} links · {}",
            graph.nodes.len(),
            graph.links.len(),
            graph.mode
        );
        graph.into_graph()
    }

    fn fingerprint(&self, vault: &Path) -> u64 {
        vault::fingerprint(vault)
    }

    fn read_note(&self, vault: &Path, rel: &str) -> Option<String> {
        vault::read_note(vault, rel)
    }

    fn edit(&self, vault: &Path, rel: &str) {
        match vault::note_path(vault, rel) {
            Some(file) => open_in_editor(&file),
            None => eprintln!("brain-map: no such note: {rel}"),
        }
    }

    fn choose_folder(&self) -> Result<Option<String>, String> {
        choose_folder()
    }

    fn open_vault(&self, typed: &str) -> Result<PathBuf, String> {
        open_vault(typed)
    }
}

fn main() {
    // A path on the command line skips the picker; flags are not it.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let vault = match args.iter().find(|a| !a.starts_with("--")) {
        Some(arg) => match open_vault(arg) {
            Ok(dir) => Some(dir),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        },
        None => None,
    };
    match &vault {
        Some(dir) => println!("brain-map: opening {}", dir.display()),
        None => println!("brain-map: pick a vault in the window"),
    }
    if let Err(why) = brain_map_app::run(Box::new(Disk), vault) {
        eprintln!("brain-map: {why}");
        std::process::exit(1);
    }
}

/// The desktop's own folder chooser, since typing a path is not how anyone finds one.
/// The first program that is installed wins; `Ok(None)` means the dialog was cancelled.
fn choose_folder() -> Result<Option<String>, String> {
    const DIALOGS: [(&str, &[&str]); 4] = [
        (
            "zenity",
            &["--file-selection", "--directory", "--title=Open a vault"],
        ),
        ("kdialog", &["--getexistingdirectory", "."]),
        ("qarma", &["--file-selection", "--directory"]),
        (
            "osascript",
            &[
                "-e",
                "POSIX path of (choose folder with prompt \"Open a vault\")",
            ],
        ),
    ];
    for (program, args) in DIALOGS {
        let Ok(done) = Command::new(program).args(args).output() else {
            continue; // not installed, try the next one
        };
        // What it printed decides, not how it exited: a dialog that names a folder has
        // chosen one whatever its status, and a cancelled one prints nothing.
        let chosen = String::from_utf8_lossy(&done.stdout).trim().to_string();
        let chosen = chosen
            .strip_prefix("file://")
            .unwrap_or(&chosen)
            .to_string();
        if !chosen.is_empty() {
            return Ok(Some(chosen));
        }
        let complaint = String::from_utf8_lossy(&done.stderr).trim().to_string();
        return match done.status.success() || complaint.is_empty() {
            true => Ok(None), // cancelled
            false => Err(format!("{program}: {complaint}")),
        };
    }
    Err("no folder dialog found — install zenity or kdialog, or type the path".into())
}

/// A typed path becomes a vault: `~` and `$HOME` expand, and it has to be a directory.
fn open_vault(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("no path given".into());
    }
    let expanded = match std::env::var("HOME") {
        Ok(home) if trimmed == "~" => home,
        Ok(home) if trimmed.starts_with("~/") => format!("{home}{}", &trimmed[1..]),
        Ok(home) if trimmed.starts_with("$HOME") => format!("{home}{}", &trimmed[5..]),
        _ => trimmed.to_string(),
    };
    let dir = PathBuf::from(&expanded);
    if !dir.is_dir() {
        return Err(format!("not a directory: {expanded}"));
    }
    dir.canonicalize().map_err(|e| format!("{expanded}: {e}"))
}

/// Terminal emulators that take the command to run after this flag. `$TERMINAL` wins when
/// it is set; anything else on this list is tried in order.
const TERMINALS: [(&str, &[&str]); 7] = [
    // The spec launcher first: it opens whichever terminal the desktop is set to.
    ("xdg-terminal-exec", &["--"]),
    ("alacritty", &["-e"]),
    ("ghostty", &["-e"]),
    ("kitty", &[]),
    ("foot", &[]),
    ("wezterm", &["start", "--"]),
    ("xterm", &["-e"]),
];

/// A terminal editor needs a terminal of its own, or it takes over the one running
/// brain-map. `$VISUAL` is the escape hatch for an editor that opens its own window.
fn open_in_editor(file: &Path) {
    let value = |name| std::env::var(name).ok().filter(|v| !v.trim().is_empty());
    let Some(argv) = editor_command(
        value("VISUAL").as_deref(),
        value("EDITOR").as_deref(),
        terminal().as_ref().map(|(t, args)| (t.as_str(), *args)),
        on_path("setsid"),
        file,
    ) else {
        eprintln!("  set $EDITOR, or $VISUAL for an editor that opens its own window");
        return;
    };
    // Its own session and no stdio of ours, so nothing it draws lands in this terminal
    // and Ctrl-C here does not take it down.
    let spawned = Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    if let Err(e) = spawned {
        eprintln!("  {}: {e}", argv.join(" "));
    }
}

/// The command line that opens `file`, or `None` when no editor is configured.
fn editor_command(
    visual: Option<&str>,
    editor: Option<&str>,
    terminal: Option<(&str, &[&str])>,
    detach: bool,
    file: &Path,
) -> Option<Vec<String>> {
    // `$EDITOR` is a command line, not a program name, so `code -w` and `nvim` both work.
    let (command, windowed) = match (visual, editor) {
        (Some(v), _) => (v, true),
        (None, Some(e)) => (e, false),
        (None, None) => return None,
    };
    let mut argv: Vec<String> = Vec::new();
    if detach {
        argv.extend(["setsid".to_string(), "-f".to_string()]);
    }
    // Without a terminal to host it there is nothing to do but run it as it is.
    if let (false, Some((program, args))) = (windowed, terminal) {
        argv.push(program.to_string());
        argv.extend(args.iter().map(|a| a.to_string()));
    }
    argv.extend(command.split_whitespace().map(str::to_string));
    argv.push(file.display().to_string());
    Some(argv)
}

fn terminal() -> Option<(String, &'static [&'static str])> {
    if let Some(named) = std::env::var("TERMINAL")
        .ok()
        .filter(|t| !t.trim().is_empty())
    {
        let args = TERMINALS
            .iter()
            .find(|(t, _)| named.ends_with(t))
            .map_or(&["-e"] as &[&str], |(_, args)| *args);
        return Some((named, args));
    }
    TERMINALS
        .iter()
        .find(|(program, _)| on_path(program))
        .map(|(program, args)| (program.to_string(), *args))
}

fn on_path(program: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(program).is_file()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_editor_gets_a_terminal_of_its_own() {
        let note = Path::new("/vault/a note.md");
        let term = Some(("alacritty", &["-e"] as &[&str]));

        assert_eq!(
            editor_command(None, Some("nvim"), term, true, note),
            Some(
                vec![
                    "setsid",
                    "-f",
                    "alacritty",
                    "-e",
                    "nvim",
                    "/vault/a note.md"
                ]
                .into_iter()
                .map(String::from)
                .collect()
            ),
            "a terminal editor is hosted and detached"
        );
        assert_eq!(
            editor_command(None, Some("nvim"), term, false, note).unwrap()[0],
            "alacritty",
            "without setsid it still gets its terminal"
        );
        assert_eq!(
            editor_command(Some("code -w"), Some("nvim"), term, false, note),
            Some(
                vec!["code", "-w", "/vault/a note.md"]
                    .into_iter()
                    .map(String::from)
                    .collect()
            ),
            "$VISUAL opens its own window, so no terminal is wrapped around it"
        );
        assert_eq!(
            editor_command(None, Some("nvim"), None, false, note),
            Some(
                vec!["nvim", "/vault/a note.md"]
                    .into_iter()
                    .map(String::from)
                    .collect()
            ),
            "no terminal found: run it as it is rather than not at all"
        );
        assert_eq!(
            editor_command(None, None, term, true, note),
            None,
            "nothing configured"
        );

        let xdg = TERMINALS[0];
        assert_eq!(xdg.0, "xdg-terminal-exec");
        assert_eq!(
            editor_command(None, Some("nvim"), Some((xdg.0, xdg.1)), false, note),
            Some(
                vec!["xdg-terminal-exec", "--", "nvim", "/vault/a note.md"]
                    .into_iter()
                    .map(String::from)
                    .collect()
            ),
            "it takes the command after --, not after -e"
        );
    }

    #[test]
    fn the_folder_dialog_reports_what_the_program_said() {
        let dir = std::env::temp_dir().join("brain-map-dialog-test");
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        let stub = dir.join("bin/zenity");
        std::fs::write(&stub, "#!/bin/sh\necho /tmp\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let path = std::env::var("PATH").unwrap_or_default();
        // Serialised by construction: this is the only test that touches PATH.
        unsafe { std::env::set_var("PATH", format!("{}:{path}", dir.join("bin").display())) };
        assert_eq!(
            choose_folder(),
            Ok(Some("/tmp".to_string())),
            "the chosen folder"
        );

        std::fs::write(&stub, "#!/bin/sh\nexit 1\n").unwrap();
        assert_eq!(
            choose_folder(),
            Ok(None),
            "a cancelled dialog picks nothing"
        );
        unsafe { std::env::set_var("PATH", path) };
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_vault_is_a_directory_that_exists() {
        let home = std::env::var("HOME").unwrap();
        // Asserted through the error, so the test holds where $HOME does not exist.
        assert!(
            open_vault("~/not-a-real-vault")
                .unwrap_err()
                .contains(&home),
            "~ expands"
        );
        assert!(
            open_vault("$HOME/not-a-real-vault")
                .unwrap_err()
                .contains(&home),
            "$HOME expands"
        );
        assert_eq!(
            open_vault(" /tmp ").unwrap(),
            PathBuf::from("/tmp").canonicalize().unwrap()
        );
        assert!(open_vault("").unwrap_err().contains("no path"));
        assert!(open_vault("/tmp/nothing-is-here-4710")
            .unwrap_err()
            .contains("not a directory"));
        assert!(
            open_vault("/etc/hostname")
                .unwrap_err()
                .contains("not a directory"),
            "a file is not a vault"
        );
    }
}
