use anyhow::Result;
use chrono::Utc;

use crate::db::Db;
use crate::git_utils;
use crate::types::{Target, ValueType};

pub fn run(target_str: &str, key: &str, value: &str, value_type_str: &str) -> Result<()> {
    let target = Target::parse(target_str)?;
    let value_type = ValueType::from_str(value_type_str)?;

    let repo = git_utils::discover_repo()?;
    let db_path = git_utils::db_path(&repo)?;
    let email = git_utils::get_email(&repo)?;
    let timestamp = Utc::now().timestamp_millis();

    let db = Db::open(&db_path)?;

    let stored_value = match value_type {
        ValueType::String => {
            // Store as JSON-encoded string
            serde_json::to_string(value)?
        }
        ValueType::List => {
            // Value should be a JSON array already; validate it
            let parsed: Vec<serde_json::Value> = serde_json::from_str(value)?;
            serde_json::to_string(&parsed)?
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

    Ok(())
}
