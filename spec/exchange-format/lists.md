# Lists

Lists in gmeta are append-friendly multi-value collections.

They are best thought of as an **ordered append log of string entries**, not as a general-purpose mutable array.

Examples:

- transcript chunks
- comments
- multiple owners where duplicates are acceptable
- a sequence of notes appended over time

## Summary

- A list is a sequence of entries
- Each entry is serialized independently
- Order is derived from entry names / timestamps
- Concurrent appends merge by union
- Removal of the entire key uses a key tombstone
- Optional removal of individual entries is possible, but should be explicit if supported

## Intended semantics

The current list design is optimized for the most common concurrent operation:

- one or more users append new values to the same logical list

This is why lists serialize as many small blobs rather than one JSON array blob.

This lets Git merge tree entries structurally and avoids array-level conflicts.

## Serialized tree shape

For a list value, entries are written under:

`[base]/<key segments>/__list/<entry-id>`

Where `<entry-id>` is of the form:

`<timestamp-ms>-<content-hash-prefix>`

Example:

`branch/06/sc-branch-1-deadbeef/agent/chat/__list/1771232450203-23c0f`

Each blob contains one string entry.

If multiple values are inserted in one command, start from one base millisecond timestamp and increment by 1 for each additional entry so local order is preserved.

## Ordering

List order is defined by sorting `<entry-id>` lexically, which is equivalent to sorting by:

1. timestamp ascending
2. hash suffix as a stable tie-breaker

This gives deterministic ordering after merges.

## Duplicate values

Lists allow duplicate string values.

If the same text is appended twice at different times, both entries remain.

This is intentional and differentiates lists from sets.

## Tombstones

### Whole-key removal

If the entire list key is removed, write a tombstone at:

`[base]/__tombstones/<key segments>/__deleted`

This means the logical list key has been removed.

### Entry removal

The current project spec has `list:pop`, but its exact exchange semantics should be explicit before implementation.

Two viable options exist:

1. **No per-entry serialized tombstones**
   - only the current list state is serialized
   - removed entries simply do not appear in the next tree
   - simple, but deletion is ambiguous under sparse or partial exchange
2. **Per-entry tombstones**
   - serialize removals of individual list entries explicitly
   - unambiguous, and consistent with the project's deletion philosophy

For now, this draft recommends **per-entry tombstones** if `list:pop` remains part of the model.

Suggested path:

`[base]/<key segments>/__tombstones/<entry-id>/__deleted`

with blob contents like:

```json
{
  "timestamp": 1771232450999,
  "email": "schacon@gmail.com"
}
```

This makes entry removal explicit and mergeable.

Because `__tombstones` is shared across collection types, serialize and materialize must ignore incompatible child tombstones for the current key type. For example, if a key currently materializes as a list, only tombstones that are valid list entry identifiers should affect the visible list state; incompatible child tombstones should be preserved but ignored for list semantics.

## Local materialized meaning

A list's current value is produced by:

1. collecting all serialized list entries for the key
2. removing any entry whose tombstone is newer than the entry itself
3. if a whole-key tombstone is newer than all surviving entries, the key is absent
4. otherwise sort surviving entries by entry id and return their blob contents

If the implementation decides not to support per-entry tombstones, then `list:pop` needs narrower semantics and should likely be treated as a local-only mutation until a safe exchange story is defined.

## Materialization scenarios

### Initial materialization

Walk all `__list/*` entries and optional `__tombstones/*` child entries:

- insert all entry rows into SQLite
- insert any entry tombstones
- insert any whole-key tombstone
- compute the current list by filtering tombstoned entries and sorting the survivors

### Fast-forward update

Diff old tree vs new tree:

- new `__list/*` paths become appended entries
- new entry tombstones remove prior entries if newer
- whole-key tombstones delete the logical key if newer than the current visible list state

This is efficient because only changed paths need to be examined.

### Multiple metadata sources

Union all entries and tombstones seen across refs for the same `(target, key)` and compute the visible list deterministically using the same filtering rules.

## Merge semantics

Lists should merge structurally by entry identity, not as a single blob.

### Concurrent append

If two users append different entries concurrently:

- both entries are kept
- final order is by entry id sort

This is the main reason for the format.

### Rare entry-id collision

If two entries somehow produce the exact same path but different blob ids:

- treat as a collision
- prefer one deterministically, likely remote during materialize

In practice this should be extremely rare because the path includes timestamp and content hash prefix.

A stricter implementation could simply lengthen the hash suffix until collisions are negligible.

### Concurrent append of identical value text

If two users append the same string text at slightly different times:

- both entries remain if entry ids differ
- the list contains duplicates

### Concurrent remove vs append of same entry

If per-entry tombstones are supported and one side removes an entry while the other adds a new different entry:

- keep the new entry
- remove the tombstoned one if the tombstone is newer than that entry

If one side removes a specific entry and the other side also still has that same entry unchanged:

- tombstone wins for that entry if newer than the entry metadata

### Whole-list delete vs append

If one side removes the whole list key and the other appends entries:

- newer of whole-key tombstone vs surviving entry activity wins

This mirrors string behavior at the collection level.

### No common ancestor

For a baseless two-way merge:

- union all list entries from both sides
- union all tombstones from both sides
- for overlapping entry ids, remote wins
- compute visible state from the resulting union

This gives append-friendly behavior even when histories started independently.

## What lists are and are not

Lists are good for:

- append-only or mostly-append data
- log-like metadata
- comments
- transcript chunks
- preserving duplicates

Lists are not a good fit for:

- enforcing uniqueness
- arbitrary insertion into the middle
- stable item identity for later moves
- heavy delete/reorder semantics

Those should use other value types.
