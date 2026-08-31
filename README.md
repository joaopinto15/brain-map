# brain-map

Turns a folder of markdown notes into an interactive knowledge graph. Point it at a
vault, watch it assemble itself node by node, then walk it.

![How brain-map works: pick a vault, it draws the graph, you read and search and edit from the page](docs/overview.svg)

## Run it

```sh
nix run .                        # pick the vault in the page
nix run . -- ~/notes             # or name it and skip the picker
nix run . -- --browser ~/notes   # or use a browser instead of the window
```

brain-map opens a window of its own. On a machine with no display or no webview it
serves `http://localhost:4710` and opens your browser instead, which is also what
`--browser` forces. The page is the same either way, and the notes never leave the
machine.

It works on an Obsidian vault, an AI Workshop OS vault, or a plain folder of `.md`
files. Edges come from `[[wikilinks]]` and relative markdown links. A vault with no
links still draws as a tree: vault → folder → note.

## Keys

**drag** or **hjkl** pans, **scroll** zooms, **click** a node to read it. **/** focuses
the search box and **n** and **N** walk the matches. **R** replays the growth animation,
**F** gives the note the whole window, **space e** hides the explorer, **Esc** releases
whatever is held. Everything configurable lives behind **⚙ Settings**: vault, theme,
growth budget, explorer width, and whether the explorer shows.

## Groups and tags

A note has exactly one *group*: its top-level folder, or its role in an AI Workshop OS
vault. The group decides the note's colour, its size and when it appears in the growth
animation, so the groups partition the vault and their counts add up to it. *Tags* come
from the `tags:` field in the frontmatter. A note can carry several or none, they cut
across folders, and they only decide the icon. Filtering by a group shows one region of
the map. Filtering by a tag shows a thread running through several.

`title` and `tags` are the only frontmatter keys read. A `type:` is read past, so the
concepts of an
[Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md)
bundle group by the directory they sit in like any other note, and bundle-absolute links
(`[orders](/tables/orders.md)`) resolve alongside relative ones.

Skipped while scanning: `.git`, `.obsidian`, `node_modules`, `.brain-map`,
`__pycache__`, `.venv`, `venv`, `dist`, `build`, `.next`, `.cache`, and any
dot-directory.

## Development

```sh
nix develop . --command cargo test   # parsing, labelling, grouping
node tests/sim.mjs                   # the force simulation settles and stays put
```

The page lives in `src/web/`, one file per concern, joined into a single served file by
`src/page.rs` at compile time. No build step and no modules: the parts are plain scripts
concatenated in order. Three crates carry the rest — `emojis`, `tao`, `wry` — and
everything else is the standard library.

`GET /changed` returns one hash over every note's path, length and modified time. The
page polls it every two seconds and reloads when it moves, so what is on screen is what
is on disk. `GET /note?path=…` serves `.md` files that canonicalize to somewhere inside
the vault. `GET /open?path=…` and `GET /browse` carry a token minted per run and
embedded in the page, so only a page this process served can repoint the vault.

The diagram above is `docs/overview.excalidraw`, drawn in
[Excalidraw](https://excalidraw.com); `docs/overview.svg` is its export.

## License

MIT — see [LICENSE](LICENSE). The emoji keyword table in `src/emojis.txt` is vendored
from [Omarchy](https://github.com/basecamp/omarchy) (MIT).
