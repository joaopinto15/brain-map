# brain-map

An Open Knowledge Format bundle, drawn as a graph you can walk. One window, one process:
the whole program is scanning a vault and drawing what it found.

## Language

### What is on disk

**Vault**:
A folder of markdown notes, opened as one graph. It is the unit the program opens, scans
and watches, and brain-map reads it as an OKF bundle.
_Avoid_: workspace, library, collection, directory

**Bundle**:
A vault as OKF names it: a directory tree of concepts, distributed as a git repo, an
archive or a subdirectory. Same thing, their word — used when the spec is what is being
talked about.
_Avoid_: catalog, corpus, package

**Import**:
Opening a vault that is not on this machine yet: a git remote is cloned into the cache, an
rclone remote is copied there, and that folder is the vault from then on. It is the same
field, the same button and the same `Source` method as a path — there is no separate
importer, and no provider is named anywhere in the program.
_Avoid_: sync, download, fetch, clone

**Drive**:
A vault held by a storage service — Google Drive, OneDrive, Dropbox and the rest — reached
as `remote:path` through rclone, which holds the account. brain-map has no idea which
service it is.
_Avoid_: cloud, provider, backend, mount

**Note**:
One `.md` file inside a vault. Every note is a node.
_Avoid_: document, page, file, entry

**Concept**:
A note read as OKF reads it: one unit of knowledge, declaring what it is in frontmatter.
Every note is one, except the reserved `index.md` and `log.md`, which describe a directory
rather than a thing.
_Avoid_: entity, item, record, asset

**Frontmatter**:
The `---` block at the head of a note, read as YAML into a value tree. `title`, `tags` and
`icon` come from it, the OKF keys the graph judges a concept by — `type`, `status`,
`verified`, `stale_after` — and the ones the reader shows: `description`, `resource`,
`generated`, `sources`.
_Avoid_: metadata, header

### What a note belongs to

**Type**:
What a concept *is*, from its `type:` field — `Metric`, `BigQuery Table`, `Attested
Computation`. OKF registers none of them centrally, so it is whatever the concept says,
never derived and never guessed. A concept that declares none is `Untyped`.
_Avoid_: kind, class, category, schema

**Group**:
A type, as the graph draws it: one colour, one radius, one place in the growth sequence.
Exactly one per node, and the groups partition the vault, so their counts add up to it.
The structural nodes and the external files are the two groups that are not types.
_Avoid_: category, section, cluster, bucket

**Tag**:
What a concept is about, from the `tags:` field. A concept may carry several or none, and
tags cut across types. A tag is a word, never an emoji.
_Avoid_: label, keyword, topic

**Signal**:
What a concept's frontmatter says about how far to trust it and whether it still holds:
its trust tier (`human-reviewed`, `machine-confirmed`, `unverified`), a `status` that left
the default (`draft`, `deprecated`), and `stale` once `stale_after` has passed. Derived in
the scanner, where the clock is, and filtered on like a tag.
_Avoid_: badge, flag, state, score

**Actor**:
Who did something, in OKF's convention — `human:<id>`, `process:<id>`, or
`<producer>/<version>` for an agent — and when. `generated` names one, `verified` a list of
them, and the `human:` prefix is what makes a concept human-reviewed.
_Avoid_: author, user, agent, verifier

**Provenance**:
One `sources` entry: a material the concept derives from, named by a URL, a path into the
bundle, or a scope descriptor nothing can follow. The reader lists them under the body,
opening the ones it can.
_Avoid_: citation, reference, origin, source (which is the seam)

**Cited by**:
The concepts whose text links to this one — the graph's edges read backwards. Shown under
the body; never stored, always derived from the links that are already drawn.
_Avoid_: backlinks, inbound, referrers

A folder is none of these. A bundle organizes its concepts into directories however it
likes — one folder holds several types and a type spreads over several folders — so a
folder is structure, and structure is not what a concept is.

### What is drawn

**Graph**:
Groups, nodes and links — everything needed to draw a vault, and the only value the
scanner hands the window. Contains no paths to read and no decisions left to make.
_Avoid_: wire, model, data, payload, world

**Scan**:
The graph mid-construction, while nodes are still identified by path rather than by index.
The scanner's own working value; it becomes a Graph and is then gone.
_Avoid_: index, build, tree

**Node**:
A note, a folder or the vault itself, as one circle in the graph.
_Avoid_: vertex, point, dot, item

**Structural node**:
A node that is not a note — the vault node and the folder nodes. They are drawn and linked
but have no source to read, so clicking one closes the reader rather than filling it.
_Avoid_: synthetic node, virtual node, container

**Link**:
An edge between two nodes, read out of note text — a `[[wikilink]]`, or a markdown link
that is relative or bundle-absolute (`/tables/orders.md`) — or drawn structurally from a
folder to what it holds.
_Avoid_: edge, connection, reference, relation

**Icon**:
The one emoji a note declares in its `icon:` frontmatter key. It belongs to that note and
to nothing else — not to its tags, not to its folder — and a note that declares none has
no icon. Never inferred, never resolved: written or absent.
_Avoid_: glyph, symbol, badge, emoji

### What the window is doing

**Growth**:
The animation that assembles the vault node by node when it opens, paced per group and
finishing inside a budget you can set.
_Avoid_: intro, load animation, reveal, build

**Filter**:
A type, a signal or a tag chosen from the legend, which holds everything else dim until it
is cleared. Three axes: what a concept is, how far it is trusted, what it is about.
_Avoid_: selection, search, facet, highlight

**Lit**:
The set of nodes drawn bright while the rest of the graph is dimmed. A filter holds it; so
does hovering or clicking a node, which lights that node and its neighbours. There is one
lit set and one dimming.
_Avoid_: focus, active, highlighted, selected

**Reading**:
Having a note's source open in the panel beside the graph. Fullscreen is the same reading,
given the whole window.
_Avoid_: previewing, viewing, opening

**Explorer**:
The panel down the left: the vault as a folder tree, replaced by the note while reading.
_Avoid_: sidebar, tree view, navigator, panel

### The seam

**Source**:
Everything the window may ask of the machine — scan a vault, fingerprint it, read a note,
open one in an editor, show a folder dialog, list the drives, turn a typed path, git URL
or drive into a vault.
Implemented by
the scanner; nothing on the window's side of it touches the disk.
_Avoid_: backend, service, provider, repository, API

**Fingerprint**:
One number over every note's path, length and modified time. It moving is the only signal
that the vault changed.
_Avoid_: hash, checksum, version, revision
