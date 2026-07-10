//! Read-only extraction of OpenCode's internal SQLite usage records.

mod analytics;
mod pricing;
mod server;

pub use analytics::{
    AnalyticsStore, ImportSummary, ModelSummary, ModelUsage, PeriodUsage, ProjectSummary,
    Reconciliation, SessionDetail, SessionUsage, Turn, UsageFilter,
};
pub use pricing::{ModelPricing, PricingCatalog};
pub use server::serve_default;

use std::{
    collections::BTreeSet,
    env,
    path::{Path, PathBuf},
    time::Duration,
};

use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

const REQUIRED_SCHEMA: &[(&str, &[&str])] = &[
    ("project", &["id", "worktree", "name"]),
    (
        "session",
        &[
            "id",
            "project_id",
            "title",
            "model",
            "cost",
            "tokens_input",
            "tokens_output",
            "tokens_reasoning",
            "tokens_cache_read",
            "tokens_cache_write",
            "time_created",
            "time_updated",
        ],
    ),
    (
        "message",
        &["id", "session_id", "data", "time_created", "time_updated"],
    ),
    (
        "part",
        &[
            "id",
            "message_id",
            "session_id",
            "data",
            "time_created",
            "time_updated",
        ],
    ),
];

#[derive(Debug, Error)]
pub enum Error {
    #[error("could not determine an OpenCode database path: {0}")]
    DataPath(String),
    #[error("database does not exist: {}", .0.display())]
    MissingDatabase(PathBuf),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("incompatible OpenCode database schema: missing {0}")]
    IncompatibleSchema(String),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not parse pricing file: {0}")]
    PricingParse(#[from] serde_yaml::Error),
    #[error("token count exceeds SQLite's signed integer range: {0}")]
    TokenCount(u64),
    #[error("aggregation period must be greater than zero")]
    InvalidPeriod,
    #[error("invalid configuration: {0}")]
    Configuration(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Model {
    pub provider_id: String,
    pub model_id: String,
    pub variant: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Usage {
    pub cost: Option<f64>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub total_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: String,
    pub name: Option<String>,
    pub worktree: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub id: String,
    pub project: Project,
    pub title: String,
    pub model: Option<Model>,
    pub usage: Usage,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantMessage {
    pub id: String,
    pub session_id: String,
    pub parent_id: Option<String>,
    pub model: Model,
    pub usage: Usage,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserMessage {
    pub id: String,
    pub session_id: String,
    pub text: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompletedStep {
    pub id: String,
    pub message_id: String,
    pub session_id: String,
    pub types: Vec<String>,
    pub reason: Option<String>,
    pub usage: Usage,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParseIssue {
    pub record_type: String,
    pub id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Extraction {
    pub source: PathBuf,
    pub sessions: Vec<Session>,
    pub assistant_messages: Vec<AssistantMessage>,
    pub user_messages: Vec<UserMessage>,
    pub steps: Vec<CompletedStep>,
    pub issues: Vec<ParseIssue>,
}

/// Resolve OpenCode's database according to its current documented path rules.
///
/// `OPENCODE_BASE_PATH` overrides the directory containing `opencode.db`, which
/// is useful when the OpenCode data directory is mounted into a container.
pub fn default_database_path() -> Result<PathBuf, Error> {
    let data_dir = opencode_data_dir()?;

    match env::var_os("OPENCODE_DB") {
        Some(value) if value == ":memory:" => Err(Error::DataPath(
            "OPENCODE_DB=:memory: has no persisted usage data".into(),
        )),
        Some(value) => {
            let path = PathBuf::from(value);
            Ok(if path.is_absolute() {
                path
            } else {
                data_dir.join(path)
            })
        }
        None => Ok(data_dir.join("opencode.db")),
    }
}

fn opencode_data_dir() -> Result<PathBuf, Error> {
    match env::var_os("OPENCODE_BASE_PATH") {
        Some(path) if !path.is_empty() => Ok(PathBuf::from(path)),
        Some(_) => Err(Error::DataPath(
            "OPENCODE_BASE_PATH must not be empty".into(),
        )),
        None => env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
            .map(|data_dir| data_dir.join("opencode"))
            .ok_or_else(|| Error::DataPath("set HOME or XDG_DATA_HOME".into())),
    }
}

/// Resolve the application-owned database used for incremental analytics imports.
pub fn default_analytics_path() -> Result<PathBuf, Error> {
    let data_dir = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .ok_or_else(|| Error::DataPath("set HOME or XDG_DATA_HOME".into()))?;
    Ok(data_dir.join("ocstats").join("analytics.db"))
}

pub fn extract_default() -> Result<Extraction, Error> {
    extract_from_path(default_database_path()?)
}

/// Verify that the configured OpenCode database can be opened read-only.
pub fn check_default_database() -> Result<(), Error> {
    check_database_path(default_database_path()?)
}

pub fn extract_from_path(path: impl AsRef<Path>) -> Result<Extraction, Error> {
    let path = path.as_ref().to_owned();
    let connection = open_database(&path)?;
    validate_schema(&connection)?;
    extract(&connection, path)
}

pub(crate) fn check_database_path(path: impl AsRef<Path>) -> Result<(), Error> {
    open_database(path.as_ref()).map(|_| ())
}

fn open_database(path: &Path) -> Result<Connection, Error> {
    if !path.is_file() {
        return Err(Error::MissingDatabase(path.to_owned()));
    }
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    connection.busy_timeout(Duration::from_secs(5))?;
    Ok(connection)
}

pub fn validate_schema(connection: &Connection) -> Result<(), Error> {
    let tables = connection
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<BTreeSet<_>, _>>()?;

    let mut missing = Vec::new();
    for (table, columns) in REQUIRED_SCHEMA {
        if !tables.contains(*table) {
            missing.push(format!("table {table}"));
            continue;
        }
        let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
        let found = statement
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<BTreeSet<_>, _>>()?;
        for column in *columns {
            if !found.contains(*column) {
                missing.push(format!("column {table}.{column}"));
            }
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(Error::IncompatibleSchema(missing.join(", ")))
    }
}

fn extract(connection: &Connection, source: PathBuf) -> Result<Extraction, Error> {
    let mut result = Extraction {
        source,
        ..Extraction::default()
    };
    extract_sessions(connection, &mut result)?;
    extract_messages(connection, &mut result)?;
    extract_steps(connection, &mut result)?;
    Ok(result)
}

fn extract_sessions(connection: &Connection, result: &mut Extraction) -> Result<(), Error> {
    let mut statement = connection.prepare(
        "SELECT s.id, s.title, s.model, s.cost, s.tokens_input, s.tokens_output, s.tokens_reasoning,
                s.tokens_cache_read, s.tokens_cache_write, s.time_created, s.time_updated,
                p.id, p.name, p.worktree
         FROM session s JOIN project p ON p.id = s.project_id ORDER BY s.time_created, s.id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(Session {
            id: row.get(0)?,
            title: row.get(1)?,
            model: parse_model(row.get::<_, Option<String>>(2)?),
            usage: Usage {
                cost: row.get(3)?,
                input_tokens: nonnegative(row.get(4)?),
                output_tokens: nonnegative(row.get(5)?),
                reasoning_tokens: nonnegative(row.get(6)?),
                cache_read_tokens: nonnegative(row.get(7)?),
                cache_write_tokens: nonnegative(row.get(8)?),
                total_tokens: None,
            },
            created_at_ms: row.get(9)?,
            updated_at_ms: row.get(10)?,
            project: Project {
                id: row.get(11)?,
                name: row.get(12)?,
                worktree: row.get(13)?,
            },
        })
    })?;
    result.sessions = rows.collect::<Result<_, _>>()?;
    Ok(())
}

fn extract_messages(connection: &Connection, result: &mut Extraction) -> Result<(), Error> {
    let mut statement = connection.prepare("SELECT id, session_id, data, time_created, time_updated FROM message ORDER BY time_created, id")?;
    for row in statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get(3)?,
            row.get(4)?,
        ))
    })? {
        let (id, session_id, raw, created_at_ms, updated_at_ms) = row?;
        let Ok(data) = serde_json::from_str::<Value>(&raw) else {
            issue(result, "message", id, "invalid JSON");
            continue;
        };
        if data.get("role").and_then(Value::as_str) != Some("assistant") {
            if data.get("role").and_then(Value::as_str) == Some("user") {
                result.user_messages.push(UserMessage {
                    id: id.clone(),
                    session_id: session_id.clone(),
                    text: message_text(connection, &id)?,
                    created_at_ms,
                    updated_at_ms,
                });
            }
            continue;
        }
        let Some(model) = model_from_value(&data) else {
            issue(
                result,
                "message",
                id,
                "assistant message lacks provider/model",
            );
            continue;
        };
        let Some(usage) = usage_from_value(&data) else {
            issue(result, "message", id, "assistant message lacks valid usage");
            continue;
        };
        result.assistant_messages.push(AssistantMessage {
            id,
            session_id,
            parent_id: data
                .get("parentID")
                .and_then(Value::as_str)
                .map(str::to_owned),
            model,
            usage,
            created_at_ms,
            updated_at_ms,
        });
    }
    Ok(())
}

fn extract_steps(connection: &Connection, result: &mut Extraction) -> Result<(), Error> {
    let mut statement = connection.prepare("SELECT id, message_id, session_id, data, time_created, time_updated FROM part ORDER BY time_created, id")?;
    for row in statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get(2)?,
            row.get::<_, String>(3)?,
            row.get(4)?,
            row.get(5)?,
        ))
    })? {
        let (id, message_id, session_id, raw, created_at_ms, updated_at_ms) = row?;
        let Ok(data) = serde_json::from_str::<Value>(&raw) else {
            issue(result, "part", id, "invalid JSON");
            continue;
        };
        if data.get("type").and_then(Value::as_str) != Some("step-finish") {
            continue;
        }
        let Some(usage) = usage_from_value(&data) else {
            issue(result, "part", id, "step-finish part lacks valid usage");
            continue;
        };
        result.steps.push(CompletedStep {
            id,
            message_id: message_id.clone(),
            session_id,
            types: part_types(connection, &message_id)?,
            reason: data
                .get("reason")
                .and_then(Value::as_str)
                .map(str::to_owned),
            usage,
            created_at_ms,
            updated_at_ms,
        });
    }
    Ok(())
}

