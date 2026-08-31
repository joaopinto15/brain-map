# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Toolchain

Nix manages every package; `cargo` and `rustc` are not on the system PATH.

```sh
nix develop . --command cargo test
nix run .                        # the vault is chosen in the page
nix run . -- ~/notes             # a path on the command line skips the picker
nix run . -- --browser ~/notes   # force the browser instead of the window
```

The window needs the WebKitGTK stack, which `flake.nix` supplies to both the package and
the devShell — `cargo` run outside `nix develop` fails on `pkg-config`. WebKitGTK also
aborts without an EGL display and a Nix binary cannot load the host distribution's
driver, so both wrap `__EGL_VENDOR_LIBRARY_DIRS` with a Mesa fallback behind NixOS's
`/run/opengl-driver`.

Flakes only see git-tracked files: `git add` a new file before building, or the build
fails on it.

## Checks

```sh
nix develop . --command cargo test   # graph model: parsing, labelling, grouping
node tests/sim.mjs                   # runs the real page script headlessly; fails if the
                                     # simulation never cools, overlaps, or drifts
```

`tests/sim.mjs` assembles the page the way `src/page.rs` does — reading that file's
`include_str!` list — and runs the script against a synthetic graph with stubbed DOM
objects, so front-end changes stay covered. Keep the script self-contained (no imports,
no top-level DOM reads outside `document.getElementById`) or the harness stops being able
to load it.

## Shape

`vault.rs` → `layout.rs` / `links.rs` → `graph.rs`, wired by `graph::build`:

- **`vault.rs`** is the only module that reads the filesystem. Keep it that way — every
  stage after it is a pure function over the `Vault` value, which is why their tests need
  no temp directory. `Vault::has_file` exists so the link resolver can ask about skipped
  files without opening the disk itself. `walk` holds the skip rules once for both
  readers of the tree — `collect` reads the notes, `fingerprint` only stats them — so
  the two can never disagree about what is part of the vault.
- **`node.rs`** owns `NodeId` and what a note path means. The `__vault__` / `__dir__`
  wire prefixes are built in `NodeId::wire_id` and nowhere else.
- **`layout.rs`** owns everything that differs between a generic folder vault and an AIOS
  vault: groups, node assignment, the structural tree, and `keeps_isolates`. Two adapters,
  `Layout::generic` and `Layout::aios`. A new rule that only applies to one of them belongs
  in its constructor, not in an `if` further down the pipeline. A note's group is where it
  lives — its top-level folder, or its AIOS role — and nothing else: frontmatter `type` is
  read past, because what a note is about is its tags, which cut across folders and filter
  on their own axis. `bucket` is that decision, and it takes only the path.
- **`page.rs`** joins `src/web/` into the served page. Its `include_str!` list is the
  only place the script's order lives, and `tests/sim.mjs` reads that list rather than
  keeping its own copy — a part missing from it is neither served nor tested.
- **`icons.rs`** resolves a tag or group key to one emoji, against the `emojis` crate
  first (gemoji shortcodes and Unicode names) and then the keyword table in
  `src/emojis.txt` (vendored from Omarchy, MIT), which still carries the looser synonyms
  the crate has no word for — `music`, `travel`, `idea`. `HINTS` maps jargon neither
  source can know (`kubernetes`, `pkm`, `vim`) onto a plain noun they do — add a hint
  there, never an emoji literal. Unresolvable terms get no icon rather than a wrong one.
- **`links.rs`** reads connections out of note text only. It never sees the layout.
- **`graph.rs`** merges, prunes, orders, indexes, and serialises.

## The page

`src/web/` — one concern per file, concatenated in this order:

| Part | Holds |
|------|-------|
| `page.html` | markup and the placeholders |
| `style.css` | every rule; chrome colours are custom properties the themes set |
| `theme.js` | `GRAPH`, `TOKEN`, the `THEMES` table, `applyTheme`, the icon lookup |
| `graph.js` | canvas sizing, the node and link model, `schedule` and its growth budget |
| `sim.js` | force constants, simulation state, `tick`, `activate`, hit testing |
| `render.js` | `draw` and the `frame` loop |
| `input.js` | mouse, the keymap and its leader, search and its matches, `goTo` |
| `markdown.js` | the note renderer, escaping inwards and HTML never outwards |
| `reader.js` | link resolution, the reader panel and its width, fullscreen, the folder tree |
| `chrome.js` | replay, the vault picker, the legend and its filter, the settings dialog |

Order is execution order, so a part may only use what an earlier part defined. Adding a
part means adding one `include_str!` line to `page.rs`; nothing else knows the list.

## Constraints

- Three Cargo dependencies and no more: `emojis` for the icon names, `tao` for the
  window, `wry` for the webview inside it. Everything else is the standard library — the
  rule is no dependency for what the standard library already does, and the page itself
  still has none. `Cargo.lock` is generated by `cargo generate-lockfile`; after adding a
  dependency, check the `cargoLock.lockFile` build still works with `nix build .`.
- The page is a folder of parts, `src/web/`, joined by `page.rs` at compile time and
  served as one file. No CDN links, no build step, no modules: the parts are concatenated
  in order into a single classic script, so a part may use what an earlier part defined
  and must not `import`. `page.html` carries the markup and the four placeholders
  (`__STYLE__`, `__SCRIPT__`, `__GRAPH__`, `__TOKEN__`).
- The vault is runtime state — `serve` in `main.rs` owns the `Option<PathBuf>`, `/open`
  sets it, and an empty `NO_VAULT` graph makes the page show the picker. `/open` and
  `/browse` require the token, so only a page this process served can repoint the vault.
  `/browse` shells out to the desktop's folder dialog, since a page cannot name a real path.
