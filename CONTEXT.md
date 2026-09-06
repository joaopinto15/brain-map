# brain-map

A folder of markdown notes, drawn as a graph you can walk. One window, one process: the
whole program is scanning a vault and drawing what it found.

## Language

### What is on disk

**Vault**:
A folder of markdown notes, opened as one graph. It is the unit the program opens, scans
and watches.
_Avoid_: workspace, library, collection, directory

**Import**:
Opening a vault that is not on this machine yet: a git URL is cloned into the cache and
the clone is the vault from then on. It is the same field, the same button and the same
`Source` method as a path — there is no separate importer.
_Avoid_: sync, download, fetch, clone

**Note**:
One `.md` file inside a vault. Every note is a node.
_Avoid_: document, page, file, entry

**Frontmatter**:
The `---` block at the head of a note. Only `title` and `tags` are read from it.
_Avoid_: metadata, header, YAML

### What a note belongs to

**Group**:
Where a note lives — its top-level folder, or its role in an AI Workshop OS vault. Exactly
one per note, and the groups partition the vault, so their counts add up to it. The group
decides a note's colour, its size and when it appears.
_Avoid_: category, section, type, cluster, bucket

**Tag**:
What a note is about, from the `tags:` field. A note may carry several or none, and tags
cut across folders. A tag is a word, never an emoji.
_Avoid_: label, keyword, topic

A note's `type:` frontmatter is neither. It is read past on purpose: what a note is
*about* is its tags, which filter on their own axis.

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
An edge between two nodes, read out of note text — a `[[wikilink]]` or a relative markdown
link — or drawn structurally from a folder to what it holds.
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
A group or a tag chosen from the legend, which holds everything else dim until it is
cleared.
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
open one in an editor, show a folder dialog, turn a typed path or git URL into a vault.
Implemented by
the scanner; nothing on the window's side of it touches the disk.
_Avoid_: backend, service, provider, repository, API

**Fingerprint**:
One number over every note's path, length and modified time. It moving is the only signal
that the vault changed.
_Avoid_: hash, checksum, version, revision
