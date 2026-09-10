# brain-map

Turns a folder of markdown notes into a knowledge graph you can walk. Point it at a
vault, watch it assemble itself node by node, then read, search and edit from the window.

![How brain-map works: pick a vault, it draws the graph, you read and search and edit from the window](docs/overview.svg)

## Install it

```sh
yay -S brain-map-bin                                 # Arch, from the AUR
nix profile install github:joaopinto15/brain-map     # anywhere Nix runs
```

Or run it without installing anything:

```sh
nix run github:joaopinto15/brain-map              # pick the vault in the window
nix run github:joaopinto15/brain-map -- ~/notes   # or name it and skip the picker
```

Linux only for now. The window opens on Wayland or X11, and nobody has packaged a
Windows or macOS build.

## Open a vault

Start it with no arguments and the picker asks for one. Or name it:

```sh
brain-map ~/notes
```

The same field takes a git URL or an rclone drive, and brain-map fetches it into the
cache first. See [importing a vault](docs/importing-a-vault.md).

## Walk it

Drag to pan, scroll to zoom, click a node to read that note. The file tree on the left
takes vi's motions: `j` and `k` move, `h` closes a folder, `l` or `Enter` opens one or
reads the note. `/` searches, `Esc` lets go of everything.

The rest of the keys, and the settings file, are in
[keys and settings](docs/keys-and-settings.md).

## What it reads

Any folder of `.md` files. brain-map reads a vault as an
[Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md)
bundle, which means it takes each note as it finds it, so an Obsidian vault or a pile of
unlabelled notes works the same as a bundle somebody curated.

A note's frontmatter decides how it draws:

```yaml
---
type: Metric
title: Gross margin
icon: 📊
tags: [finance, margin]
status: stable
verified: { by: human:jp, at: 2026-06-25T09:00:00Z }
---
```

`type:` gives the node its colour and puts it in the legend. `tags:` and the trust
signals are the other two ways the legend cuts the graph, and clicking any row dims
everything else. `icon:` is the only thing that puts an emoji on a note. Every key is
optional.

Edges come from `[[wikilinks]]` and markdown links. A vault with no links still draws as
a tree.

For the full set of keys, what the reader shows, and which directories the scan skips,
see [what brain-map reads](docs/what-it-reads.md).

## Your notes stay yours

One window, one process. No browser, no server, no port. brain-map reads the folder off
the disk and draws it, so nothing is sent anywhere and nothing is listening.

## Working on it

```sh
nix develop . --command cargo test --workspace   # every crate
nix develop . --command cargo run -- ~/notes     # the window, on a vault
```

Rust the whole way down, three crates, and one trait between the half that reads the disk
and the half that draws. [How brain-map is put together](docs/architecture.md) has the
rest, and the words the project uses for things are in [CONTEXT.md](CONTEXT.md).

## License

MIT. See [LICENSE](LICENSE).
