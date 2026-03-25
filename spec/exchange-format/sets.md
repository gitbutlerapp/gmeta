# Sets

Sets are unordered collections of unique string members.

Unlike lists:

- order does not matter
- you cannot have duplicate values

Examples:

- labels
- code owners
- capabilities or feature flags

## Serialized tree shape

For a set key, store members under:

`[base]/<key segments>/__set`

Where `__set` is a tree of the values where the tree entry name is the SHA1 of the value.

Example:

`path/src/metrics/__target__/owners/__set`

So the `__set` entry might look something like this:

```
❯ git cat-file -p meta/local:path/src/metrics/__target__/owners/__set
100644 blob 0dd8[...]4b1b2a    0dd8[...]4b1b2a
100644 blob 1296[...]a463c7    1296[...]a463c7
```

Notice that the tree entry blob sha is the same as the tree entry name.

## Member tombstones

If a member is removed from the set, write:

`[base]/<key segments>/__tombstones/<member-sha>`

with contents of the deleted entry as the tree entry value.

## Whole-key tombstones

If the entire set key is removed, also support:

`[base]/__tombstones/<key segments>/__deleted`

This deletes the logical collection itself, not just one member. The tree of `__deleted` should be what `__set` was when it was deleted.
