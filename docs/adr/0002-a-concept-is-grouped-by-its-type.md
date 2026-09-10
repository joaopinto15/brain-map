# A concept is grouped by its type

A note's group used to be where it lived: its top-level folder, or its role in an AI
Workshop OS vault. That was two layout adapters, two sets of rules, and a `type:`
frontmatter key deliberately read past. It is now one rule — a concept's group is the
`type` it declares — and brain-map reads every vault as an
[Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md)
bundle.

A folder is a filing decision the author made once; a type is what the concept says it is.
In an OKF bundle they disagree on purpose: `computations/`, `metrics/` and `policies/` each
hold several types, and a `Metric` turns up in three of them. Grouping by folder drew that
bundle as its directory listing, which the explorer already is.

## Considered options

- **Keep folders, add types as a second axis.** Two groupings, two legends, and the
  question of which one paints the node. The colour can only mean one thing.
- **Detect an OKF bundle and switch layouts.** A third adapter, and a detection rule to be
  wrong about — a bundle where half the concepts declare a type is still a bundle, and §11
  says to consume it anyway. Nothing detects: every vault is read as OKF, and one that
  declares nothing has a single `Untyped` group.
- **Group by type, always.** One rule, one legend axis, and the folders kept as the
  structural tree they already were.

## Consequences

`Layout::generic` and `Layout::aios` are gone, and with them `keeps_isolates` and the
`mode` string — an AIOS vault is now grouped by the types its notes declare, which for
most of them is `Untyped`. A vault of plain notes draws as one colour where it used to
draw one per folder. That is the price: the folder structure is still in the tree and the
explorer, but it no longer paints.

Trust came with it. `verified`, `status` and `stale_after` are read into *signals*, a
second filter axis beside tags, so the legend answers "how much of this bundle has a human
behind it" — which the old grouping had no way to ask. The tiers are derived on every
scan, never stored, because §5.3 defines them as a reading of `verified` and a stored
verdict goes stale the moment someone signs off.

The scanner still takes no dependencies. The frontmatter is read by `yaml.rs`, a parser
for the subset of YAML the sample bundles are written in — about 150 lines, because
`sources` is a list of mappings and `generated` is written both inline and as a block —
and `stale_after` is compared as an instant by 20 lines of calendar arithmetic. A YAML
crate would have read more of the language than any bundle uses; a datetime crate would
have read more of the calendar.
