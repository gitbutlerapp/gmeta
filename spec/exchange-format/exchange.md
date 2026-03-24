# Exchange format and refs

This document describes how gmeta serializes metadata into Git and where it is stored.

## Goals

The exchange format should:

- work over existing Git transport
- use Git trees and commits directly
- diff efficiently
- merge structurally where possible
- reconstruct current shareable state during materialization

Only the latest shareable state needs to be serialized. Full local mutation history does not.

## Refs

Local serialized metadata head:

- `refs/meta/local`

If `meta.namespace` Git config is set, that namespace should be used instead of `meta`.

Fetched remote metadata heads should be stored under a remote-specific namespace, for example:

- `refs/meta/remotes/origin`

If multiple local metadata destinations are supported later, the local layout may expand to directory-shaped refs such as:

- `refs/meta/local/public`
- `refs/meta/local/private`

## Commit model

Serialization writes a Git commit whose tree contains the current shareable metadata state.

The commit message is not semantically important; the commit is mainly used for:

- tree pointer
- author identity
- author date / commit date
- ancestry for incremental materialization and merging

## Tree root layout

The base tree path for a target is:

`<target-type>/<fanout>/<full-target-value>`

Fanout is target-type dependent:

- for `commit`, use the first 2 characters of the commit SHA
- for all other target types with a target value, use the first 2 hexadecimal characters of the SHA-1 hash of the target value

Examples:

- `commit/13/13a7d29cde8f8557b54fd6474f547a56822180ae/...`
- `branch/06/sc-branch-1-deadbeef/...`

This keeps commit paths readable and naturally distributed by object id prefix, while still avoiding oversized top-level directories for arbitrary branch, path, or change-id targets.

`project` may be represented as a special well-known subtree because it has no user-facing target value.

## Key path layout

Under the target base path, key segments are serialized directly as path components.

Metadata structure begins when a reserved `__*` component is encountered.

Examples:

- string: `<base>/agent/model/__value`
- list: `<base>/agent/chat/__list/<entry-id>`
- whole-key tombstone: `<base>/__tombstones/agent/model/__deleted`

## Per-type layouts

Per-type layouts are defined in:

- [Strings](./strings.md)
- [Lists](./lists.md)
- [Sets](./sets.md)

## Serialization policy

Serialization takes local current state and writes a new Git tree/commit representing the latest shareable metadata view.

A later optimization may serialize only values changed since the last successful materialization or serialize by reusing unchanged subtrees.

## Why trees instead of structured blobs

The exchange format prefers many independently addressable paths over one large JSON blob because:

- Git diffs trees efficiently
- Git merges non-overlapping paths naturally
- list entries and set members can merge as unions instead of blob conflicts
- large append-only data can be chunked

## Explicit deletion

Exchange must not assume that missing paths mean deletion.

Reasons:

- sparse or pruned trees may omit data
- multiple metadata refs may represent different subsets
- absence should not be ambiguous

Intentional deletion is represented by explicit tombstones.

A single reserved `__tombstones` namespace is used for both whole-key and child-level deletions. Child tombstones are interpreted relative to the current key type. Serialize and materialize must ignore incompatible child tombstones for the current type rather than treating them as errors or as deletions for another collection model.

## Large-data considerations

This format is intended to work with blobless / partial clone workflows.

Large metadata histories can remain practical because:

- trees and commits are relatively small
- blobs can be fetched on demand
- recent or important working sets can be prioritized
- pruning strategies can reduce tip tree size without losing reconstructability of older introduced metadata

A future pruning/checkpoint scheme may periodically shrink the visible tip tree while retaining enough history to reconstruct older metadata when needed.
