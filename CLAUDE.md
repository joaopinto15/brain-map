# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

The words this project uses for things — vault, note, group, tag, graph, lit, source — are
defined in [CONTEXT.md](CONTEXT.md). Use them, and add to it rather than inventing a
synonym. Decisions that would otherwise be surprising are in [docs/adr/](docs/adr/).

## Toolchain

Nix manages every package; `cargo` and `rustc` are not on the system PATH.

```sh
nix develop . --command cargo test --workspace   # every crate
nix develop . --command cargo build --release
nix run .                        # the vault is chosen in the window
nix run . -- ~/notes             # a path on the command line skips the picker
```

One workspace and one lock. There is no separate build step and nothing is compiled for
another target: `cargo build` produces the whole program.

The window opens with libraries it dlopens rather than links — wayland, libxkbcommon and
GL — so `cargo run` outside `nix develop` fails at runtime, not at compile time. The
flake supplies them to both the package and the devShell. glvnd also needs a vendor file
to find a driver and mesa no longer ships one, so `flake.nix` builds one; NixOS's
`/run/opengl-driver` is still looked at first.

Flakes only see git-tracked files: `git add` a new file before building, or the build
fails on it.

## Shape

Three crates, and one seam between them.

- **the root crate** reads the disk and nothing else. `vault.rs` → `layout.rs` /
  `links.rs` → `graph.rs`, wired by `graph::build`, and `main.rs` implements `Source` over
  the result. `build` returns a `Scan` — the graph while nodes are still paths — and
  `Scan::into_graph` is where it becomes the `Graph` the window gets.
- **`crates/model`** is `Graph` — `Group`, `Node`, `Link` — and the `Source`
  trait. It carries no dependency at all.
- **`crates/app`** is the window. It may not touch the filesystem: everything it needs
  from the disk it asks `Source` for.

`Source` is the seam, and it is the old HTTP API with the HTTP taken out: `scan`,
`fingerprint`, `read_note`, `edit`, `choose_folder`, `open_vault` were `/graph.json`,
`/changed`, `/note`, `/edit`, `/browse`, `/open`. Keep it that way. A new thing the
window needs from the machine is a method here, implemented in `src/main.rs` — never a
`std::fs` call inside `crates/app`. `drives` is the one that was never a route: the start
page lists what rclone is configured with, and running a program is the scanner's job.

### The scanner

- **`vault.rs`** is the only module that reads the filesystem. Keep it that way — every
  stage after it is a pure function over the `Vault` value, which is why their tests need
  no temp directory. `Vault::has_file` exists so the link resolver can ask about skipped
  files without opening the disk itself. `walk` holds the skip rules once for both
  readers of the tree — `collect` reads the notes, `fingerprint` only stats them — so
  the two can never disagree about what is part of the vault.
- **`node.rs`** owns `NodeId` and what a note path means. The `__vault__` / `__dir__`
  prefixes are built in `NodeId::node_id` and nowhere else.
- **`yaml.rs`** is the frontmatter as a value tree: the subset of YAML the sample bundles
  are written in — block and flow mappings and lists, quoted and folded scalars, comments.
  Every value is a string, so a timestamp stays the text the author wrote. No anchors,
  block scalars or tags: none of the samples use them, and a YAML crate is a dependency
  the scanner does not take. `vault.rs` only finds the `---` block; this reads it.
- **`okf.rs`** is the Open Knowledge Format, decided: `Front` is every key the graph reads,
  typed, and the one place a key name is spelled — `type`, `title`, `description`,
  `resource`, `tags`, `icon`, `status`, `stale_after`, `generated`, `verified`, `sources`,
  and the v0.1 `timestamp` fallback. The rules live beside it: which trust tier `verified`
  earns, whether `stale_after` has passed, which filenames are reserved. Every rule is a
  pure function over what a note said, so the spec's sections are checkable one test at a
  time.
