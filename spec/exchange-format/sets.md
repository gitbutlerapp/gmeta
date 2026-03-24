# Sets

Sets are unordered collections of unique string members.

Unlike lists:

- order does not matter
- duplicates collapse to one logical member
- add/remove operations should merge at the member level

Examples:

- reviewers
- labels
- code owners when uniqueness matters
- capabilities or feature flags

## Recommended model

This draft recommends a **member-addressed set with explicit member tombstones**.

This is intentionally simpler than a full CRDT OR-Set, while still matching the project's explicit-deletion philosophy.

Semantically, each member behaves like a tiny string-valued presence record inside the set.

## Summary

- Each member is serialized independently
- Membership is keyed by normalized member value
- Removal is explicit per member
- Concurrent adds of different members union cleanly
- Concurrent add/remove of the same member resolves by recency

## Serialized tree shape

For a set key, store members under:

`[base]/<key segments>/__set/<member-id>/__value`

Where `<member-id>` is a stable identifier derived from the member string.

Recommended:

- `member-id = sha256(member-string)`

The `__value` blob contains the raw original string member.

Example:

`path/83/src/metrics/owners/__set/8f14e45fceea167a5a36dedd4bea2543.../__value`

This avoids path encoding issues and makes lookup stable.

## Member tombstones

If a member is removed from the set, write:

`[base]/<key segments>/__tombstones/<member-id>/__deleted`

with contents like:

```json
{
  "timestamp": 1771232450999,
  "email": "schacon@gmail.com"
}
```

The materializer can recompute `member-id` from the string value during local operations, so remove operations are straightforward.

Because `__tombstones` is shared across collection types, serialize and materialize must ignore incompatible child tombstones for the current key type. For example, if a key currently materializes as a set, only tombstones that are valid set member identifiers should affect visible membership; incompatible child tombstones should be preserved but ignored for set semantics.

## Whole-key tombstones

If the entire set key is removed, also support:

`[base]/__tombstones/<key segments>/__deleted`

This deletes the logical collection itself, not just one member.

## Local materialized meaning

To compute the visible set for a `(target, key)`:

1. collect all member `__value` entries
2. collect all member tombstones
3. for each member id:
   - if only value exists, member is present
   - if only tombstone exists, member is absent
   - if both exist, newer of value vs tombstone wins
4. if a whole-key tombstone is newer than all visible members, the set key is absent
5. otherwise return the unique surviving member strings in deterministic order for display

Because sets are unordered semantically, display order should be normalized, for example lexical sort of member string.

## Materialization scenarios

### Initial materialization

Walk:

- all `__set/*/__value` entries
- all `__tombstones/*/__deleted` child entries
- any whole-key tombstone

Then compute visible members using the recency rule.

### Fast-forward update

Diff old tree vs new tree:

- new member paths add members
- new member tombstones remove members if newer
- new whole-key tombstones remove the collection if newer than surviving member writes

### Multiple metadata sources

Union all member values and member tombstones observed across refs, then derive the current set deterministically.

## Merge semantics

Sets should merge at the member level.

This is the main reason to model them separately from strings or JSON arrays.

### Concurrent add of different members

If two users add different members concurrently:

- both members remain in the final set

### Concurrent add of same member

If two users add the same member concurrently:

- the final set still contains only one logical member
- if metadata differs, newer metadata wins

Because membership is keyed by normalized member value, this is naturally deduplicated.

### Concurrent remove of different members

If two users remove different members concurrently:

- both removals apply

### Concurrent add vs remove of same member

If one user adds a member and another removes that same member concurrently:

- newer of member value vs member tombstone wins

This is a last-writer-wins set policy.

It is simpler than an OR-Set and likely good enough for the project's intended metadata use cases.

### Whole-set delete vs member add

If one side deletes the whole set key and the other adds a member:

- newer of whole-key tombstone vs member add activity wins

### No common ancestor

For a baseless two-way merge:

- union all member values from both trees
- union all member tombstones from both trees
- for overlapping member ids, remote wins
- compute visible membership from the resulting union

This preserves the project's existing no-common-ancestor behavior while still allowing non-conflicting members to union cleanly.

## Why not store sets as JSON arrays?

A JSON array would create whole-value conflicts for what should be independent member-level operations.

Example:

- one user adds `schacon`
- another adds `caleb`

With a JSON array blob, both modified the same file and require custom merge logic.
With a member-addressed set, both just added different tree entries and Git can merge them structurally.

## Why not use a full OR-Set yet?

A full OR-Set would model adds and removes as operation tags and gives more formal semantics for concurrent add/remove races.

However, it is significantly more complex:

- more paths
- more replay logic
- more storage growth
- likely a need for periodic compaction

For gmeta's current goals, a member-level last-writer-wins set is a better first design point.

If later use cases need stronger replicated-set guarantees, this format could evolve or a new set variant could be added.

## Good fits for sets

Sets are good for:

- unique owners
- reviewers
- labels
- policy flags
- membership-like metadata

Sets are not good for:

- preserving insertion order
- preserving duplicates
- transcript-like logs
- arbitrary ranking or reordering
