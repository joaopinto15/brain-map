# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Toolchain

Nix manages every package; `cargo` and `rustc` are not on the system PATH.

```sh
nix develop . --command cargo test
nix run . -- ~/notes
```

Flakes only see git-tracked files: `git add` a new file before building, or the build
fails on it.

## Checks

```sh
nix develop . --command cargo test   # graph model: parsing, labelling, grouping
node tests/sim.mjs                   # runs the real page script headlessly; fails if the
                                     # simulation never cools, overlaps, or drifts
```

`tests/sim.mjs` slices the `<script>` out of `src/index.html` and runs it against a
synthetic graph with stubbed DOM objects, so front-end changes stay covered. Keep the
script self-contained (no imports, no top-level DOM reads outside `document.getElementById`)
or the harness stops being able to load it.

## Shape

`vault.rs` → `layout.rs` / `links.rs` → `graph.rs`, wired by `graph::build`:

- **`vault.rs`** is the only module that reads the filesystem. Keep it that way — every
  stage after it is a pure function over the `Vault` value, which is why their tests need
  no temp directory. `Vault::has_file` exists so the link resolver can ask about skipped
  files without opening the disk itself.
- **`node.rs`** owns `NodeId` and what a note path means. The `__vault__` / `__dir__`
  wire prefixes are built in `NodeId::wire_id` and nowhere else.
- **`layout.rs`** owns everything that differs between a generic folder vault and an AIOS
  vault: groups, node assignment, the structural tree, and `keeps_isolates`. Two adapters,
  `Layout::generic` and `Layout::aios`. A new rule that only applies to one of them belongs
  in its constructor, not in an `if` further down the pipeline.
- **`links.rs`** reads connections out of note text only. It never sees the layout.
- **`graph.rs`** merges, prunes, orders, indexes, and serialises.

## Constraints

- Zero Cargo dependencies. `Cargo.lock` is hand-written; adding a dependency means
  regenerating it and the `cargoLock.lockFile` build keeps working.
- `src/index.html` is embedded with `include_str!` and served after replacing the
  `__GRAPH__` placeholder with the graph JSON. No CDN links, no build step for the JS.
- Graph JSON is hand-serialised in `graph.rs` — keep new fields escaped with `esc`.
- The graph model mirrors the reference project's `build.py`: group table with
  `pace`/`pause` driving the growth animation, structural tree nodes, AIOS detection
  via `CLAUDE.md` + `wiki/`.
- Colours in the page come from the `THEMES` table in `src/index.html`, never from
  literals in the stylesheet or the renderer. Chrome colours are pushed to CSS custom
  properties; canvas colours are read from the active theme each frame.
- The force simulation is a hand-rolled port of d3-force (link, charge, collide, center,
  gravity) with alpha cooling. Never remove the alpha decay or the collide pass — without
  them the nodes jitter and fling apart.
