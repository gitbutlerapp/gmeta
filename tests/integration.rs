use assert_cmd::Command;
use predicates::prelude::*;
use std::path::Path;
use tempfile::TempDir;

/// Create a temporary git repo and return the TempDir handle.
fn setup_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();

    // Set up user config so commands can read email
    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    config.set_str("user.name", "Test User").unwrap();

    // Create an initial commit so the repo is valid
    let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
    let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
        .unwrap();

    dir
}

fn gmeta(dir: &Path) -> Command {
    let mut cmd = Command::cargo_bin("gmeta").unwrap();
    cmd.current_dir(dir);
    cmd
}

#[test]
fn test_set_and_get_string() {
    let dir = setup_repo();

    gmeta(dir.path())
        .args(["set", "commit:abc123def", "agent:model", "claude-4.6"])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["get", "commit:abc123def"])
        .assert()
        .success()
        .stdout(predicate::str::contains("agent:model"))
        .stdout(predicate::str::contains("claude-4.6"));
}

#[test]
fn test_set_and_get_specific_key() {
    let dir = setup_repo();

    gmeta(dir.path())
        .args(["set", "commit:abc123def", "agent:model", "claude-4.6"])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["set", "commit:abc123def", "agent:provider", "anthropic"])
        .assert()
        .success();

    // Get specific key
    gmeta(dir.path())
        .args(["get", "commit:abc123def", "agent:model"])
        .assert()
        .success()
        .stdout(predicate::str::contains("claude-4.6"))
        .stdout(predicate::str::contains("provider").not());
}

#[test]
fn test_set_and_get_json() {
    let dir = setup_repo();

    gmeta(dir.path())
        .args(["set", "commit:abc123def", "agent:model", "claude-4.6"])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["set", "commit:abc123def", "agent:provider", "anthropic"])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["get", "--json", "commit:abc123def"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"model\": \"claude-4.6\""))
        .stdout(predicate::str::contains("\"provider\": \"anthropic\""));
}

#[test]
fn test_json_with_authorship() {
    let dir = setup_repo();

    gmeta(dir.path())
        .args(["set", "commit:abc123def", "agent:model", "claude-4.6"])
        .assert()
        .success();

    gmeta(dir.path())
        .args([
            "get",
            "--json",
            "--with-authorship",
            "commit:abc123def",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"value\": \"claude-4.6\""))
        .stdout(predicate::str::contains("\"author\": \"test@example.com\""))
        .stdout(predicate::str::contains("\"timestamp\""));
}

#[test]
fn test_partial_key_matching() {
    let dir = setup_repo();

    gmeta(dir.path())
        .args(["set", "commit:abc123def", "agent:model", "claude-4.6"])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["set", "commit:abc123def", "agent:provider", "anthropic"])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["set", "commit:abc123def", "other:key", "value"])
        .assert()
        .success();

    // Partial key "agent" should match both agent: keys
    gmeta(dir.path())
        .args(["get", "commit:abc123def", "agent"])
        .assert()
        .success()
        .stdout(predicate::str::contains("agent:model"))
        .stdout(predicate::str::contains("agent:provider"))
        .stdout(predicate::str::contains("other:key").not());
}

#[test]
fn test_rm_removes_value() {
    let dir = setup_repo();

    gmeta(dir.path())
        .args(["set", "commit:abc123def", "agent:model", "claude-4.6"])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["rm", "commit:abc123def", "agent:model"])
        .assert()
        .success();

    // Should produce no output now
    gmeta(dir.path())
        .args(["get", "commit:abc123def", "agent:model"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_list_push() {
    let dir = setup_repo();

    gmeta(dir.path())
        .args(["list:push", "commit:abc123def", "tags", "first"])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["list:push", "commit:abc123def", "tags", "second"])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["get", "commit:abc123def", "tags"])
        .assert()
        .success()
        .stdout(predicate::str::contains("first"))
        .stdout(predicate::str::contains("second"));
}

#[test]
fn test_list_push_converts_string_to_list() {
    let dir = setup_repo();

    gmeta(dir.path())
        .args(["set", "commit:abc123def", "note", "original"])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["list:push", "commit:abc123def", "note", "appended"])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["get", "commit:abc123def", "note"])
        .assert()
        .success()
        .stdout(predicate::str::contains("original"))
        .stdout(predicate::str::contains("appended"));
}

#[test]
fn test_list_pop() {
    let dir = setup_repo();

    gmeta(dir.path())
        .args(["list:push", "commit:abc123def", "tags", "a"])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["list:push", "commit:abc123def", "tags", "b"])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["list:pop", "commit:abc123def", "tags", "b"])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["get", "commit:abc123def", "tags"])
        .assert()
        .success()
        .stdout(predicate::str::contains("a"))
        .stdout(predicate::str::contains("b").not());
}

