# Third-party vendored crates

This directory records provenance of upstream source that was vendored into
the original fork tree.

The Mermaid layout stack (`mermaid-to-svg`, `dagre_rust`, `graphlib_rust`,
`ordered_hashmap`) — used by the diagram rendering path — was removed from
this tree during the fusion cleanup (2026-08), together with the pager crate
that consumed it.  The [`NOTICE`](./NOTICE) file remains as the record of what
was vendored and under which licenses.
