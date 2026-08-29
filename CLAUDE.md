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

## Constraints

- Zero Cargo dependencies. `Cargo.lock` is hand-written; adding a dependency means
  regenerating it and the `cargoLock.lockFile` build keeps working.
- `src/index.html` is embedded with `include_str!` and served after replacing the
  `__GRAPH__` placeholder with the graph JSON. No CDN links, no build step for the JS.
- Graph JSON is hand-serialised in `build_graph`/`esc` — keep new fields escaped.
- The graph model mirrors the reference project's `build.py`: group table with
  `pace`/`pause` driving the growth animation, structural tree nodes (`__vault__`,
  `__dir__<top>`) in generic mode, AIOS detection via `CLAUDE.md` + `wiki/`.
- The force simulation is a hand-rolled port of d3-force (link, charge, collide, center,
  gravity) with alpha cooling. Never remove the alpha decay or the collide pass — without
  them the nodes jitter and fling apart.
