# Strings

Strings are the simplest gmeta value type.

A string represents one current scalar value for a `(target, key)` pair.

Examples:

- `agent:model = claude-4.6`
- `owner = schacon`
- `review:status = approved`

## Summary

- One current value per key
- Serialized as a single terminal blob
- Removal uses a key tombstone
- Concurrent updates resolve by last-writer-wins

## Serialized tree shape

For a string value, write the string to:

`[base]/<key segments>/__value`

Examples:

- `commit/13/<full-target>/agent/model/__value`
- `path/src/metrics/__target__/owner/__value`

The blob contents are the raw string value.

## Tombstones

If a string key is removed, serialize a tombstone at:

`[base]/__tombstones/<key segments>/__deleted`

The tombstone blob should store enough metadata to compare recency during materialization and merge decisions, for example:

```json
{
  "timestamp": 1771232450000,
  "email": "schacon@gmail.com"
}
```

When a string is set again after deletion, the tombstone is cleared locally and omitted from the next serialization.

## Local materialized meaning

For a given `(target, key)`, the current string state is derived as follows:

1. If there is a value and no tombstone, the current state is that string.
2. If there is a tombstone and no value, the current state is deleted / absent.
3. If both are present, compare timestamps:
   - if the value is newer, the string is present
   - if the tombstone is newer, the key is absent

This means deletion is explicit and can compete with a concurrent write.

## Materialization scenarios

### Initial materialization

If there is no local state, walk the incoming tree:

- create local rows for all `__value` blobs
- create local tombstones for all `__deleted` blobs
- compute current state using the recency rule above

### Fast-forward update

If a previous materialized metadata tree exists locally:

- diff old tree vs new tree
- update any changed string values
- add new keys
- apply newly introduced tombstones
- for keys that changed from value to tombstone or vice versa, use the newer timestamp

### Multiple metadata sources

If multiple refs are materialized into one local database, the current string is the result of applying the same recency rule across all observed value and tombstone records for that `(target, key)`.

## Merge semantics

Strings are intentionally coarse-grained.

If two users update the same string key concurrently, this is a real conflict and should resolve predictably.

### Three-way merge with common ancestor

Cases:

1. Only one side changed from the base:
   - take the changed side
2. Both sides changed to the same value:
   - take that value
3. Both sides changed to different values:
   - newer value wins
4. One side deleted, the other kept base unchanged:
   - delete wins if newer than existing value metadata
5. One side deleted, the other modified:
   - newer of tombstone vs value wins

Recommended tie-breaker order:

1. greater timestamp
2. if timestamps equal, prefer remote during materialize / pull
3. if still needed, stable lexical compare of commit id or blob id

### No common ancestor

If local and remote metadata histories were initialized independently:

- union all keys from both trees
- for overlapping string keys, remote wins
- if overlap is value vs tombstone, remote wins

This preserves the existing project-level policy for baseless two-way merges.

## Why strings are simple

Strings are stored as a single blob because they are expected to behave like a scalar setting, not a collaborative collection.

If users need concurrent append or union behavior, they should use a list or set instead.

## Practical implications

Strings are a good fit for:

- status values
- model names
- owners when there is only one logical owner
- configuration knobs
- identifiers

Strings are a poor fit for:

- comments
- transcripts
- reviewers where multiple users may add values concurrently
- anything that requires union semantics
