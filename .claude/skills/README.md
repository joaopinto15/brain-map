# Vendored OKF skills

`okf/`, `validate/`, and `visualize/` are vendored from
[scaccogatto/okf-skills](https://github.com/scaccogatto/okf-skills), MIT licensed
(see `LICENSE`), pinned at commit `aa678799eb1018080bad42494fcaa83e77ff8fb5`.

They author, validate, and render Open Knowledge Format bundles — the format
brain-map reads. Re-vendor by copying `skills/` from a newer upstream checkout.

Upstream also ships as a Claude Code plugin (`/plugin install okf@scaccogatto`),
which updates itself; this copy is frozen so the repo carries its own tooling.
