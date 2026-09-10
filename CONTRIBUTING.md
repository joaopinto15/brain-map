# Working on brain-map

How a change gets from your editor into a release. Read
[how brain-map is put together](docs/architecture.md) first if you want to know where the
code lives.

## Set up

Nix manages every package, so `cargo` and `rustc` are not on the system PATH. Everything
runs through the flake:

```sh
nix develop . --command cargo test --workspace   # every crate
nix develop . --command cargo run -- ~/notes     # the window, on a vault
nix develop . --command cargo clippy --workspace # lints, not required by CI
nix build                                        # the release build
nix run .                                        # run what nix build produced
```

The window opens libraries it never links against, Wayland, libxkbcommon and GL, so
`cargo run` outside `nix develop` compiles and then fails to start.

Flakes only see files git knows about. Run `git add` on a new file before building, or
the build fails on it.

## Branch

`main` takes no direct pushes. Every change starts as a branch whose name says what kind
of change it is:

| Prefix | For |
|--------|-----|
| `feature/` | Something the program could not do before |
| `bugfix/` | Something that was meant to work and did not |
| `hotfix/` | A break in a release that cannot wait |

After the prefix, lower case words joined by hyphens.

```sh
git checkout -b feature/walk-the-tree-with-vim
git checkout -b bugfix/legend-runs-off-a-short-window
```

Anything else fails the `branch-name` check, which prints the commands that rename the
branch. To rename one yourself:

```sh
git branch -m feature/<name>
git push origin -u feature/<name>
git push origin --delete <old-name>
```

## Commit

`<type>: <subject>`, in the imperative, no full stop, 50 characters at most. The types in
use are `feat`, `fix`, `build`, `docs`, `refactor` and `test`.

Add a body when the reason is not obvious from the diff, wrapped at 80 columns. Say why
the change exists, not what the diff already shows.

```
fix: open a vault off the frame thread

The folder dialog and a git clone ran inside `update`, so the window
answered no compositor pings for as long as they took.
```

Keep one change per commit. Seven small commits read better than one that does the week.

## Before you push

```sh
nix develop . --command cargo test --workspace
nix develop . --command cargo fmt --all
```

CI runs both, and `cargo fmt --all --check` fails on formatting you did not apply.

If the change touches how the program behaves, run it against a real vault and look at
it. Tests do not catch a panel drawn off the edge of the window.

Update the docs in the same commit as the code:

- a new word for a thing goes in [CONTEXT.md](CONTEXT.md)
- a decision that would surprise the next reader goes in [docs/adr](docs/adr)
- a new key, setting or frontmatter field goes in the doc that lists them

## Open a pull request

```sh
git push -u origin feature/<name>
gh pr create --base main
```

Two checks must pass before it can merge:

| Check | Runs | Fails when |
|-------|------|------------|
| `branch-name` | About 5 seconds | The branch is not `feature/`, `bugfix/` or `hotfix/` |
| `tests` | About 90 seconds | `cargo test --workspace --locked` or `cargo fmt --all --check` fails |

`main` needs no approving review, so you can merge your own work once the checks are
green. Comment threads on the PR must be resolved first. Force pushes to `main` and
deleting it are refused, for you as well as for anyone else.

```sh
gh pr merge --merge --delete-branch
```

## Release

A tag is the release. Nothing is run by hand.

1. Bump `version` in `Cargo.toml`. That value is the one version in the repository.
2. Merge that through a pull request like anything else.
3. Tag `main` and push the tag:

   ```sh
   git checkout main && git pull
   git tag -m "brain-map 0.2.0" v0.2.0
   git push origin v0.2.0
   ```

   The tag needs a message because tags here are signed.

The workflow checks the tag against `Cargo.toml`, refuses to continue if they disagree,
runs the tests, builds on Arch, and publishes six files: the tarball, its checksum, and a
stamped `PKGBUILD` and `.SRCINFO` for each AUR package.

Then update the two AUR repositories with the files the release produced:

```sh
cd ~/aur/brain-map
cp <downloaded>/brain-map.PKGBUILD PKGBUILD
cp <downloaded>/brain-map.SRCINFO .SRCINFO
git commit -am "Update to 0.2.0" && git push
```

Same again in `~/aur/brain-map-bin` with the two `brain-map-bin` files. The Nix flake
needs nothing: it builds from whatever revision someone asks for.

## House rules

The ones worth knowing before you write code. The full list is in
[CLAUDE.md](CLAUDE.md), which the agents read.

- The scanner takes no dependencies. Not few, none. The standard library does all of it.
- `crates/app` never touches the filesystem. What the window needs from the machine is a
  method on `Source`, implemented in `src/main.rs`.
- An icon is written in a note, never guessed from its name or its folder.
- Colours come from the `THEMES` table, never from a literal in the renderer.
- A character the window draws must be in one of the four fonts egui bundles, and
  `chrome/glyph.rs` checks that against the real font files.

## When git will not talk to GitHub

This repository pushes over SSH, because a global git setting rewrites GitHub HTTPS URLs
to SSH. If the signing key is not to hand, push through the `gh` token instead:

```sh
git -c url.git@github.com:.insteadOf= -c credential.helper='!gh auth git-credential' \
    push https://github.com/joaopinto15/brain-map.git HEAD
```
