# brain-map

Turns a folder of markdown notes into an interactive knowledge graph in your browser —
watch your second brain assemble itself node by node, then explore it.
Zero dependencies: the Rust standard library and a browser, nothing else.

```sh
nix run . -- ~/notes
```

Serves `http://localhost:4710` and opens it. Notes never leave the machine.

Works on any Obsidian vault, AI Workshop OS vault, or plain folder of `.md` files.
Connections come from `[[wikilinks]]` and relative markdown links; folders with no links
still render as a clean structural tree (vault → folder → note).

## What you get

- **Growth animation** — the vault assembles from a single node, group by group, each
  folder paced by its own size. Press **R** or ⟲ Replay to watch again.
- **Explore** — scroll to zoom, drag empty space to pan, grab nodes to rearrange them.
- **Focus** — click a node and it and its links stay lit while everything else fades.
  Click again or press **Esc** to release.
- **Search** — type in the box and press Enter; the camera flies to the note.
- **Legend** — top-right, one row per group with its note count.
- **Themes** — pick one from the bar; the choice is remembered. Midnight is the default,
  One Dark repaints the chrome, the canvas and the group colours.
- Auto-grouping and colours by top-level folder, node size by connection count, and a
  camera that keeps the whole graph framed until you take over.

## Vault layouts

Generic folders get one colour group per top-level directory, a `Loose notes` group for
files at the root, and a structural tree so nothing floats alone. A vault with both
`CLAUDE.md` and `wiki/` is detected as an AI Workshop OS layout and grouped by role
(router, wiki, concepts, suites, skills, tools, worlds, notes) with unlinked notes dropped.

Skipped while scanning: `.git`, `.obsidian`, `node_modules`, `.brain-map`, `__pycache__`,
`.venv`, `venv`, `dist`, `build`, `.next`, `.cache`, and any dot-directory.

## Development

```sh
nix develop . --command cargo test   # parsing, labelling, grouping
node tests/sim.mjs                   # force simulation settles and stays put
```

The graph is rescanned on every page load, so adding notes and refreshing is enough.

A theme is one entry in the `THEMES` table in `src/index.html`: chrome colours become CSS
custom properties, canvas colours are read by the renderer, and an optional `palette`
overrides the group colours the vault picked for itself.
