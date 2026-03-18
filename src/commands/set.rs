use anyhow::{bail, Context, Result};
use chrono::Utc;
use std::fs;

use crate::db::Db;
use crate::git_utils;
use crate::list_value::{encode_entries, parse_entries};
use crate::types::{validate_key, Target, ValueType, GIT_REF_THRESHOLD};

pub fn run(
    target_str: &str,
    key: &str,
    value: Option<&str>,
    file: Option<&str>,
    value_type_str: &str,
) -> Result<()> {
    let mut target = Target::parse(target_str)?;
    validate_key(key)?;
    let value_type = ValueType::from_str(value_type_str)?;

    let from_file = file.is_some();
    let raw_value = match (value, file) {
        (Some(_), Some(_)) => bail!("cannot specify both a value and -F/--file"),
        (None, None) => bail!("must specify either a value or -F/--file"),
        (Some(v), None) => v.to_string(),
        (None, Some(path)) => {
            fs::read_to_string(path).with_context(|| format!("failed to read file: {}", path))?
        }
    };

    let repo = git_utils::discover_repo()?;
    target.resolve(&repo)?;
    let db_path = git_utils::db_path(&repo)?;
    let email = git_utils::get_email(&repo)?;
    let timestamp = Utc::now().timestamp_millis();

    let db = Db::open(&db_path)?;

    // For large file imports (>1KB via -F), store as a git blob reference
    let use_git_ref =
        from_file && matches!(value_type, ValueType::String) && raw_value.len() > GIT_REF_THRESHOLD;

    if use_git_ref {
        let blob_oid = repo.blob(raw_value.as_bytes())?;
        db.set_with_git_ref(
            None,
            target.type_str(),
            target.value_str(),
            key,
            &blob_oid.to_string(),
            value_type.as_str(),
            &email,
            timestamp,
            true,
        )?;
    } else {
        let stored_value = match value_type {
            ValueType::String => {
                // Store as JSON-encoded string
                serde_json::to_string(&raw_value)?
            }
            ValueType::List => {
                let entries = parse_entries(&raw_value)?;
                encode_entries(&entries)?
            }
        };

        db.set(
            target.type_str(),
            target.value_str(),
            key,
            &stored_value,
            value_type.as_str(),
            &email,
            timestamp,
        )?;
    }

    Ok(())
}
