# What brain-map reads from a vault

Every note is a node. What a note declares in its frontmatter decides its colour, its
size, its icon, and which legend rows it answers to. Nothing is guessed.

brain-map reads a vault as an
[Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md)
bundle, and OKF tells a reader to take a concept as it finds it. So any folder of `.md`
files works: an OKF bundle, an Obsidian vault, a pile of notes you never labelled.

## A note, fully dressed

```yaml
---
type: Metric
title: Gross margin
description: Revenue minus cost of goods, over revenue.
icon: 📊
tags: [finance, margin]
status: stable
resource: https://dash.example.com/margin
generated: { by: process:etl, at: 2026-06-01T08:00:00Z }
verified: { by: human:jp, at: 2026-06-25T09:00:00Z }
stale_after: 2026-12-31T00:00:00Z
sources:
  - id: orders
    title: Orders table
    resource: /tables/orders.md
---
```

Every key is optional. A note with no frontmatter at all is an `Untyped`, `unverified`
concept, and it draws like any other.

## Types

A concept has exactly one type: whatever `type:` says, or `Untyped`. The type decides the
node's colour, its size, and when it appears while the graph assembles, so the types
divide the vault and their counts add up to it.

Folders are structure, not type. A bundle files its concepts into directories however it
likes, and brain-map draws the tree without ever grouping on it. The reserved `index.md`
and `log.md` describe a directory rather than a concept, so they group as *Index & log*.

Why type and not folder is written up in
[ADR 0002](adr/0002-a-concept-is-grouped-by-its-type.md).

## Signals

Signals are what the frontmatter says about trusting a concept. Each one is read, never
inferred.

| Signal | Comes from |
|--------|------------|
| `human-reviewed` | A `human:` actor in `verified` |
| `machine-confirmed` | Any other actor in `verified` |
| `unverified` | Nothing has verified it |
| `draft`, `deprecated` | `status:`, when it left the default |
| `stale` | `stale_after:`, once the date has passed |

## Tags

Tags come from `tags:`. A concept can carry several or none, and they cut across types.

The legend cuts the graph these three ways, and clicking a row dims everything else.
Filter by type to see one region of the map, by signal to see what you can trust in it,
by tag to follow one thread through several.

## Icons

`icon:` is one emoji, and it is the only thing that puts one on a note. A note without
the key draws as a plain disc, and so does every folder.

There was a resolver here once, 158 lines against a 1,500-line keyword table, and it made
`machine-learning` a slot machine and `Service` a service dog. A missing icon is a line
in the note, not a rule in the code.

## Links

Edges come from `[[wikilinks]]` and markdown links, relative or bundle-absolute
(`[orders](/tables/orders.md)`). A link that resolves to a note in the vault becomes an
edge. An `http` or `https` link opens in your browser. Nothing else is a link.

A vault with no links still draws as a tree: vault, folder, note.

## What the reader shows

Open a note and you get what it declared, the way the format's own viewer shows it: the
`description`, the `resource` as a link, who `generated` and `verified` it, and the
`sources` it derives from. A source naming a note in this vault opens that note. A URL
opens outside. A scope descriptor is words.

Under those, read backwards off the graph, are the concepts that cite this one.

## The frontmatter parser

brain-map reads the YAML subset the sample bundles are written in: block and flow
mappings, lists, quoted and folded scalars, comments. No anchors, block scalars, or tags,
because no sample uses them and the scanner takes no dependencies.

Every value stays a string, so a timestamp is the text the author wrote. The v0.1
`timestamp` key still stands in for `generated.at`.

## What the scan skips

`.git`, `.obsidian`, `node_modules`, `.brain-map`, `__pycache__`, `.venv`, `venv`,
`dist`, `build`, `.next`, `.cache`, and any directory whose name starts with a dot.