- The page is displayed by whichever of two paths works. `try_window` builds a `tao`
  window with a `wry` webview on it; every failure it can see — `--browser`, no display,
  no webview — returns `Err` and `main` falls back to `xdg-open` and a browser, exactly as
  the program behaved before the window existed. The loopback server is what makes both
  possible, so it stays even though `wry` could serve the page over a custom protocol:
  every fetch in `src/web/` is a relative path and neither path knows the difference. The
  browser is also how you get devtools on a release build.
- `serve` runs on its own thread because the event loop must own the main one. The
  listener is bound in `main` before the thread starts, so the socket is already
  listening when the window asks for the page.
- Graph JSON is hand-serialised in `graph.rs` — keep new fields escaped with `esc`.
- The graph model mirrors the reference project's `build.py`: group table with
  `pace`/`pause` driving the growth animation, structural tree nodes, AIOS detection
  via `CLAUDE.md` + `wiki/`.
- Scrollbars are themed off the same custom properties as the rest of the chrome, set once
  on `html` since `scrollbar-color` inherits. `color-scheme: dark` sits there too, so the
  native widgets match; a light theme would have to set it per theme instead.
- `open_in_editor` wraps `$EDITOR` in a terminal emulator (table `TERMINALS`, `$TERMINAL`
  first) and detaches it with `setsid`, so a terminal editor never lands in the terminal
  running the server. `$VISUAL` skips the wrapper. The argv is built by `editor_command`,
  a pure function, which is where its tests live.
- Colours in the page come from the `THEMES` table in `src/web/theme.js`, never from
  literals in the stylesheet or the renderer. `CSS_KEYS` is the list that gets pushed to
  custom properties, prose colours (`heading`, `strong`, `code`) included; a key listed
  there and missing from a theme writes `undefined` into the page, which is what the
  theme pass in `tests/sim.mjs` catches by recording what `applyTheme` actually sets. Emoji go the other way: the page holds none,
  and reads the `icons` map off the graph JSON. Chrome colours are pushed to CSS custom
  properties; canvas colours are read from the active theme each frame.
- The reader's two states are classes on `body`: `reading` swaps the folder tree for the
  note, `full` gives the panel the window. `closeNote` clears both — a full-width panel
  holding nothing would cover the graph with no way back — and `toggleFull` refuses to
  act with no note open. `tests/sim.mjs` holds a real class list rather than a stub so
  those two rules are actually checked.
- The page follows the vault by polling, not by watching: `GET /changed` is
  `vault::fingerprint`, one FNV hash over every note's path, length and mtime, and the
  page reloads when it moves. There is no notifier in the standard library and a fourth
  dependency is not worth it for a stat walk. The first answer is only a baseline —
  reloading on it loops forever, which is what `changed` in `chrome.js` guards and what
  `tests/sim.mjs` checks. A watcher-driven reload sets `brain-map-resume` in
  sessionStorage, so the note being read comes back and that one load skips the growth
  animation without touching the picker's setting.
- Keys are one `keydown` listener in `input.js`, and it returns early for `INPUT` and
  `SELECT` so typing never triggers a binding. Space is a leader cleared by whatever key
  follows it, and only the key it binds is consumed: swallowing the rest means one stray
  space silently kills the next keystroke. `space e` hides the explorer. `/` focuses the search box, Enter runs the query, `n`/`N` walk the matches.
  A new binding is a line in that listener and a word in `#hint`; nothing else knows the
  keymap. `tests/sim.mjs` records the listeners the page registers and presses the keys
  through them — a no-op `addEventListener` stub cannot tell working wiring from none.
- The explorer's width lives in one place, `--panel-w`: the stylesheet lays out against
  it, `graph.js` mirrors it into `PANEL` so the canvas centres on what is left, and
  `setPanel` is the only writer. Anything that needs the panel's width reads that
  property, never a literal.
- The bar holds what you do — count, search, reload — and every setting lives in
  the `<dialog id="settings">`: vault, theme, growth budget, explorer width and whether
  the explorer shows. It is a native `<dialog>`, so the modality, the backdrop and
  Esc-to-close are the platform's, not ours. A setting reachable two ways reads its state
  back through `drawSettings` rather than keeping a copy — the width is also the grip, and
  the explorer switch is also `space e`. `tests/sim.mjs` asserts the split holds and that
  both paths to a setting agree.
- The legend filter and the click/hover focus share one mechanism: `lit`, the set of node
  indices that stay bright while everything else draws at 0.12 alpha. A filter holds that
  set until it is cleared; add no second dimming path.
- The force simulation is a hand-rolled port of d3-force (link, charge, collide, center,
  gravity) with alpha cooling. Never remove the alpha decay or the collide pass — without
  them the nodes jitter and fling apart.
- What the collide pass keeps apart is names, not discs. `graph.js` measures every label
  once into `n.hw` (capped, so one long filename cannot open a crater), and `sim.js`
  counts vertical distance `LABEL_SQUASH` times heavier, since a label is wide and short:
  nodes may stack closely above one another and never sit side by side with their names
  overlapping. A node is also born clear of that radius, or its first tick flings it out.
- Sections read as sections because a link that leaves its folder — `l.far`, decided once
  in `graph.js` — rests `GROUP_GAP` times longer. Spreading them costs zoom, since the
  camera fits the whole graph, so pushing that constant further eventually shrinks every
  name below the size `render.js` will draw it at. `tests/sim.mjs` holds both ends: no
  label overlap, folders visibly further apart than their own notes, and most names still
  drawn at the settled zoom. Its RNG is seeded so those numbers are the same every run.
