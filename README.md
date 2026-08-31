# brain-map

Turns a folder of markdown notes into an interactive knowledge graph — watch your second
brain assemble itself node by node, then explore it. Near-zero dependencies: the Rust
standard library, one crate for emoji names, and a webview.

```sh
nix run .                        # pick the vault in the page
nix run . -- ~/notes             # or name it and skip the picker
nix run . -- --browser ~/notes   # or use a browser instead of the window
```

Opens a window of its own. On a machine with no display or no webview it falls back to
serving `http://localhost:4710` and opening your browser, which is also what `--browser`
forces; the page is the same either way. Without a path it opens on a vault picker:
**Browse…** opens the desktop's own folder dialog (zenity or kdialog), or type a path —
`~` and `$HOME` expand. Recent vaults are remembered, and **Change…** under ⚙ Settings
switches to another one without restarting. Notes never leave the machine. The bar keeps
only what you act on — search, ⟲ Replay, ⟳ Reload — and everything configurable is behind
**⚙ Settings**: vault, theme, growth animation, explorer width, and whether the explorer
shows.

Works on any Obsidian vault, AI Workshop OS vault, or plain folder of `.md` files.
Connections come from `[[wikilinks]]` and relative markdown links; folders with no links
still render as a clean structural tree (vault → folder → note).

## What you get

- **Growth animation** — the vault assembles from a single node, group by group, each
  folder paced by its own size. Press **R** or ⟲ Replay to watch again. The whole thing
  fits one budget however many folders you have, and the picker in the bar sets it —
  Instant through 30s, remembered between runs.
- **Live** — the page watches the vault and reloads itself when a note is added, edited,
  renamed or deleted, so what is on screen is what is on disk. The note you were reading
  reopens and the growth animation is skipped, since that reload was the disk's idea.
  ⟳ Reload does the same on demand.
- **Explore** — scroll to zoom, drag empty space to pan, grab nodes to rearrange them.
  **hjkl** pans from the keyboard, **space e** hides the explorer to give the graph the
  whole window.