- **`layout.rs`** owns groups, node assignment and the structural tree. A concept's group
  is the `type` it declares and nothing else: not its folder, not a keyword, not a guess.
  Folders are structure — a bundle organizes its concepts into directories however it
  likes, so one holds several types and a type spreads over several folders — and the tree
  is drawn but never grouped on. There is one layout, because there is one format.
- **`links.rs`** reads connections out of note text only. It never sees the layout.
- **`graph.rs`** merges, orders, indexes, and hands over a `Graph`. It is where the clock
  is read, once, so every concept's staleness is judged against the same instant.

### The window

`crates/app` — Rust on `eframe`, with the graph and the note drawn by hand. Half of it
never mentions egui, and that half is where the tests are:

| Part | Holds | Needs a window |
|------|-------|----------------|
| `sim.rs` | the node and link model, the growth schedule, the d3-force port | no |
| `markdown.rs` | `Parser` and `Inline`: a note as `Block`s and `Span`s | no |
| `links.rs` | resolving a link target back to a note | no |
| `tree.rs` | the vault as a folder tree | no |
| `filter.rs` | the legend filter, its counts, the picker's vault search, and the watcher's baseline rule | no |
| `settings.rs` | `Settings`: the config file, read once and written through | no |
| `keys.rs` | the keymap, and what egui's `Key` is called in it | almost |
| `theme.rs` | the `THEMES` table, its colours, egui's `Visuals`, group colours | almost |
| `emoji.rs` | `Icons`: the colour bitmaps, read out of the desktop's emoji font | almost |
| `render.rs` | `Frame`: the graph — links, discs, icons, names, and the lit set | yes |
| `reader.rs` | `Reader`: drawing the blocks a note parsed into | yes |
| `state.rs` | `Ui` (what was chosen and is shown) and `Engine` (the graph and its camera) | yes |
| `app.rs` | `App` and `Session`: the vault, the frame, the watcher | yes |
| `chrome/` | one object per panel: bar, legend, explorer, settings, picker, viewport, keymap | yes |

`Session` and `Chrome` are siblings rather than one inside the other, and that is load
bearing: a panel is handed the whole session to act on, which it could not be if the
session owned it. Each panel owns only the state nobody else needs — the bar owns the
search text, the picker owns the path being typed.

## Constraints

- **The scanner has no dependencies at all.** Not few — none. Everything it does is the
  standard library, and it stays that way. The window takes `eframe`, which brings egui and
  the platform under it, plus `ttf-parser` and `png` — both already underneath eframe, and
  named directly only because `emoji.rs` calls them. `eframe` is pinned to a minor version
  because egui reshapes its `App` trait between them — 0.36 moved from `update(ctx)` to
  `ui(ui)`, so a bump is a port, not a number.
- **An icon is written, never guessed.** A note's `icon:` frontmatter key is the only
  source of one; a note that declares none draws as a plain disc, and so does every folder
  and the vault itself. There was a resolver here once — 158 lines against a 1,500-line
  keyword table — and it was deleted because a guess is wrong often enough to be worse
  than nothing: it made `machine-learning` a slot machine and `Service` a service dog. If
  an icon is missing, the fix is a line in the note, not a rule in the code.
- A tag has no icon. An icon belongs to a note, and a tag belongs to many, so the legend
  lists tags as words. A type and a signal are words for the same reason.
- **A concept is taken as it is found.** OKF §11 forbids a consumer from rejecting one for
  a missing field, an unknown `type`, an unrecognized key or a broken link, and nothing
  here does: a note with no frontmatter at all is an `Untyped`, `unverified` concept and
  draws like any other. That is also why there is no bundle detection and no OKF mode —
  every vault is read this way, and a vault that declares nothing simply has one type.
- Trust is derived, never stored: `okf::trust` reads the `verified` actors and returns a
  tier, and the graph carries the tier. The `human:` prefix (§7) is the whole of that rule.
- The reader shows what the concept declared and nothing it did not: the description, the
  `resource` as a link, who generated and verified it, the `sources` it derives from, and
  the concepts that cite it — the last read backwards off the graph's own edges, in
  `Engine::cited_by`. A source that names a note in this vault opens it; a URL opens
  outside; a scope descriptor (§5.1) is words. `Node::concept` is the seam that carries
  all of it, and the simulation never copies it.
