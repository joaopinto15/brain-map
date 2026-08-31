mod graph;
mod icons;
mod layout;
mod links;
mod node;
mod page;
mod vault;

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const PORT: u16 = 4710;

/// The graph the page gets before a vault is chosen: enough for the script to run,
/// empty enough for it to show the picker instead of a canvas.
const NO_VAULT: &str = r#"{"vault":"","groups":{},"icons":{},"nodes":[],"links":[]}"#;

fn main() {
    // A path on the command line skips the picker; flags are not it.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = match args.iter().find(|a| !a.starts_with("--")) {
        Some(arg) => match open_vault(arg) {
            Ok(dir) => Some(dir),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        },
        None => None,
    };

    let addr = format!("127.0.0.1:{PORT}");
    // Bound here rather than in the server thread, so the socket is already listening
    // by the time the window asks for the page.
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("bind {addr}: {e}");
            std::process::exit(1);
        }
    };
    // Only a page served by this process knows the token, so another site cannot
    // point the picker at a directory of its own choosing.
    let token = format!("{:x}", nanos());
    let url = format!("http://{addr}");
    match &root {
        Some(dir) => println!("brain-map: serving {} at {url}", dir.display()),
        None => println!("brain-map: {url} — pick a vault in the page"),
    }
    let server = std::thread::spawn(move || serve(listener, root, token));

    // A window of our own when this machine can make one, the browser when it cannot.
    // Both are served the same page off the same socket.
    if let Err(why) = try_window(&args, &url) {
        println!("brain-map: {why} — opening a browser instead");
        let _ = Command::new("xdg-open").arg(&url).spawn();
        let _ = server.join();
    }
}

/// The page in a window this process owns. Returns only when there is no window to be
/// had — the event loop never gives the thread back.
fn try_window(args: &[String], url: &str) -> Result<(), String> {
    use tao::event::{Event, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoop};
    use tao::window::WindowBuilder;
    use wry::WebViewBuilder;

    let env = |name| std::env::var(name).ok().filter(|v| !v.is_empty());
    if let Some(why) = browser_reason(args, env("WAYLAND_DISPLAY"), env("DISPLAY")) {
        return Err(why.into());
    }

    // Every failure below is a reason to fall back, not to die: a machine can have a
    // display and still have no webview runtime behind it.
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("brain-map")
        .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 800.0))
        .build(&event_loop)
        .map_err(|e| format!("no window: {e}"))?;
    let builder = WebViewBuilder::new()
        .with_url(url)
        .with_devtools(cfg!(debug_assertions));
    // WebKitGTK attaches to the window's own GTK container, not to a raw handle.
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        match window.default_vbox() {
            Some(vbox) => builder.build_gtk(vbox),
            None => return Err("no gtk container".into()),
        }
    };
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let webview = builder.build(&window);
    let _webview = webview.map_err(|e| format!("no webview: {e}"))?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent { event: WindowEvent::CloseRequested, .. } = event {
            *control_flow = ControlFlow::Exit;
        }
    });
}

/// Why the browser should be used instead of a window, or `None` to try for one.
fn browser_reason(args: &[String], wayland: Option<String>, x11: Option<String>) -> Option<&'static str> {
    if args.iter().any(|a| a == "--browser") {
        return Some("--browser");
    }
    match wayland.is_none() && x11.is_none() {
        true => Some("no display"),
        false => None,
    }
}

