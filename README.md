# brain-map

Turns a folder of markdown notes into an interactive knowledge graph. Point it at a
vault, watch it assemble itself node by node, then walk it.

![How brain-map works: pick a vault, it draws the graph, you read and search and edit from the window](docs/overview.svg)

## Install it

```sh
yay -S brain-map-bin                          # Arch, from the AUR
nix profile install github:joaopinto15/brain-map   # anywhere Nix runs
```

Or run it without installing:

```sh
nix run github:joaopinto15/brain-map           # pick the vault in the window
nix run github:joaopinto15/brain-map -- ~/notes   # or name it and skip the picker
```

Linux only for now — the window opens on Wayland or X11, and there is no Windows or
macOS build to package.

## Run it

```sh
nix run .              # pick the vault in the window
nix run . -- ~/notes   # or name it and skip the picker
```

## Import a vault

A vault that is not on this machine yet is the same field and the same argument as one
that is:

```sh
brain-map https://github.com/you/notes   # any git remote: https, ssh, git@host:path
brain-map gdrive:notes                   # any rclone remote: the drive, and a folder on it
```

Either one is fetched into `$XDG_CACHE_HOME/brain-map/vaults` and opened from there.
Opening it again brings down what changed, and an offline machine still opens the last
import rather than nothing.

Drives go through [rclone](https://rclone.org), which is where the accounts and the
tokens already live — `rclone config` once, and the name you gave the remote is what you
type here:

| Drive | `rclone config` type | Then |
|-------|----------------------|------|
| Google Drive | `drive` | `brain-map gdrive:notes` |
| OneDrive | `onedrive` | `brain-map onedrive:vaults/brain` |
| Dropbox, S3, Nextcloud, SFTP, … | [any of the ~70](https://rclone.org/overview/) | `brain-map remote:path` |

brain-map knows none of those providers by name: it runs `rclone copy`, so a drive rclone
supports is a drive this supports. Notes are copied, never deleted — a note you edited
from the window is not lost because the drive no longer has it — so a vault that has
shrunk on the drive is refreshed by removing its folder under the cache.

If the drive's own desktop client already syncs the vault into a local folder, open that
folder and none of this applies.

brain-map is one window and one process. There is no browser, no server and no port: it
reads the vault off the disk and draws it, so the notes never leave the machine and
nothing is listening while it does.

It reads a vault as an
[Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md)
bundle, and works on any folder of `.md` files — an OKF bundle, an Obsidian vault, a pile
of notes — because OKF asks a consumer to take a concept as it finds it. Edges come from
`[[wikilinks]]` and markdown links, relative or bundle-absolute
(`[orders](/tables/orders.md)`). A vault with no links still draws as a tree: vault →
folder → note.

## Keys

**drag** or **hjkl** pans, **scroll** zooms, **click** a node to read it. **/** focuses
the search box and **n** and **N** walk the matches. **R** replays the growth animation,
**F** gives the note the whole window, **space e** hides the explorer, **Esc** releases
whatever is held. Everything configurable lives behind **⚙ Settings**: vault, theme,
growth budget, explorer width, and whether the explorer shows.

## Types, signals and tags

The legend cuts the graph three ways, and a row of it is a filter that dims everything
else.

A concept has exactly one *type*: the `type:` it declares, `Untyped` if it declares none.
The type decides the concept's colour, its size and when it appears in the growth
animation, so the types partition the vault and their counts add up to it. Folders are
structure, not type — a bundle organizes its concepts into directories however it likes,
and the tree is drawn but never grouped on. The reserved `index.md` and `log.md` are not
concepts, so they group as *Index & log*.

*Signals* are what the frontmatter says about trusting a concept: its trust tier —
`human-reviewed` when a `human:` actor verified it, `machine-confirmed` when something
else did, `unverified` when nothing has — plus `draft` or `deprecated` when `status:`
left the default, and `stale` once `stale_after:` has passed. Every one of them is read,
never inferred.

*Tags* come from the `tags:` field. A concept can carry several or none, and they cut
across types. Filtering by a type shows one region of the map, by a signal what to trust
in it, by a tag a thread running through several.

`icon:` is one emoji and it is the only thing that puts one on a note — nothing is
inferred from a note's name, its folder or its tags, so a note without the key draws as a
plain disc.

```yaml
---
type: Metric
title: Gross margin
icon: 📊
tags: [finance, margin]
status: stable
verified: { by: human:jp, at: 2026-06-25T09:00:00Z }
stale_after: 2026-12-31T00:00:00Z
---
```

Every one of those keys is optional: OKF forbids a consumer from rejecting a concept for
what it left out, so a note with no frontmatter at all is an `Untyped`, `unverified`
concept and draws like any other.

Reading a concept shows the rest of what it declared, the way the format's own viewer
does: its `description` and `resource`, who `generated` and `verified` it, the `sources`
it derives from — a bundle path opens that concept, a URL opens outside — and, read
backwards off the graph, the concepts that cite it. The frontmatter is read as YAML: block
and flow mappings and lists, quoted and folded scalars, and a v0.1 `timestamp` still
stands in for `generated.at`.

Skipped while scanning: `.git`, `.obsidian`, `node_modules`, `.brain-map`,
`__pycache__`, `.venv`, `venv`, `dist`, `build`, `.next`, `.cache`, and any
dot-directory.

## Development

```sh
nix develop . --command cargo test --workspace   # every crate
nix develop . --command cargo run -- ~/notes     # the window, on a vault
```

It is Rust the whole way down, and no JavaScript at all — there is no web page left to
put any in. Three crates: this one scans the vault, `crates/app` is the window, and
`crates/model` is the graph they meet on. The window is `eframe`, and the graph, the note
renderer, the folder tree and the chrome are all drawn by hand against it.

The icons are the real thing rather than outlines: egui cannot draw a colour bitmap font,
so `emoji.rs` reads the PNG for a glyph out of Noto Color Emoji itself and hands egui a
texture. Install `noto-fonts-emoji` — the Nix package ships its own copy — or set
`$BRAIN_MAP_EMOJI_FONT` at a font of your own.

The split the browser drew is still there, as a trait. `Source` in `crates/model` is the
everything the window may ask for — scan the vault, fingerprint it, read a note, open one
in `$EDITOR`, show a folder dialog, list the drives, and turn what was typed into a vault
— and it is implemented once, in `src/main.rs`. Six of those were HTTP routes when the
page ran in a browser. Nothing on the window's side of that trait can touch the disk, which is why most
of `crates/app` is tested with a plain `cargo test` and no window: the force simulation,
the note renderer, link resolution, the keymap, the legend filter and the theme table all
run on the host.

The words the project uses are in [CONTEXT.md](CONTEXT.md), and the decisions that would
otherwise be surprising are in [docs/adr](docs/adr).

The vault is followed by polling, not by watching: once every two seconds the window asks
for one hash over every note's path, length and modified time, and reloads when it moves —
keeping the note that was open. There is no notifier in the standard library and a stat
walk is not worth a dependency.

The diagram above is `docs/overview.excalidraw`, drawn in
[Excalidraw](https://excalidraw.com); `docs/overview.svg` is its export.

## License

MIT — see [LICENSE](LICENSE).