- The legend's three axes are one mechanism: `Filter::Group`, `Filter::Signal` and
  `Filter::Tag` differ only in what `filter::matches` compares, and all three produce the
  same lit set. A fourth axis is a variant and an arm, not a second way to dim.
- A name too long for the explorer ends in an ellipsis, and the whole of it is the
  tooltip. A file row is `Button::selectable(..).truncate()`, which is egui's own; a
  folder row cannot be, because `CollapsingHeader` lays its title out with
  `TextWrapMode::Extend` and takes no say in it — so `elide` cuts the title to the width
  first, measuring with the fonts that will draw it. It takes its measure function as an
  argument, the way `Sim::new` does, which is why it is tested without a window.
- A character the chrome draws must be in one of the four fonts egui bundles, and one
  that is not draws as an empty box with no warning and no error. `✎` (U+270E) and `✕`
  (U+2715) are in none of them, which is how the edit and close buttons shipped invisible.
  Every such character is named in `chrome/glyph.rs` and checked there against the real
  font files, so a nice-looking codepoint cannot be picked without the fonts agreeing.
- Icons are pictures, not characters. egui rasterises outlines with `ab_glyph` and cannot
  draw a colour bitmap font, so an emoji drawn as text comes out a grey outline however
  good the font is. `emoji.rs` reads the PNG for a glyph straight out of Noto Color
  Emoji's `CBDT` table and hands egui a texture, cached one per emoji; `render.rs` and the
  chrome draw an image where they used to draw a character, and fall back to the character
  where no colour font is installed. The font is found by `$BRAIN_MAP_EMOJI_FONT` first —
  which is what the Nix wrapper and the AUR dependency point at — and then by a short list
  of the usual paths.
- **No JavaScript, and no HTML.** That is the point of the program's shape: the window
  draws itself, so there is no page for any to live in. A note is text — `markdown.rs`
  produces `Block`s and `Span`s and never a string of markup, so there is nothing to
  escape and no way for a note to reach past the panel it is drawn in.
- A link in a note is either an `http(s)` URL or a path that has to resolve to a note in
  this vault. There is no third kind, and `Target` is where that is decided.
- `Ui` and `Engine` have no public fields, and neither does any panel. A change to what is
  shown is a method that names what it is for — `toggle_filter`, `open_settings`,
  `stop_reading`, `set_panel` — so the list of things that can happen to a value is the
  list of its methods. Reaching into a field from the chrome is how the last version ended
  up with the same rule written in four places.
- The camera's arithmetic lives in `Engine`, never in the chrome. `chrome/viewport.rs`
  turns a wheel, a drag and a click into `zoom`, `grab`, `drag_to`, `pan_by` and `click`,
  and knows none of the maths behind them.
- Colours come from the `THEMES` table in `crates/app/src/theme.rs`, never from literals
  in the renderer. A theme is a struct, so a colour missing from one theme is a compile
  error. `theme::color` is the one place a `#rrggbb` or `rgba(…)` spec becomes something
  the painter takes, and an unparseable one comes out as egui's placeholder magenta so a
  typo is visible rather than black. Emoji go the other way: the window holds none, and
  draws whatever each node's `icon` says.
- egui has no fonts until the first frame, and a name cannot be measured before there is
  something to measure it with. `App::new` therefore builds an empty engine and the vault
  named on the command line is opened on the first `update`, after the viewport is known.
  Loading against a viewport that is still zero clamps the explorer to its minimum and it
  never grows back.
- The graph is painted on egui's background layer across the whole window, and the panels
  are drawn over it. The camera centres on the whole window and knows nothing about the
  explorer, which floats over the graph's left edge rather than pushing it aside — so
  opening the tree never moves the graph.
- The explorer's width lives in one place, `Ui::panel_w`, and `Engine::set_panel` is the
  only writer. The grip and the settings slider both go through it; nothing reads a
  literal. It is a layout width only — the simulation never hears about it. It outlives the
  run in the settings file as `panel`, written by `Engine::remember_panel` when the grip is
  let go rather than while it moves, since a drag changes the width every frame.
