mod graph;
mod layout;
mod links;
mod node;
mod vault;

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;

const PORT: u16 = 4710;

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
            let graph = graph::build(&root);
            println!(
                "  {} nodes · {} links · {}",
                graph.nodes.len(),
                graph.links.len(),
                graph.mode
            );
            let page = include_str!("index.html").replace("__GRAPH__", &graph.to_json());
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