- **Search** — **/** jumps to the box, Enter runs it, **n** and **N** walk the matches,
  wrapping. A note whose name matches comes before one that only matches by path.
- **Explorer** — a collapsible folder tree of the vault sits permanently down the left,
  each note with its icon. Click one to open it and fly the camera to its node.
- **Read** — click a node or a file and the explorer gives way to the note, markdown
  rendered, frontmatter stripped, tags along the top. **Esc** or ✕ brings the tree back.
  Drag the panel's right edge to widen it; the width is remembered.
- **Fullscreen** — **F** or ⛶ gives the note the whole window, the text held to a
  readable measure rather than run across a monitor. Press it again, or close the note.
- **Edit** — ✎ in the reader header opens the note in `$EDITOR`, in a terminal window of
  its own so it never takes over the terminal running brain-map. `$TERMINAL` picks the
  emulator (`xdg-terminal-exec`, alacritty, ghostty, kitty, foot, wezterm or xterm are
  found otherwise); set `$VISUAL` instead if your editor already opens its own window.
- **Follow links** — `[[wikilinks]]` and markdown links inside the note are clickable:
  they open the target note and fly the camera to its node, resolved exactly as the
  graph resolved them into edges. A link to a note that isn't there greys out instead.
  External `http(s)` links open in a new tab.
- **Focus** — the clicked node and its links stay lit while everything else fades.
  Click again or press **Esc** to release.
- **Search** — type in the box and press Enter; the camera flies to the note.
- **Icons** — every node carries one emoji inside its circle: its first frontmatter tag,
  or its group when it has none. Words are resolved against the
  [`emojis`](https://crates.io/crates/emojis) crate first and then a keyword table
  vendored from [Omarchy](https://github.com/basecamp/omarchy) (MIT), so `linux` draws 🐧,
  `banking` draws 🏦 and `travel` draws 🗺️. The busiest tags get their own legend section.
- **Legend, and filtering** — top-right, one row per group with its note count, then the
  busiest tags. Click any row to keep just those notes lit while the rest fade; click it
  again, or press **Esc**, to let go. The counter in the bar reads `12 of 83 notes`
  while a filter is on.

  **Groups and tags are not the same thing.** A note has exactly one *group* — its
  top-level folder, or its role in an AI Workshop OS vault. The group decides the note's
  colour, its size and when it appears in the growth animation, so the groups partition
  the whole vault and their counts add up to it. *Tags* come from the `tags:` field in a
  note's frontmatter: a note can carry several or none, they cut across folders, and they
  only decide the icon. Filtering by a group shows one region of the map; filtering by a
  tag shows a thread running through several. Where a note lives is its group; what it is
  about is its tags, and no other frontmatter field is read for either.
- **Themes** — pick one from the bar; the choice is remembered. Midnight is the default,
  One Dark repaints the chrome, the canvas and the group colours.
- Auto-grouping and colours by top-level folder, node size by connection count, and a
  camera that keeps the whole graph framed until you take over.

## Vault layouts

Generic folders get one colour group per top-level directory, a `Loose notes` group for
files at the root, and a structural tree so nothing floats alone. A vault with both
`CLAUDE.md` and `wiki/` is detected as an AI Workshop OS layout and grouped by role
(router, wiki, concepts, suites, skills, tools, worlds, notes) with unlinked notes dropped.

Frontmatter is read on any vault, and only two keys are: `title` names the node and
`tags` give it icons and its cross-cutting filter. A `type:` is read past — an OKF bundle
declares one on every concept, and those concepts still group by the directory they sit
in. Those field names and the bundle-absolute links (`[orders](/tables/orders.md)`) that
resolve alongside relative ones come from the
[Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md),
so an OKF bundle drops straight in.

Skipped while scanning: `.git`, `.obsidian`, `node_modules`, `.brain-map`, `__pycache__`,
`.venv`, `venv`, `dist`, `build`, `.next`, `.cache`, and any dot-directory.

## Development

```sh
nix develop . --command cargo test   # parsing, labelling, grouping
node tests/sim.mjs                   # force simulation settles and stays put
```

The graph is rescanned on every page load, so a reload is a rescan. `GET /changed`
returns one number over every note's path, length and modified time; the page polls it
every two seconds and reloads when it moves. Nothing is read to compute it, and the skip
list the scan uses applies, so a busy `.git` does not trigger anything.
The reader panel pulls note source from `GET /note?path=…`, which only serves `.md` files
that canonicalize to somewhere inside the vault.

`GET /open?path=…` sets the vault and `GET /browse` opens the folder dialog. Both carry a
token minted per run and embedded in the page, so only a page this process served can
repoint it — another site in the same browser cannot.

The panel is typeset for long notes, not just short ones: the gap between paragraphs
beats the gap between lines, headings step down in size and take their own colour, and
bold, inline code and list markers are coloured from the active theme.

Markdown in the panel is rendered by a small hand-rolled pass: headings, paragraphs,
nested lists and task lists, tables with column alignment, quotes, fenced code with a
language class, images, footnotes, emphasis, strikethrough, autolinks, links and
wikilinks. Not covered: reference links, setext headings, and inline HTML — HTML stays
escaped on purpose, since a note is not trusted markup.

Internal links carry their raw target in a `data-note` attribute and are resolved on
click by the same rules as `links.rs`, so the reader follows exactly the links the graph
drew.

The page lives in `src/web/`, one file per concern, joined into a single served file by
`src/page.rs` at compile time — no build step and no modules, so the parts are plain
scripts concatenated in order.

A theme is one entry in the `THEMES` table in `src/web/theme.js`: chrome colours become CSS
custom properties, canvas colours are read by the renderer, and an optional `palette`
overrides the group colours the vault picked for itself.
