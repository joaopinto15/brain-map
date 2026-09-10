# Keys and settings

Every key the window binds, and every setting that outlives a run.

## Mouse

| Do this | To get this |
|---------|-------------|
| Drag the background | Pan the graph |
| Scroll | Zoom |
| Click a node | Read that note |
| Click it again | Close the note |
| Drag the explorer's right edge | Resize the panel |

## The file tree

The explorer takes vi's motions. A key moves the tree when no note is open, and the note
when one is.

| Key | In the tree | While reading |
|-----|-------------|---------------|
| `j` | Down a row | Scroll down |
| `k` | Up a row | Scroll up |
| `h` | Close the folder, or step out to it | Close the note |
| `l` or `Enter` | Open the folder, step into it, or read the note | Nothing |
| `g` | First row | Top of the note |
| `G` | Last row | Bottom of the note |

The cursor scrolls itself into view, and reading a note flies the camera to its node.

## Everywhere else

| Key | Does |
|-----|------|
| `/` | Focus the search box |
| `n`, `N` | Next and previous match |
| Arrow keys | Pan the camera |
| `R` | Replay the growth animation |
| `F` | Give the note the whole window |
| `space` `e` | Hide or show the explorer |
| `Esc` | Drop the selection, the filter and the open note |

Typing in a field never triggers a binding, so `r`, `n` and `/` are safe to type into the
search box.

## The settings dialog

The gear in the top bar opens it. It holds the vault, the theme, the growth budget, the
explorer's width, and whether the explorer shows at all.

Growth budget is how long the whole vault takes to assemble when it opens: instant, 3s,
5s, 8s, 15s, or 30s. `R` replays it.

## The settings file

Settings live in one file of `key=value` lines at `~/.config/brain-map/settings`, or
under `$XDG_CONFIG_HOME` if you set one. Edit it by hand if you prefer.

| Key | Value | Default |
|-----|-------|---------|
| `theme` | `midnight`, `onedark`, or `gruvbox` | `midnight` |
| `growth` | Milliseconds the growth animation takes, `0` for instant | `8000` |
| `panel` | Explorer width in pixels, 220 to 900 | `220` |
| `explorer` | `on` or `off` | `on` |
| `recent` | A vault you opened. The key repeats, newest first, up to six | none |

A value the program cannot parse falls back to the default, so a typo costs you the
setting and nothing else. Delete the file to start over.

To add a theme, add a `Theme` struct to the `THEMES` table in
`crates/app/src/theme.rs`. The dialog lists it, the file remembers it by its `key`, and a
colour you forget to name is a compile error.

## Opening a note in your editor

The pencil in the note's header runs `$EDITOR` inside a terminal emulator, detached, so a
terminal editor never lands in the terminal that started brain-map. Set `$TERMINAL` to
choose which emulator. Set `$VISUAL` instead for an editor that opens its own window, and
brain-map runs it without the terminal.