fn serve(listener: TcpListener, mut root: Option<PathBuf>, token: String) {
    // ponytail: single-threaded, rescan per request so a browser refresh picks up new notes
    for mut stream in listener.incoming().flatten() {
        let mut request = String::new();
        if BufReader::new(&stream).read_line(&mut request).is_err() {
            continue;
        }
        let response = if let Some(query) = path_request(&request, "GET /open?") {
            match query_field(query, "t").as_deref() == Some(token.as_str()) {
                false => text(403, "not this page's token"),
                true => match open_vault(&percent_decode(&query_field(query, "path").unwrap_or_default())) {
                    Ok(dir) => {
                        println!("brain-map: vault is now {}", dir.display());
                        root = Some(dir);
                        text(200, "ok")
                    }
                    Err(e) => text(400, &e),
                },
            }
        } else if let Some(query) = path_request(&request, "GET /browse?") {
            match query_field(query, "t").as_deref() == Some(token.as_str()) {
                false => text(403, "not this page's token"),
                true => match choose_folder() {
                    Ok(Some(dir)) => {
                        println!("brain-map: dialog chose {dir}");
                        text(200, &dir)
                    }
                    Ok(None) => {
                        println!("brain-map: dialog cancelled");
                        text(204, "")
                    }
                    Err(e) => {
                        eprintln!("brain-map: {e}");
                        text(501, &e)
                    }
                },
            }
        } else if let Some(query) = path_request(&request, "GET /note?path=") {
            match root.as_deref().and_then(|r| vault::read_note(r, &percent_decode(query))) {
                Some(text) => format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                    text.len()
                ),
                None => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
            }
        } else if let Some(query) = path_request(&request, "GET /edit?path=") {
            match root.as_deref().and_then(|r| vault::note_path(r, &percent_decode(query))) {
                Some(file) => {
                    open_in_editor(&file);
                    "HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n".to_string()
                }
                None => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
            }
        } else if request.starts_with("GET /changed ") {
            // One number over the vault's notes. The page holds the last one it saw and
            // reloads when it moves, which is the whole of live reload.
            text(200, &root.as_deref().map_or(0, vault::fingerprint).to_string())
        } else if request.starts_with("GET / ") {
            let json = match &root {
                Some(dir) => {
                    let graph = graph::build(dir);
                    println!(
                        "  {} nodes · {} links · {}",
                        graph.nodes.len(),
                        graph.links.len(),
                        graph.mode
                    );
                    graph.to_json()
                }
                None => NO_VAULT.to_string(),
            };
            let page = page::render(&json, &token);
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

fn nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos())
}

fn text(status: u16, body: &str) -> String {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        403 => "Forbidden",
        501 => "Not Implemented",
        _ => "Error",
    };
    format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

/// The desktop's own folder chooser, since a browser will not hand back a real path.
/// The first program that is installed wins; `Ok(None)` means the dialog was cancelled.
fn choose_folder() -> Result<Option<String>, String> {
    const DIALOGS: [(&str, &[&str]); 4] = [
        ("zenity", &["--file-selection", "--directory", "--title=Open a vault"]),
        ("kdialog", &["--getexistingdirectory", "."]),
        ("qarma", &["--file-selection", "--directory"]),
        ("osascript", &["-e", "POSIX path of (choose folder with prompt \"Open a vault\")"]),
    ];
    for (program, args) in DIALOGS {
        let Ok(done) = Command::new(program).args(args).output() else {
            continue; // not installed, try the next one
        };
        // What it printed decides, not how it exited: a dialog that names a folder has
        // chosen one whatever its status, and a cancelled one prints nothing.
        let chosen = String::from_utf8_lossy(&done.stdout).trim().to_string();
        let chosen = chosen.strip_prefix("file://").unwrap_or(&chosen).to_string();
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

/// `path=~/notes&t=9f2` — one still-encoded field out of a query string.
fn query_field(query: &str, name: &str) -> Option<String> {
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
        .map(str::to_string)
}

/// `GET /note?path=… HTTP/1.1` — the still-encoded path out of a request line.
fn path_request<'a>(request: &'a str, route: &str) -> Option<&'a str> {
    request.strip_prefix(route)?.split_whitespace().next()
}