fn message_text(connection: &Connection, message_id: &str) -> Result<String, Error> {
    let mut statement = connection.prepare(
        "SELECT data FROM part WHERE message_id = ?1 AND json_extract(data, '$.type') = 'text'
         ORDER BY time_created, id",
    )?;
    let texts = statement
        .query_map([message_id], |row| {
            let raw: String = row.get(0)?;
            let value: Value = serde_json::from_str(&raw).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(value
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned())
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(texts.join("\n"))
}

fn part_types(connection: &Connection, message_id: &str) -> Result<Vec<String>, Error> {
    let mut statement = connection
        .prepare("SELECT data FROM part WHERE message_id = ?1 ORDER BY time_created, id")?;
    let mut types = Vec::new();
    for row in statement.query_map([message_id], |row| row.get::<_, String>(0))? {
        let raw = row?;
        let data: Value = serde_json::from_str(&raw).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
        let Some(kind) = data.get("type").and_then(Value::as_str) else {
            continue;
        };
        if kind == "step-finish" || kind == "step-start" {
            continue;
        }
        let label = if kind == "tool" {
            data.get("tool").and_then(Value::as_str).unwrap_or(kind)
        } else {
            kind
        };
        if !types.iter().any(|item| item == label) {
            types.push(label.to_owned());
        }
    }
    Ok(types)
}

fn parse_model(raw: Option<String>) -> Option<Model> {
    raw.and_then(|value| serde_json::from_str(&value).ok())
        .and_then(|value| model_from_value(&value))
}
fn model_from_value(value: &Value) -> Option<Model> {
    Some(Model {
        provider_id: value.get("providerID")?.as_str()?.to_owned(),
        model_id: value
            .get("modelID")
            .or_else(|| value.get("id"))?
            .as_str()?
            .to_owned(),
        variant: value
            .get("variant")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}
fn usage_from_value(value: &Value) -> Option<Usage> {
    let tokens = value.get("tokens")?;
    Some(Usage {
        cost: value.get("cost").and_then(Value::as_f64),
        input_tokens: number(tokens, "input")?,
        output_tokens: number(tokens, "output")?,
        reasoning_tokens: number(tokens, "reasoning")?,
        cache_read_tokens: number(tokens.get("cache")?, "read")?,
        cache_write_tokens: number(tokens.get("cache")?, "write")?,
        total_tokens: tokens.get("total").and_then(Value::as_u64),
    })
}
fn number(value: &Value, key: &str) -> Option<u64> {
    value.get(key)?.as_u64()
}
fn nonnegative(value: i64) -> u64 {
    value.max(0) as u64
}
fn issue(result: &mut Extraction, record_type: &str, id: String, reason: &str) {
    result.issues.push(ParseIssue {
        record_type: record_type.into(),
        id,
        reason: reason.into(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_missing_schema() {
        let connection = Connection::open_in_memory().unwrap();
        let error = validate_schema(&connection).unwrap_err();
        assert!(error.to_string().contains("table project"));
    }

    #[test]
    fn parses_step_usage() {
        let data: Value = serde_json::json!({"cost": 0.2, "tokens": {"total": 10, "input": 2, "output": 3, "reasoning": 1, "cache": {"read": 2, "write": 2}}});
        assert_eq!(usage_from_value(&data).unwrap().total_tokens, Some(10));
    }
}
