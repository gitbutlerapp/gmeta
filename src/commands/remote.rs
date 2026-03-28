use anyhow::{bail, Result};

use crate::git_utils;

pub fn run_add(url: &str, name: &str) -> Result<()> {
    let repo = git_utils::discover_repo()?;
    let ns = git_utils::get_namespace(&repo)?;

    // Check if this remote name already exists
    let existing = repo.remotes()?;
    for existing_name in existing.iter().flatten() {
        if existing_name == name {
            bail!("remote '{}' already exists", name);
        }
    }

    // Write git config entries for the meta remote
    let mut config = repo.config()?;
    let prefix = format!("remote.{}", name);

    config.set_str(&format!("{}.url", prefix), url)?;
    config.set_str(
        &format!("{}.fetch", prefix),
        &format!("+refs/{ns}/main:refs/{ns}/remotes/main"),
    )?;
    config.set_bool(&format!("{}.meta", prefix), true)?;
    config.set_bool(&format!("{}.promisor", prefix), true)?;
    config.set_str(&format!("{}.partialclonefilter", prefix), "blob:none")?;

    println!("Added meta remote '{}' -> {}", name, url);

    // Initial blobless fetch
    let fetch_refspec = format!("refs/{ns}/main:refs/{ns}/remotes/main");
    print!("Fetching metadata...");
    match git_utils::run_git(
        &repo,
        &["fetch", "--filter=blob:none", name, &fetch_refspec],
    ) {
        Ok(_) => {
            println!(" done.");

            // Hydrate tip tree blobs so we can read the metadata
            let remote_ref = format!("{ns}/remotes/main");
            let blob_list =
                git_utils::run_git(&repo, &["ls-tree", "-r", "--object-only", &remote_ref]);

            if let Ok(blobs) = blob_list {
                if !blobs.trim().is_empty() {
                    // Pipe blob OIDs into fetch to hydrate them
                    let workdir = repo
                        .workdir()
                        .or_else(|| Some(repo.path()))
                        .expect("cannot determine repository directory");

                    let mut child = std::process::Command::new("git")
                        .args([
                            "-c",
                            "fetch.negotiationAlgorithm=noop",
                            "fetch",
                            name,
                            "--no-tags",
                            "--no-write-fetch-head",
                            "--recurse-submodules=no",
                            "--filter=blob:none",
                            "--stdin",
                        ])
                        .current_dir(workdir)
                        .stdin(std::process::Stdio::piped())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::piped())
                        .spawn()?;

                    if let Some(mut stdin) = child.stdin.take() {
                        use std::io::Write;
                        stdin.write_all(blobs.as_bytes())?;
                    }

                    let output = child.wait_with_output()?;
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        eprintln!("Warning: blob hydration failed: {}", stderr.trim());
                    }
                }
            }
        }
        Err(e) => {
            eprintln!(
                "\nWarning: initial fetch failed (remote may not have metadata yet): {}",
                e
            );
            eprintln!("You can fetch later with: git fetch {name}");
        }
    }

    Ok(())
}

pub fn run_remove(name: &str) -> Result<()> {
    let repo = git_utils::discover_repo()?;
    let ns = git_utils::get_namespace(&repo)?;

    // Verify this is a meta remote
    let config = repo.config()?;
    let meta_key = format!("remote.{}.meta", name);
    match config.get_bool(&meta_key) {
        Ok(true) => {}
        _ => bail!("'{}' is not a metadata remote (no meta = true)", name),
    }

    // Remove the git config section for this remote
    let mut config = repo.config()?;
    config.remove_multivar(&format!("remote.{}.url", name), ".*")?;
    config.remove_multivar(&format!("remote.{}.fetch", name), ".*")?;
    let _ = config.remove_multivar(&format!("remote.{}.meta", name), ".*");
    let _ = config.remove_multivar(&format!("remote.{}.promisor", name), ".*");
    let _ = config.remove_multivar(&format!("remote.{}.partialclonefilter", name), ".*");

    // Delete refs under refs/{ns}/remotes/
    let ref_prefix = format!("refs/{}/remotes/", ns);
    let references: Vec<String> = repo
        .references_glob(&format!("{}*", ref_prefix))?
        .filter_map(|r| r.ok())
        .filter_map(|r| r.name().map(String::from))
        .collect();

    for refname in &references {
        let mut reference = repo.find_reference(refname)?;
        reference.delete()?;
        println!("Deleted ref {}", refname);
    }

    // Also delete refs under refs/{ns}/local/
    let local_prefix = format!("refs/{}/local/", ns);
    let local_refs: Vec<String> = repo
        .references_glob(&format!("{}*", local_prefix))?
        .filter_map(|r| r.ok())
        .filter_map(|r| r.name().map(String::from))
        .collect();

    for refname in &local_refs {
        let mut reference = repo.find_reference(refname)?;
        reference.delete()?;
        println!("Deleted ref {}", refname);
    }

    println!("Removed meta remote '{}'", name);
    Ok(())
}

pub fn run_list() -> Result<()> {
    let repo = git_utils::discover_repo()?;
    let remotes = git_utils::list_meta_remotes(&repo)?;

    if remotes.is_empty() {
        println!("No metadata remotes configured.");
        println!("Add one with: gmeta remote add <url>");
    } else {
        for (name, url) in &remotes {
            println!("{}\t{}", name, url);
        }
    }

    Ok(())
}
