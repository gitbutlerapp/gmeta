## Notes

~ offload larger blobs to git references in sqlite (remove from sqlite file)
- git-ai import

- namespaces (local, shared, internal, etc - push targets (none, remote)
  - materialize targets too
  - on conflicts, which wins?

## Scenarios

- simple
  - user A adds a key, serializes, pushes to meta remote
  - user B fetches, materializes, adds a key, modifies the first key, pushes to remote
  - user A adds a third key
  - user A fetches and materializes, has all 3 keys

## Stuff Butler needs to do

- transfer metadata