/// Terminal emulators that take the command to run after this flag. `$TERMINAL` wins
/// when it is set; anything else on this list is tried in order.
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
    if let Some(named) = std::env::var("TERMINAL").ok().filter(|t| !t.trim().is_empty()) {
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
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|dir| dir.join(program).is_file())
    })
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match u8::from_str_radix(s.get(i + 1..i + 3).unwrap_or(""), 16) {
            Ok(byte) if bytes[i] == b'%' => {
                out.push(byte);
                i += 3;
            }
            _ => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_note_path_off_the_request_line() {
        assert_eq!(
            path_request("GET /note?path=0%20Inbox/a.md HTTP/1.1\r\n", "GET /note?path=")
                .map(percent_decode),
            Some("0 Inbox/a.md".to_string())
        );
        assert_eq!(
            percent_decode("a+b%2Bc.md"),
            "a+b+c.md",
            "plus stays a plus"
        );
        assert_eq!(
            path_request("GET /edit?path=a.md HTTP/1.1\r\n", "GET /edit?path="),
            Some("a.md")
        );
        assert!(path_request("GET / HTTP/1.1\r\n", "GET /note?path=").is_none());
    }

    #[test]
    fn reads_the_fields_of_an_open_request() {
        let query = path_request("GET /open?path=%2Ftmp%2Fv&t=9f2 HTTP/1.1\r\n", "GET /open?").unwrap();
        assert_eq!(query_field(query, "t").as_deref(), Some("9f2"));
        assert_eq!(
            query_field(query, "path").map(|p| percent_decode(&p)),
            Some("/tmp/v".to_string())
        );
        assert_eq!(query_field(query, "nope"), None);
    }

    #[test]
    fn the_browser_takes_over_when_there_is_no_window_to_be_had() {
        let wayland = || Some("wayland-1".to_string());
        let x11 = || Some(":0".to_string());
        let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();

        assert_eq!(browser_reason(&args(&["~/notes"]), wayland(), None), None, "wayland is a display");
        assert_eq!(browser_reason(&args(&[]), None, x11()), None, "so is x11");
        assert_eq!(
            browser_reason(&args(&["--browser", "~/notes"]), wayland(), x11()),
            Some("--browser"),
            "asked for, so it wins over a display that works"
        );
        assert_eq!(browser_reason(&args(&["~/notes"]), None, None), Some("no display"));
    }

    #[test]
    fn an_editor_gets_a_terminal_of_its_own() {
        let note = Path::new("/vault/a note.md");
        let term = Some(("alacritty", &["-e"] as &[&str]));

        assert_eq!(
            editor_command(None, Some("nvim"), term, true, note),
            Some(vec!["setsid", "-f", "alacritty", "-e", "nvim", "/vault/a note.md"]
                .into_iter().map(String::from).collect()),
            "a terminal editor is hosted and detached"
        );
        assert_eq!(
            editor_command(None, Some("nvim"), term, false, note).unwrap()[0],
            "alacritty",
            "without setsid it still gets its terminal"
        );
        assert_eq!(
            editor_command(Some("code -w"), Some("nvim"), term, false, note),
            Some(vec!["code", "-w", "/vault/a note.md"]
                .into_iter().map(String::from).collect()),
            "$VISUAL opens its own window, so no terminal is wrapped around it"
        );
        assert_eq!(
            editor_command(None, Some("nvim"), None, false, note),
            Some(vec!["nvim", "/vault/a note.md"].into_iter().map(String::from).collect()),
            "no terminal found: run it as it is rather than not at all"
        );
        assert_eq!(editor_command(None, None, term, true, note), None, "nothing configured");

        let xdg = TERMINALS[0];
        assert_eq!(xdg.0, "xdg-terminal-exec");
        assert_eq!(
            editor_command(None, Some("nvim"), Some((xdg.0, xdg.1)), false, note),
            Some(vec!["xdg-terminal-exec", "--", "nvim", "/vault/a note.md"]
                .into_iter().map(String::from).collect()),
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
        assert_eq!(choose_folder(), Ok(Some("/tmp".to_string())), "the chosen folder");

        std::fs::write(&stub, "#!/bin/sh\nexit 1\n").unwrap();
        assert_eq!(choose_folder(), Ok(None), "a cancelled dialog picks nothing");
        unsafe { std::env::set_var("PATH", path) };
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_vault_is_a_directory_that_exists() {
        let home = std::env::var("HOME").unwrap();
        // Asserted through the error, so the test holds where $HOME does not exist.
        assert!(open_vault("~/not-a-real-vault").unwrap_err().contains(&home), "~ expands");
        assert!(open_vault("$HOME/not-a-real-vault").unwrap_err().contains(&home), "$HOME expands");
        assert_eq!(open_vault(" /tmp ").unwrap(), PathBuf::from("/tmp").canonicalize().unwrap());
        assert!(open_vault("").unwrap_err().contains("no path"));
        assert!(open_vault("/tmp/nothing-is-here-4710").unwrap_err().contains("not a directory"));
        assert!(open_vault("/etc/hostname").unwrap_err().contains("not a directory"),
            "a file is not a vault");
    }
}
