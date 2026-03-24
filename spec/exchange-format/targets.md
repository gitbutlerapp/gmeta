# Targets and keys

This document defines what metadata can be attached to and how keys are structured.

## Target model

Every metadata value is scoped to a target.

A target has two conceptual parts:

- `target_type`
- `target_value`

Supported target types:

- `commit` — target value is a Git commit SHA
- `change-id` — target value is a UUID
- `branch` — target value is a branch UUID or name
- `path` — target value is a file or directory path in the project
- `project` — global project scope; no associated target value in the user-facing model

CLI syntax uses:

- `<type>:<value>` for targets with a value, for example `commit:13a7d29...`
- `project` for the global target

## Keys

Keys are arbitrary strings with optional namespace structure.

Examples:

- `owner`
- `agent:model`
- `agent:provider`
- `agent:claude:session-id`

Keys are split on `:` into path segments during serialization.

## Key validation

To keep the exchange tree layout unambiguous, keys are strictly validated.

Rules:

- key cannot be empty
- key segments cannot be empty
- key segments cannot be `.` or `..`
- key segments cannot contain `/` or null bytes
- key segments cannot start with `__`

The last rule reserves all `__*` path components for gmeta structural metadata.

## Key path reservation

Keys are serialized directly under the target base path.

There is no dedicated `k/` subtree.

Any path component beginning with `__` is reserved for gmeta structural paths such as:

- `__value`
- `__list`
- `__set`
- `__tombstones`
- `__target__` (used only as the separator inside serialized `path` targets)

This means user keys occupy normal path segments and metadata structure begins when a reserved `__*` path component is encountered.

## Path target encoding

`path` targets are serialized using their raw path segments rather than a hash-derived fanout prefix.

Shape:

`path/<escaped path segments...>/__target__/...`

Rules:

- each `/`-separated path component becomes one tree path component
- `__target__` marks the end of the target path and the beginning of key segments
- if a path segment begins with `__`, escape it by prefixing the segment with `~`
- if a path segment begins with `~`, also escape it by prefixing the segment with `~`

Examples:

- `path:src/metrics` → `path/src/metrics/__target__/...`
- `path:src/__generated/file.rs` → `path/src/~__generated/file.rs/__target__/...`
- `path:src/~scratch/file.rs` → `path/src/~~scratch/file.rs/__target__/...`

## Value types

Current and proposed value types:

- `string` — single scalar string value
- `list` — append-friendly ordered sequence of string entries
- `set` — unordered unique collection of string members

The per-type exchange semantics are defined in:

- [Strings](./strings.md)
- [Lists](./lists.md)
- [Sets](./sets.md)
