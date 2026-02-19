use anyhow::Result;
use chrono::Utc;
use std::collections::BTreeMap;

use crate::db::Db;
use crate::git_utils;

/// A parsed metadata entry from a Git tree.
#[derive(Debug, Clone)]
enum TreeValue {
    String(String),
    List(Vec<(String, String)>), // (entry_name, content)
}

pub fn run(remote: Option<&str>) -> Result<()> {
    let repo = git_utils::discover_repo()?;
    let db_path = git_utils::db_path(&repo)?;
    let db = Db::open(&db_path)?;

    let ns = git_utils::get_namespace(&repo)?;
    let local_ref_name = git_utils::local_ref(&repo)?;

    // Find remote refs to materialize
    let remote_refs = find_remote_refs(&repo, &ns, remote)?;

    if remote_refs.is_empty() {
        println!("no remote metadata refs found");
        return Ok(());
    }

    let email = git_utils::get_email(&repo)?;
    let now = Utc::now().timestamp_millis();

    for (ref_name, commit_oid) in &remote_refs {
        let commit = repo.find_commit(*commit_oid)?;
        let tree = commit.tree()?;

        // Parse the remote tree into metadata entries
        let remote_entries = parse_tree(&repo, &tree, "")?;

        // Read local tree for merge
        let local_entries = if let Ok(local_ref) = repo.find_reference(&local_ref_name) {
            if let Ok(local_commit) = local_ref.peel_to_commit() {
                let local_tree = local_commit.tree()?;
                parse_tree(&repo, &local_tree, "")?
            } else {
                BTreeMap::new()
            }
        } else {
            BTreeMap::new()
        };

        let remote_timestamp = commit.time().seconds();

        // Merge remote entries into local
        let merged = merge_entries(&local_entries, &remote_entries, remote_timestamp)?;

        // Update SQLite
        for ((target_type, target_value, key), tree_val) in &merged {
            match tree_val {
                TreeValue::String(s) => {
                    let json_val = serde_json::to_string(s)?;
                    db.set(
                        target_type,
                        target_value,
                        key,
                        &json_val,
                        "string",
                        &email,
                        now,
                    )?;
                }
                TreeValue::List(entries) => {
                    let items: Vec<String> = entries.iter().map(|(_, content)| content.clone()).collect();
                    let json_val = serde_json::to_string(&items)?;
                    db.set(
                        target_type,
                        target_value,
                        key,
                        &json_val,
                        "list",
                        &email,
                        now,
                    )?;
                }
            }
        }

        // Merge the remote tree into local ref
        merge_into_local_ref(&repo, &local_ref_name, *commit_oid, &email)?;

        println!("materialized {}", ref_name);
    }

    db.set_last_materialized(now)?;

    Ok(())
}

fn find_remote_refs(
    repo: &git2::Repository,
    ns: &str,
    remote: Option<&str>,
) -> Result<Vec<(String, git2::Oid)>> {
    let mut results = Vec::new();

    let refs = repo.references()?;
    let prefix = match remote {
        Some(r) => format!("refs/{}/{}", ns, r),
        None => format!("refs/{}/", ns),
    };

    for reference in refs {
        let reference = reference?;
        if let Some(name) = reference.name() {
            if name.starts_with(&prefix) && name != format!("refs/{}/local", ns) {
                if let Ok(commit) = reference.peel_to_commit() {
                    results.push((name.to_string(), commit.id()));
                }
            }
        }
    }

    Ok(results)
}

/// Parse a Git tree into metadata entries.
/// Returns a map of (target_type, target_value, key) -> TreeValue
fn parse_tree(
    repo: &git2::Repository,
    tree: &git2::Tree,
    prefix: &str,
) -> Result<BTreeMap<(String, String, String), TreeValue>> {
    let mut result = BTreeMap::new();

    // Walk the tree recursively and collect all blob paths
    let mut paths: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    collect_blobs(repo, tree, prefix, &mut paths)?;

    // Group paths by target and key, detecting lists
    for (path, content) in &paths {
        let parts: Vec<&str> = path.split('/').collect();

        if parts.is_empty() {
            continue;
        }

        let (target_type, target_value, key_parts) = parse_path_parts(&parts)?;

        if key_parts.is_empty() {
            continue;
        }

        // Check if the last blob name looks like a list entry
        let last = *key_parts.last().unwrap();
        if git_utils::is_list_entry_name(last) {
            // This is a list entry
            let key = key_parts[..key_parts.len() - 1].join(":");
            let content_str = String::from_utf8_lossy(content).to_string();
            let entry = result
                .entry((target_type, target_value, key))
                .or_insert_with(|| TreeValue::List(Vec::new()));
            if let TreeValue::List(ref mut list) = entry {
                list.push((last.to_string(), content_str));
            }
        } else {
            // String value
            let key = key_parts.join(":");
            let content_str = String::from_utf8_lossy(content).to_string();
            result.insert(
                (target_type, target_value, key),
                TreeValue::String(content_str),
            );
        }
    }

    // Sort list entries by name (timestamp-hash)
    for value in result.values_mut() {
        if let TreeValue::List(ref mut list) = value {
            list.sort_by(|a, b| a.0.cmp(&b.0));
        }
    }

    Ok(result)
}