#[test]
fn test_set_list_type() {
    let dir = setup_repo();

    gmeta(dir.path())
        .args([
            "set",
            "-t",
            "list",
            "commit:abc123def",
            "items",
            r#"["hello","world"]"#,
        ])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["get", "commit:abc123def", "items"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hello"))
        .stdout(predicate::str::contains("world"));
}

#[test]
fn test_serialize_creates_ref() {
    let dir = setup_repo();

    gmeta(dir.path())
        .args([
            "set",
            "commit:13a7d29cde8f8557b54fd6474f547a56822180ae",
            "agent:model",
            "claude-4.6",
        ])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["serialize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("refs/meta/local"));

    // Verify the ref exists and contains the right tree structure
    let repo = git2::Repository::open(dir.path()).unwrap();
    let reference = repo.find_reference("refs/meta/local").unwrap();
    let commit = reference.peel_to_commit().unwrap();
    let tree = commit.tree().unwrap();

    // Walk the tree and verify structure
    let mut found = false;
    tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
        let full_path = format!("{}{}", root, entry.name().unwrap_or(""));
        if full_path == "commit/13/0ae/13a7d29cde8f8557b54fd6474f547a56822180ae/agent/model" {
            // Verify blob content
            let blob = repo.find_blob(entry.id()).unwrap();
            let content = std::str::from_utf8(blob.content()).unwrap();
            assert_eq!(content, "claude-4.6");
            found = true;
        }
        git2::TreeWalkResult::Ok
    })
    .unwrap();

    assert!(found, "expected tree path not found in serialized tree");
}

#[test]
fn test_project_target() {
    let dir = setup_repo();

    gmeta(dir.path())
        .args(["set", "project", "name", "my-project"])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["get", "project", "name"])
        .assert()
        .success()
        .stdout(predicate::str::contains("my-project"));
}

#[test]
fn test_invalid_target_type() {
    let dir = setup_repo();

    gmeta(dir.path())
        .args(["set", "unknown:abc123", "key", "value"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown target type"));
}

#[test]
fn test_target_value_too_short() {
    let dir = setup_repo();

    gmeta(dir.path())
        .args(["set", "commit:ab", "key", "value"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("at least 3 characters"));
}

#[test]
fn test_serialize_list_values() {
    let dir = setup_repo();

    gmeta(dir.path())
        .args([
            "set",
            "-t",
            "list",
            "branch:sc-branch-1-deadbeef",
            "agent:chat",
            r#"["how's it going","pretty good"]"#,
        ])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["serialize"])
        .assert()
        .success();

    // Verify tree structure has list entries with timestamp-hash format
    let repo = git2::Repository::open(dir.path()).unwrap();
    let reference = repo.find_reference("refs/meta/local").unwrap();
    let commit = reference.peel_to_commit().unwrap();
    let tree = commit.tree().unwrap();

    let mut list_entries = Vec::new();
    tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
        let full_path = format!("{}{}", root, entry.name().unwrap_or(""));
        if full_path.starts_with("branch/sc/eef/sc-branch-1-deadbeef/agent/chat/") {
            if entry.kind() == Some(git2::ObjectType::Blob) {
                list_entries.push(full_path);
            }
        }
        git2::TreeWalkResult::Ok
    })
    .unwrap();

    assert_eq!(
        list_entries.len(),
        2,
        "expected 2 list entries, got: {:?}",
        list_entries
    );

    // Verify entry names follow timestamp-hash format
    for entry_path in &list_entries {
        let filename = entry_path.rsplit('/').next().unwrap();
        let parts: Vec<&str> = filename.split('-').collect();
        assert_eq!(parts.len(), 2, "list entry should be timestamp-hash: {}", filename);
        assert!(
            parts[0].chars().all(|c| c.is_ascii_digit()),
            "first part should be digits: {}",
            filename
        );
        assert_eq!(parts[1].len(), 5, "hash part should be 5 chars: {}", filename);
    }
}

#[test]
fn test_upsert_overwrites() {
    let dir = setup_repo();

    gmeta(dir.path())
        .args(["set", "commit:abc123def", "agent:model", "v1"])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["set", "commit:abc123def", "agent:model", "v2"])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["get", "commit:abc123def", "agent:model"])
        .assert()
        .success()
        .stdout(predicate::str::contains("v2"))
        .stdout(predicate::str::contains("v1").not());
}

#[test]
fn test_path_target() {
    let dir = setup_repo();

    gmeta(dir.path())
        .args(["set", "path:src/main.rs", "review:status", "approved"])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["get", "path:src/main.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("review:status"))
        .stdout(predicate::str::contains("approved"));
}

#[test]
fn test_change_id_target() {
    let dir = setup_repo();

    gmeta(dir.path())
        .args([
            "set",
            "change-id:550e8400-e29b-41d4-a716-446655440000",
            "status",
            "merged",
        ])
        .assert()
        .success();

    gmeta(dir.path())
        .args(["get", "change-id:550e8400-e29b-41d4-a716-446655440000"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("merged"));
}

#[test]
fn test_serialize_empty() {
    let dir = setup_repo();

    gmeta(dir.path())
        .args(["serialize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("no metadata to serialize"));
}