- The picker is the start page, so it is the only thing on screen until a vault is open:
  `Chrome::show` draws the bar, the hint, the explorer and the legend only once
  `graph().vault` is set, since an empty count and a legend with no rows say nothing. One
  field takes a path, a git URL and a drive because the scanner decides which it is;
  `filter::is_search` decides the other way, that a bare word searches the vaults already
  opened instead of naming a new one. Esc closes the picker only when there is a vault
  behind it — `chrome/keymap.rs` guards that, or the window would have no way back.
- The reader's two states are `Ui::reading` and `Ui::full`: reading swaps the folder tree
  for the note, full gives the panel the window. `Engine::close_note` clears both — a
  full-width panel holding nothing would cover the graph with no way back — and `F`
  refuses to act with no note open.
- The window follows the vault by polling, not by watching: `Source::fingerprint` is one
  FNV hash over every note's path, length and mtime, and the window reloads when it moves.
  There is no notifier in the standard library and another dependency is not worth it for
  a stat walk. The first answer is only a baseline — reloading on it loops forever, which
  is what `filter::changed` guards and what its own test checks. A reload the watcher
  asked for keeps the note that was open and skips the growth animation, without touching
  the budget.
- Keys are one decision, `keys::binding`, which takes the key's name and returns `None`
  while a field has focus so typing never triggers a binding. `keys::key_name` is the only
  place egui's `Key` becomes that name. Space is a leader cleared by whatever key follows
  it. A new binding is an `Action`, a line in `binding`, an arm in `ui::keys` and a word in
  the hint bar; nothing else knows the keymap, and both halves are tested on the host.
- `/` focuses the search box, and the same keystroke would otherwise be typed into it —
  the field is focused in the frame the key arrived in. `ui::keys` drops that one text
  event, which is what `preventDefault` used to do.
- The legend filter and the click/hover focus share one mechanism: `render::lit`, the set
  of node indices that stay bright while everything else draws at 0.12 alpha. A filter
  holds that set until it is cleared; add no second dimming path.
- The force simulation is a hand-rolled port of d3-force (link, charge, collide, center,
  gravity) with alpha cooling. Never remove the alpha decay or the collide pass — without
  them the nodes jitter and fling apart.
- What the collide pass keeps apart is names, not discs. `Sim::new` measures every label
  once into `hw` (capped, so one long filename cannot open a crater), through a function
  the caller supplies — egui's fonts in the window, a plain width in the tests — and `tick`
  counts vertical distance `LABEL_SQUASH` times heavier, since a label is wide and short:
  nodes may stack closely above one another and never sit side by side with their names
  overlapping. A node is also born clear of that radius, or its first tick flings it out.
- Sections read as sections because a link that leaves its folder — `Link::far`, decided
  once in `Sim::new` — rests `GROUP_GAP` times longer. Spreading them costs zoom, since
  the camera fits the whole graph, so pushing that constant further eventually shrinks
  every name below the size `render.rs` will draw it at. The tests in `sim.rs` hold both
  ends: no label overlap, folders visibly further apart than their own notes, and most
  names still drawn at the settled zoom. The simulation's RNG is seeded, so a layout is
  reproducible and those numbers are the same every run.
- Settings outlive a run in one `key=value` file under the desktop's config directory,
  `~/.config/brain-map/settings`. A key may repeat, which is the whole of the recent-vaults
  list — no format and no parser. Every setting the dialog has is a key in it — `theme`,
  `growth`, `panel`, `explorer`, `recent` — so a setting added to the dialog is a key added
  to `settings.rs` and read in `Ui::new`.
- `open_in_editor` wraps `$EDITOR` in a terminal emulator (table `TERMINALS`, `$TERMINAL`
  first) and detaches it with `setsid`, so a terminal editor never lands in the terminal
  running brain-map. `$VISUAL` skips the wrapper. The argv is built by `editor_command`,
  a pure function, which is where its tests live.