fn collect_blobs(
    repo: &git2::Repository,
    tree: &git2::Tree,
    prefix: &str,
    paths: &mut BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    for entry in tree.iter() {
        let name = entry.name().unwrap_or("");
        let full_path = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}/{}", prefix, name)
        };

        match entry.kind() {
            Some(git2::ObjectType::Blob) => {
                let blob = repo.find_blob(entry.id())?;
                paths.insert(full_path, blob.content().to_vec());
            }
            Some(git2::ObjectType::Tree) => {
                let subtree = repo.find_tree(entry.id())?;
                collect_blobs(repo, &subtree, &full_path, paths)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Parse path segments into (target_type, target_value, key_parts).
/// Handles the sharding format: type/first2/last3/full_value/key/parts
/// And project format: project/key/parts
fn parse_path_parts<'a>(parts: &'a [&'a str]) -> Result<(String, String, &'a [&'a str])> {
    if parts.is_empty() {
        anyhow::bail!("empty path");
    }

    let target_type = parts[0];

    if target_type == "project" {
        return Ok(("project".to_string(), "".to_string(), &parts[1..]));
    }

    // Sharded format: type/first2/last3/full_value/key_parts...
    if parts.len() < 4 {
        anyhow::bail!("path too short for sharded target: {:?}", parts);
    }

    // parts[1] = first 2 chars, parts[2] = last 3 chars, parts[3] = full value
    let target_value = parts[3].to_string();

    Ok((target_type.to_string(), target_value, &parts[4..]))
}

/// Merge remote entries into local entries.
fn merge_entries(
    local: &BTreeMap<(String, String, String), TreeValue>,
    remote: &BTreeMap<(String, String, String), TreeValue>,
    _remote_timestamp: i64,
) -> Result<BTreeMap<(String, String, String), TreeValue>> {
    let mut merged = local.clone();

    for (key, remote_val) in remote {
        match (merged.get(key), remote_val) {
            // Both have lists: union of entries
            (Some(TreeValue::List(local_list)), TreeValue::List(remote_list)) => {
                let mut combined: BTreeMap<String, String> = BTreeMap::new();
                for (name, content) in local_list {
                    combined.insert(name.clone(), content.clone());
                }
                for (name, content) in remote_list {
                    combined.entry(name.clone()).or_insert_with(|| content.clone());
                }
                let list: Vec<(String, String)> = combined.into_iter().collect();
                merged.insert(key.clone(), TreeValue::List(list));
            }
            // Both have strings: remote wins (latest commit timestamp)
            (Some(TreeValue::String(_)), TreeValue::String(_)) => {
                // Remote wins as it's the incoming change
                merged.insert(key.clone(), remote_val.clone());
            }
            // Remote has value, local doesn't: take remote
            (None, _) => {
                merged.insert(key.clone(), remote_val.clone());
            }
            // Mismatched types: remote wins
            _ => {
                merged.insert(key.clone(), remote_val.clone());
            }
        }
    }

    Ok(merged)
}

/// Merge a remote commit into the local ref (create merge commit or new commit).
fn merge_into_local_ref(
    repo: &git2::Repository,
    local_ref_name: &str,
    remote_oid: git2::Oid,
    email: &str,
) -> Result<()> {
    let remote_commit = repo.find_commit(remote_oid)?;

    // Rebuild tree from merged metadata would be complex,
    // so we use the remote's tree as the new tree for simplicity.
    // A full implementation would rebuild from the merged DB state.
    let remote_tree = remote_commit.tree()?;

    let sig = git2::Signature::now(email, email)?;

    let local_commit = repo
        .find_reference(local_ref_name)
        .ok()
        .and_then(|r| r.peel_to_commit().ok());

    let parents: Vec<&git2::Commit> = match &local_commit {
        Some(c) => vec![c, &remote_commit],
        None => vec![&remote_commit],
    };

    repo.commit(
        Some(local_ref_name),
        &sig,
        &sig,
        "materialize",
        &remote_tree,
        &parents,
    )?;

    Ok(())
}
