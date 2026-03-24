# Shared principles

Across all value types, the following rules should hold.

## 1. Local storage vs exchange format

Locally, gmeta can store whatever representation is most efficient in SQLite.

For exchange, values are serialized into Git tree entries so they can be pushed, fetched, diffed, and merged using normal Git object and tree mechanics.

## 2. Current state and mutation history

The local database may track both:

- current materialized value state
- mutation history / provenance

The exchange format only needs to represent the latest shareable state needed to reconstruct current values.

## 3. Deletion is explicit

Missing data in a tree should generally **not** be interpreted as deletion.

This is important because:

- sparse or pruned trees may omit data
- different metadata refs may contain different subsets
- exchange should not make absence ambiguous

If a value or collection member is intentionally removed, that removal should be represented explicitly with a tombstone-like entry.

## 4. Key path reservation

Keys are serialized directly under the target base path; there is no dedicated `k/` subtree.

To keep the tree layout unambiguous, key segments must not start with `__`.
Any path component beginning with `__` is reserved for gmeta structural metadata such as:

- `__value`
- `__list`
- `__set`
- `__tombstones`

This means user keys can safely occupy normal path segments while gmeta can still recognize where value metadata begins.

## 5. Merge by independently addressable units

The main design strategy is to avoid large monolithic blobs for values that may be concurrently mutated.

In practice:

- strings merge at the whole-value level
- lists merge at the entry level
- sets merge at the member level

This preserves Git's strength at merging trees with many small paths rather than requiring custom blob-level merge algorithms for structured documents.

## 6. Materialize is deterministic

Given one or more metadata refs, materialization should deterministically compute the local current value from the serialized tree plus merge rules for the value type.

If two replicas materialize the same set of refs, they should compute the same current value.
