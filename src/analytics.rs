use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, OptionalExtension, Row, params, types::Value as SqlValue};
use serde::{Deserialize, Serialize};

use crate::{
    AssistantMessage, CompletedStep, Error, Extraction, OutputPart, Session, Usage,
    default_analytics_path,
};

const EXTRACTOR_SCHEMA_VERSION: u32 = 3;
const EXTRACTOR_SCHEMA_SIGNATURE: &str = "project(id,worktree,name);session(id,parent_id,project_id,title,model,cost,tokens_input,tokens_output,tokens_reasoning,tokens_cache_read,tokens_cache_write,time_created,time_updated);message(id,session_id,data,time_created,time_updated);part(id,message_id,session_id,data,time_created,time_updated)";

/// Application-owned SQLite store populated from read-only OpenCode extractions.
pub struct AnalyticsStore {
    connection: Connection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportSummary {
    pub sessions: usize,
    pub assistant_messages: usize,
    pub steps: usize,
    pub issues: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionUsage {
    pub source: String,
    pub session_id: String,
    pub project_id: String,
    pub title: String,
    pub created_at_ms: i64,
    pub parent_id: Option<String>,
    pub usage: Usage,
    pub models: Vec<ModelUsage>,
    pub source_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelUsage {
    pub provider_id: String,
    pub model_id: String,
    pub variant: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Turn {
    pub id: String,
    pub message_id: String,
    pub model: Option<ModelSummary>,
    pub user_message: Option<String>,
    pub types: Vec<String>,
    pub reason: Option<String>,
    pub usage: Usage,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TurnText {
    pub turn_id: String,
    pub message_id: String,
    pub parts: Option<Vec<OutputPart>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionDetail {
    pub source: String,
    pub session_id: String,
    pub project_id: String,
    pub title: String,
    pub created_at_ms: i64,
    pub parent_id: Option<String>,
    pub usage: Usage,
    pub source_kind: String,
    pub models: Vec<ModelUsage>,
    pub turns: Vec<Turn>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageFilter {
    pub project_id: Option<String>,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub start_at_ms: Option<i64>,
    pub end_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectSummary {
    pub source: String,
    pub id: String,
    pub name: Option<String>,
    pub worktree: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelSummary {
    pub provider_id: String,
    pub model_id: String,
    pub variant: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeriodUsage {
    pub source: String,
    pub start_at_ms: i64,
    pub sessions: u64,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Reconciliation {
    pub source: String,
    pub session_id: String,
    pub completed_steps: Option<Usage>,
    pub assistant_messages: Option<Usage>,
    pub session: Usage,
}

impl Reconciliation {
    pub fn has_mismatch(&self) -> bool {
        self.completed_steps
            .as_ref()
            .is_some_and(|usage| !usage_matches(usage, &self.session))
            || self
                .assistant_messages
                .as_ref()
                .is_some_and(|usage| !usage_matches(usage, &self.session))
            || matches!(
                (&self.completed_steps, &self.assistant_messages),
                (Some(steps), Some(messages)) if !usage_matches(steps, messages)
            )
    }
}

impl AnalyticsStore {
    pub fn open_default() -> Result<Self, Error> {
        Self::open(default_analytics_path()?)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let store = Self {
            connection: Connection::open(path)?,
        };
        store.initialize()?;
        Ok(store)
    }

    pub fn import(&mut self, extraction: &Extraction) -> Result<ImportSummary, Error> {
        let source = extraction.source.to_string_lossy();
        let imported_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before the Unix epoch")
            .as_millis() as i64;
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "INSERT INTO source (path, schema_version, schema_signature, imported_at_ms)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(path) DO UPDATE SET
               schema_version = excluded.schema_version,
               schema_signature = excluded.schema_signature,
               imported_at_ms = excluded.imported_at_ms",
            params![
                source,
                EXTRACTOR_SCHEMA_VERSION,
                EXTRACTOR_SCHEMA_SIGNATURE,
                imported_at_ms
            ],
        )?;
        for session in &extraction.sessions {
            upsert_session(&transaction, &source, session)?;
        }
        for message in &extraction.assistant_messages {
            upsert_message(&transaction, &source, message)?;
        }
        for message in &extraction.user_messages {
            upsert_user_message(&transaction, &source, message)?;
        }
        for step in &extraction.steps {
            upsert_step(&transaction, &source, step)?;
        }
        transaction.execute("DELETE FROM parse_issue WHERE source = ?1", params![source])?;
        for issue in &extraction.issues {
            transaction.execute(
                "INSERT INTO parse_issue (source, record_type, record_id, reason) VALUES (?1, ?2, ?3, ?4)",
                params![source, issue.record_type, issue.id, issue.reason],
            )?;
        }
        transaction.commit()?;
        Ok(ImportSummary {
            sessions: extraction.sessions.len(),
            assistant_messages: extraction.assistant_messages.len(),
            steps: extraction.steps.len(),
            issues: extraction.issues.len(),
        })
    }

    pub fn session_usage(&self) -> Result<Vec<SessionUsage>, Error> {
        self.session_usage_filtered(&UsageFilter::default())
    }

    pub fn session_usage_filtered(&self, filter: &UsageFilter) -> Result<Vec<SessionUsage>, Error> {
        let (where_clause, values) = usage_filter(filter);
        let mut statement = self.connection.prepare(&format!(
            "SELECT su.source, su.session_id, s.project_id, s.title, su.cost, su.input_tokens, su.output_tokens,
                    su.reasoning_tokens, su.cache_read_tokens, su.cache_write_tokens, su.total_tokens,
                    su.source_kind, s.created_at_ms, s.parent_id
              FROM session_usage su JOIN session s ON s.source = su.source AND s.id = su.session_id
              {where_clause} ORDER BY s.created_at_ms, su.source, s.id"
        ))?;
        let sessions = statement
            .query_map(rusqlite::params_from_iter(values), |row| {
                let source: String = row.get(0)?;
                let session_id: String = row.get(1)?;
                Ok(SessionUsage {
                    source,
                    session_id,
                    project_id: row.get(2)?,
                    title: row.get(3)?,
                    created_at_ms: row.get(12)?,
                    parent_id: row.get(13)?,
                    usage: usage_from_row(row, 4)?,
                    models: Vec::new(),
                    source_kind: row.get(11)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        sessions
            .into_iter()
            .map(|mut session| {
                session.models = self.session_models(&session.source, &session.session_id)?;
                Ok(session)
            })
            .collect()
    }

    fn session_models(&self, source: &str, session_id: &str) -> Result<Vec<ModelUsage>, Error> {
        let mut statement = self.connection.prepare(
            "SELECT provider_id, model_id, variant, SUM(cost), SUM(input_tokens),
                    SUM(output_tokens), SUM(reasoning_tokens), SUM(cache_read_tokens),
                    SUM(cache_write_tokens), SUM(total_tokens)
             FROM assistant_message
             WHERE source = ?1 AND session_id = ?2
             GROUP BY provider_id, model_id, variant
             ORDER BY provider_id, model_id, variant",
        )?;
        statement
            .query_map(params![source, session_id], |row| {
                Ok(ModelUsage {
                    provider_id: row.get(0)?,
                    model_id: row.get(1)?,
                    variant: row.get(2)?,
                    usage: usage_from_row(row, 3)?,
                })
            })?
            .collect::<Result<_, _>>()
            .map_err(Error::from)
    }

    pub fn model_usage_filtered(&self, filter: &UsageFilter) -> Result<Vec<ModelUsage>, Error> {
        let (where_clause, values) = usage_filter(filter);
        let mut statement = self.connection.prepare(&format!(
            "SELECT am.provider_id, am.model_id, am.variant, SUM(am.cost), SUM(am.input_tokens),
                    SUM(am.output_tokens), SUM(am.reasoning_tokens), SUM(am.cache_read_tokens),
                    SUM(am.cache_write_tokens), SUM(am.total_tokens)
             FROM assistant_message am JOIN session s ON s.source = am.source AND s.id = am.session_id
             {where_clause} GROUP BY am.provider_id, am.model_id, am.variant
             ORDER BY am.provider_id, am.model_id, am.variant"
        ))?;
        statement
            .query_map(rusqlite::params_from_iter(values), |row| {
                Ok(ModelUsage {
                    provider_id: row.get(0)?,
                    model_id: row.get(1)?,
                    variant: row.get(2)?,
                    usage: usage_from_row(row, 3)?,
                })
            })?
            .collect::<Result<_, _>>()
            .map_err(Error::from)
    }

    pub fn session_detail(
        &self,
        source: &str,
        session_id: &str,
    ) -> Result<Option<SessionDetail>, Error> {
        let mut statement = self.connection.prepare(
            "SELECT su.source, su.session_id, s.project_id, s.title, su.cost, su.input_tokens,
                    su.output_tokens, su.reasoning_tokens, su.cache_read_tokens, su.cache_write_tokens,
                    su.total_tokens, su.source_kind, s.created_at_ms, s.parent_id
              FROM session_usage su JOIN session s ON s.source = su.source AND s.id = su.session_id
              WHERE su.source = ?1 AND su.session_id = ?2",
        )?;
        let Some((
            source,
            session_id,
            project_id,
            title,
            usage,
            source_kind,
            created_at_ms,
            parent_id,
        )) = statement
            .query_row(params![source, session_id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    usage_from_row(row, 4)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                ))
            })
            .optional()?
        else {
            return Ok(None);
        };

        let mut models_statement = self.connection.prepare(
            "SELECT provider_id, model_id, variant, SUM(cost), SUM(input_tokens), SUM(output_tokens),
                    SUM(reasoning_tokens), SUM(cache_read_tokens), SUM(cache_write_tokens), SUM(total_tokens)
             FROM assistant_message
             WHERE source = ?1 AND session_id = ?2
             GROUP BY provider_id, model_id, variant
             ORDER BY provider_id, model_id, variant",
        )?;
        let models = models_statement
            .query_map(params![source, session_id], |row| {
                Ok(ModelUsage {
                    provider_id: row.get(0)?,
                    model_id: row.get(1)?,
                    variant: row.get(2)?,
                    usage: usage_from_row(row, 3)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        let mut turns_statement = self.connection.prepare(
            "SELECT cs.id, cs.message_id, am.provider_id, am.model_id, am.variant,
                    cs.types, cs.reason, um.text, cs.cost, cs.input_tokens, cs.output_tokens, cs.reasoning_tokens,
                    cs.cache_read_tokens, cs.cache_write_tokens, cs.total_tokens,
                    cs.created_at_ms, cs.updated_at_ms
             FROM completed_step cs
             LEFT JOIN assistant_message am
               ON am.source = cs.source AND am.id = cs.message_id
             LEFT JOIN user_message um
               ON um.source = cs.source AND um.id = am.parent_id
             WHERE cs.source = ?1 AND cs.session_id = ?2
             ORDER BY cs.created_at_ms, cs.id",
        )?;
        let turns = turns_statement
            .query_map(params![source, session_id], |row| {
                let provider_id: Option<String> = row.get(2)?;
                let model_id: Option<String> = row.get(3)?;
                let variant: Option<String> = row.get(4)?;
                let types: String = row.get(5)?;
                Ok(Turn {
                    id: row.get(0)?,
                    message_id: row.get(1)?,
                    model: provider_id
                        .zip(model_id)
                        .map(|(provider_id, model_id)| ModelSummary {
                            provider_id,
                            model_id,
                            variant,
                        }),
                    user_message: row.get(7)?,
                    types: types
                        .split('\u{1f}')
                        .filter(|item| !item.is_empty())
                        .map(str::to_owned)
                        .collect(),
                    reason: row.get(6)?,
                    usage: usage_from_row(row, 8)?,
                    created_at_ms: row.get(15)?,
                    updated_at_ms: row.get(16)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        Ok(Some(SessionDetail {
            source,
            session_id,
            project_id,
            title,
            created_at_ms,
            parent_id,
            usage,
            source_kind,
            models,
            turns,
        }))
    }

    pub fn turn_text(
        &self,
        source: &str,
        session_id: &str,
        turn_id: &str,
    ) -> Result<Option<TurnText>, Error> {
        let mut statement = self.connection.prepare(
            "SELECT cs.id, cs.message_id, am.parts, am.text
             FROM completed_step cs
             JOIN assistant_message am
               ON am.source = cs.source AND am.id = cs.message_id
             WHERE cs.source = ?1 AND cs.session_id = ?2 AND cs.id = ?3",
        )?;
        statement
            .query_row(params![source, session_id, turn_id], |row| {
                let raw_parts: Option<String> = row.get(2)?;
                let legacy_text: Option<String> = row.get(3)?;
                let parts = raw_parts
                    .map(|raw| serde_json::from_str(&raw))
                    .transpose()
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?
                    .or_else(|| {
                        legacy_text.map(|text| {
                            vec![OutputPart {
                                id: row.get(1).unwrap_or_default(),
                                part_type: "text".into(),
                                data: serde_json::json!({"type": "text", "text": text}),
                                created_at_ms: 0,
                            }]
                        })
                    });
                Ok(TurnText {
                    turn_id: row.get(0)?,
                    message_id: row.get(1)?,
                    parts,
                })
            })
            .optional()
            .map_err(Error::from)
    }

    pub fn period_usage(
        &self,
        filter: &UsageFilter,
        period_ms: i64,
    ) -> Result<Vec<PeriodUsage>, Error> {
        if period_ms <= 0 {
            return Err(Error::InvalidPeriod);
        }
        let (where_clause, mut values) = usage_filter(filter);
        values.insert(0, SqlValue::Integer(period_ms));
        let mut statement = self.connection.prepare(&format!(
            "SELECT su.source, (s.created_at_ms / ?1) * ?1 AS start_at_ms, COUNT(*) AS sessions,
                    SUM(su.cost), SUM(su.input_tokens), SUM(su.output_tokens), SUM(su.reasoning_tokens),
                    SUM(su.cache_read_tokens), SUM(su.cache_write_tokens), SUM(su.total_tokens)
             FROM session_usage su JOIN session s ON s.source = su.source AND s.id = su.session_id
             {where_clause} GROUP BY su.source, start_at_ms ORDER BY su.source, start_at_ms"
        ))?;
        statement
            .query_map(rusqlite::params_from_iter(values), |row| {
                Ok(PeriodUsage {
                    source: row.get(0)?,
                    start_at_ms: row.get(1)?,
                    sessions: row.get::<_, i64>(2)? as u64,
                    usage: usage_from_row(row, 3)?,
                })
            })?
            .collect::<Result<_, _>>()
            .map_err(Error::from)
    }

    pub fn reconcile(&self, filter: &UsageFilter) -> Result<Vec<Reconciliation>, Error> {
        let (where_clause, values) = usage_filter(filter);
        let mut statement = self.connection.prepare(&format!(
            "WITH step_totals AS (
               SELECT source, session_id, COUNT(*) AS records, SUM(cost) AS cost,
                 SUM(input_tokens) AS input_tokens, SUM(output_tokens) AS output_tokens,
                 SUM(reasoning_tokens) AS reasoning_tokens, SUM(cache_read_tokens) AS cache_read_tokens,
                 SUM(cache_write_tokens) AS cache_write_tokens, SUM(total_tokens) AS total_tokens
               FROM completed_step GROUP BY source, session_id
             ), message_totals AS (
               SELECT source, session_id, COUNT(*) AS records, SUM(cost) AS cost,
                 SUM(input_tokens) AS input_tokens, SUM(output_tokens) AS output_tokens,
                 SUM(reasoning_tokens) AS reasoning_tokens, SUM(cache_read_tokens) AS cache_read_tokens,
                 SUM(cache_write_tokens) AS cache_write_tokens, SUM(total_tokens) AS total_tokens
               FROM assistant_message GROUP BY source, session_id
             )
             SELECT s.source, s.id, st.records, st.cost, st.input_tokens, st.output_tokens,
                    st.reasoning_tokens, st.cache_read_tokens, st.cache_write_tokens, st.total_tokens,
                    mt.records, mt.cost, mt.input_tokens, mt.output_tokens, mt.reasoning_tokens,
                    mt.cache_read_tokens, mt.cache_write_tokens, mt.total_tokens,
                    s.cost, s.input_tokens, s.output_tokens, s.reasoning_tokens,
                    s.cache_read_tokens, s.cache_write_tokens
             FROM session s LEFT JOIN step_totals st ON st.source = s.source AND st.session_id = s.id
             LEFT JOIN message_totals mt ON mt.source = s.source AND mt.session_id = s.id
             {where_clause} ORDER BY s.source, s.id"
        ))?;
        statement
            .query_map(rusqlite::params_from_iter(values), |row| {
                Ok(Reconciliation {
                    source: row.get(0)?,
                    session_id: row.get(1)?,
                    completed_steps: optional_usage_from_row(row, 2, 3)?,
                    assistant_messages: optional_usage_from_row(row, 10, 11)?,
                    session: session_usage_from_row(row, 18)?,
                })
            })?
            .collect::<Result<_, _>>()
            .map_err(Error::from)
    }

    pub fn projects(&self) -> Result<Vec<ProjectSummary>, Error> {
        let mut statement = self.connection.prepare(
            "SELECT DISTINCT source, project_id, project_name, project_worktree
             FROM session ORDER BY source, project_name, project_id",
        )?;
        statement
            .query_map([], |row| {
                Ok(ProjectSummary {
                    source: row.get(0)?,
                    id: row.get(1)?,
                    name: row.get(2)?,
                    worktree: row.get(3)?,
                })
            })?
            .collect::<Result<_, _>>()
            .map_err(Error::from)
    }

    pub fn models(&self) -> Result<Vec<ModelSummary>, Error> {
        let mut statement = self.connection.prepare(
            "SELECT DISTINCT provider_id, model_id, variant FROM session
             WHERE provider_id IS NOT NULL AND model_id IS NOT NULL
             ORDER BY provider_id, model_id, variant",
        )?;
        statement
            .query_map([], |row| {
                Ok(ModelSummary {
                    provider_id: row.get(0)?,
                    model_id: row.get(1)?,
                    variant: row.get(2)?,
                })
            })?
            .collect::<Result<_, _>>()
            .map_err(Error::from)
    }

    pub fn source_schema_version(&self, source: impl AsRef<Path>) -> Result<Option<u32>, Error> {
        self.connection
            .query_row(
                "SELECT schema_version FROM source WHERE path = ?1",
                params![source.as_ref().to_string_lossy()],
                |row| row.get(0),
            )
            .optional()
            .map_err(Error::from)
    }

    fn initialize(&self) -> Result<(), Error> {
        self.connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS source (
               path TEXT PRIMARY KEY,
               schema_version INTEGER NOT NULL,
               schema_signature TEXT NOT NULL,
               imported_at_ms INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS session (
               source TEXT NOT NULL REFERENCES source(path), id TEXT NOT NULL,
               project_id TEXT NOT NULL, project_name TEXT, project_worktree TEXT NOT NULL,
                title TEXT NOT NULL, provider_id TEXT, model_id TEXT, variant TEXT,
                parent_id TEXT,
                cost REAL, input_tokens INTEGER NOT NULL, output_tokens INTEGER NOT NULL,
               reasoning_tokens INTEGER NOT NULL, cache_read_tokens INTEGER NOT NULL,
               cache_write_tokens INTEGER NOT NULL, created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL,
               PRIMARY KEY (source, id)
             );
             CREATE TABLE IF NOT EXISTS assistant_message (
               source TEXT NOT NULL REFERENCES source(path), id TEXT NOT NULL, session_id TEXT NOT NULL,
               parent_id TEXT, provider_id TEXT NOT NULL, model_id TEXT NOT NULL, variant TEXT,
               cost REAL, input_tokens INTEGER NOT NULL, output_tokens INTEGER NOT NULL,
               reasoning_tokens INTEGER NOT NULL, cache_read_tokens INTEGER NOT NULL, cache_write_tokens INTEGER NOT NULL,
                total_tokens INTEGER, created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL,
                text TEXT, parts TEXT,
                PRIMARY KEY (source, id)
             );
             CREATE TABLE IF NOT EXISTS completed_step (
               source TEXT NOT NULL REFERENCES source(path), id TEXT NOT NULL, message_id TEXT NOT NULL, session_id TEXT NOT NULL,
               types TEXT NOT NULL DEFAULT '', reason TEXT,
               cost REAL, input_tokens INTEGER NOT NULL, output_tokens INTEGER NOT NULL,
               reasoning_tokens INTEGER NOT NULL, cache_read_tokens INTEGER NOT NULL, cache_write_tokens INTEGER NOT NULL,
                total_tokens INTEGER, created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL,
                PRIMARY KEY (source, id)
             );
             CREATE TABLE IF NOT EXISTS parse_issue (
               source TEXT NOT NULL REFERENCES source(path), record_type TEXT NOT NULL, record_id TEXT NOT NULL, reason TEXT NOT NULL,
               PRIMARY KEY (source, record_type, record_id, reason)
             );
             CREATE INDEX IF NOT EXISTS assistant_message_session ON assistant_message (source, session_id);
             CREATE INDEX IF NOT EXISTS completed_step_session ON completed_step (source, session_id);
             CREATE TABLE IF NOT EXISTS user_message (
               source TEXT NOT NULL REFERENCES source(path), id TEXT NOT NULL, session_id TEXT NOT NULL,
               text TEXT NOT NULL, created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL,
               PRIMARY KEY (source, id)
             );
             CREATE VIEW IF NOT EXISTS session_usage AS
             WITH step_totals AS (
               SELECT source, session_id, COUNT(*) AS records, SUM(cost) AS cost,
                 SUM(input_tokens) AS input_tokens, SUM(output_tokens) AS output_tokens,
                 SUM(reasoning_tokens) AS reasoning_tokens, SUM(cache_read_tokens) AS cache_read_tokens,
                 SUM(cache_write_tokens) AS cache_write_tokens, SUM(total_tokens) AS total_tokens
               FROM completed_step GROUP BY source, session_id
             ), message_totals AS (
               SELECT source, session_id, SUM(cost) AS cost, SUM(input_tokens) AS input_tokens,
                 SUM(output_tokens) AS output_tokens, SUM(reasoning_tokens) AS reasoning_tokens,
                 SUM(cache_read_tokens) AS cache_read_tokens, SUM(cache_write_tokens) AS cache_write_tokens,
                 SUM(total_tokens) AS total_tokens
               FROM assistant_message GROUP BY source, session_id
             )
             SELECT s.source, s.id AS session_id,
               CASE WHEN st.records > 0 THEN st.cost ELSE mt.cost END AS cost,
               COALESCE(CASE WHEN st.records > 0 THEN st.input_tokens ELSE mt.input_tokens END, 0) AS input_tokens,
               COALESCE(CASE WHEN st.records > 0 THEN st.output_tokens ELSE mt.output_tokens END, 0) AS output_tokens,
               COALESCE(CASE WHEN st.records > 0 THEN st.reasoning_tokens ELSE mt.reasoning_tokens END, 0) AS reasoning_tokens,
               COALESCE(CASE WHEN st.records > 0 THEN st.cache_read_tokens ELSE mt.cache_read_tokens END, 0) AS cache_read_tokens,
               COALESCE(CASE WHEN st.records > 0 THEN st.cache_write_tokens ELSE mt.cache_write_tokens END, 0) AS cache_write_tokens,
               CASE WHEN st.records > 0 THEN st.total_tokens ELSE mt.total_tokens END AS total_tokens,
               CASE WHEN st.records > 0 THEN 'steps' ELSE 'messages' END AS source_kind
             FROM session s LEFT JOIN step_totals st ON st.source = s.source AND st.session_id = s.id
             LEFT JOIN message_totals mt ON mt.source = s.source AND mt.session_id = s.id;",
        )?;
        ensure_column(&self.connection, "session", "parent_id", "TEXT")?;
        ensure_column(&self.connection, "assistant_message", "parent_id", "TEXT")?;
        ensure_column(&self.connection, "assistant_message", "text", "TEXT")?;
        ensure_column(&self.connection, "assistant_message", "parts", "TEXT")?;
        ensure_column(
            &self.connection,
            "completed_step",
            "types",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        ensure_column(&self.connection, "completed_step", "reason", "TEXT")?;
        Ok(())
    }
}

fn usage_filter(filter: &UsageFilter) -> (String, Vec<SqlValue>) {
    let mut conditions = Vec::new();
    let mut values = Vec::new();
    if let Some(project_id) = &filter.project_id {
        conditions.push("s.project_id = ?".to_owned());
        values.push(SqlValue::Text(project_id.clone()));
    }
    if let Some(provider_id) = &filter.provider_id {
        conditions.push("s.provider_id = ?".to_owned());
        values.push(SqlValue::Text(provider_id.clone()));
    }
    if let Some(model_id) = &filter.model_id {
        conditions.push("s.model_id = ?".to_owned());
        values.push(SqlValue::Text(model_id.clone()));
    }
    if let Some(start_at_ms) = filter.start_at_ms {
        conditions.push("s.created_at_ms >= ?".to_owned());
        values.push(SqlValue::Integer(start_at_ms));
    }
    if let Some(end_at_ms) = filter.end_at_ms {
        conditions.push("s.created_at_ms < ?".to_owned());
        values.push(SqlValue::Integer(end_at_ms));
    }
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };
    (where_clause, values)
}

fn usage_from_row(row: &Row<'_>, offset: usize) -> rusqlite::Result<Usage> {
    Ok(Usage {
        cost: row.get(offset)?,
        input_tokens: row.get::<_, i64>(offset + 1)? as u64,
        output_tokens: row.get::<_, i64>(offset + 2)? as u64,
        reasoning_tokens: row.get::<_, i64>(offset + 3)? as u64,
        cache_read_tokens: row.get::<_, i64>(offset + 4)? as u64,
        cache_write_tokens: row.get::<_, i64>(offset + 5)? as u64,
        total_tokens: row
            .get::<_, Option<i64>>(offset + 6)?
            .map(|value| value as u64),
    })
}

fn optional_usage_from_row(
    row: &Row<'_>,
    count_offset: usize,
    usage_offset: usize,
) -> rusqlite::Result<Option<Usage>> {
    match row.get::<_, Option<i64>>(count_offset)? {
        Some(_) => usage_from_row(row, usage_offset).map(Some),
        None => Ok(None),
    }
}

fn session_usage_from_row(row: &Row<'_>, offset: usize) -> rusqlite::Result<Usage> {
    Ok(Usage {
        cost: row.get(offset)?,
        input_tokens: row.get::<_, i64>(offset + 1)? as u64,
        output_tokens: row.get::<_, i64>(offset + 2)? as u64,
        reasoning_tokens: row.get::<_, i64>(offset + 3)? as u64,
        cache_read_tokens: row.get::<_, i64>(offset + 4)? as u64,
        cache_write_tokens: row.get::<_, i64>(offset + 5)? as u64,
        total_tokens: None,
    })
}

fn usage_matches(left: &Usage, right: &Usage) -> bool {
    (left.cost.is_none() || right.cost.is_none() || left.cost == right.cost)
        && left.input_tokens == right.input_tokens
        && left.output_tokens == right.output_tokens
        && left.reasoning_tokens == right.reasoning_tokens
        && left.cache_read_tokens == right.cache_read_tokens
        && left.cache_write_tokens == right.cache_write_tokens
        && (left.total_tokens.is_none()
            || right.total_tokens.is_none()
            || left.total_tokens == right.total_tokens)
}

fn upsert_session(connection: &Connection, source: &str, session: &Session) -> Result<(), Error> {
    let input_tokens = sqlite_token_count(session.usage.input_tokens)?;
    let output_tokens = sqlite_token_count(session.usage.output_tokens)?;
    let reasoning_tokens = sqlite_token_count(session.usage.reasoning_tokens)?;
    let cache_read_tokens = sqlite_token_count(session.usage.cache_read_tokens)?;
    let cache_write_tokens = sqlite_token_count(session.usage.cache_write_tokens)?;
    connection.execute(
        "INSERT INTO session
           (source, id, parent_id, project_id, project_name, project_worktree, title,
            provider_id, model_id, variant, cost, input_tokens, output_tokens, reasoning_tokens,
            cache_read_tokens, cache_write_tokens, created_at_ms, updated_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
         ON CONFLICT(source, id) DO UPDATE SET project_id=excluded.project_id, project_name=excluded.project_name,
         parent_id=excluded.parent_id, project_worktree=excluded.project_worktree, title=excluded.title,
         provider_id=excluded.provider_id,
         model_id=excluded.model_id, variant=excluded.variant, cost=excluded.cost, input_tokens=excluded.input_tokens,
         output_tokens=excluded.output_tokens, reasoning_tokens=excluded.reasoning_tokens,
         cache_read_tokens=excluded.cache_read_tokens, cache_write_tokens=excluded.cache_write_tokens,
         created_at_ms=excluded.created_at_ms, updated_at_ms=excluded.updated_at_ms",
         params![source, session.id, session.parent_id, session.project.id, session.project.name, session.project.worktree,
              session.title, session.model.as_ref().map(|model| &model.provider_id), session.model.as_ref().map(|model| &model.model_id),
              session.model.as_ref().and_then(|model| model.variant.as_ref()), session.usage.cost, input_tokens,
             output_tokens, reasoning_tokens, cache_read_tokens, cache_write_tokens, session.created_at_ms, session.updated_at_ms],
    )?;
    Ok(())
}

fn upsert_message(
    connection: &Connection,
    source: &str,
    message: &AssistantMessage,
) -> Result<(), Error> {
    let parts = serde_json::to_string(&message.parts).map_err(|error| {
        Error::Configuration(format!("could not serialize assistant parts: {error}"))
    })?;
    upsert_usage_record(
        connection,
        "assistant_message",
        source,
        UsageRecord {
            id: &message.id,
            session_id: &message.session_id,
            message_id: None,
            usage: &message.usage,
            created_at_ms: message.created_at_ms,
            updated_at_ms: message.updated_at_ms,
            model: Some((
                &message.model.provider_id,
                &message.model.model_id,
                message.model.variant.as_deref(),
            )),
            parent_id: message.parent_id.as_deref(),
            text: Some(&message.text),
            parts: Some(&parts),
            types: None,
            reason: None,
        },
    )
}

fn upsert_user_message(
    connection: &Connection,
    source: &str,
    message: &crate::UserMessage,
) -> Result<(), Error> {
    connection.execute(
        "INSERT INTO user_message (source, id, session_id, text, created_at_ms, updated_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(source, id) DO UPDATE SET session_id=excluded.session_id, text=excluded.text,
         created_at_ms=excluded.created_at_ms, updated_at_ms=excluded.updated_at_ms",
        params![
            source,
            message.id,
            message.session_id,
            message.text,
            message.created_at_ms,
            message.updated_at_ms
        ],
    )?;
    Ok(())
}

fn upsert_step(connection: &Connection, source: &str, step: &CompletedStep) -> Result<(), Error> {
    upsert_usage_record(
        connection,
        "completed_step",
        source,
        UsageRecord {
            id: &step.id,
            session_id: &step.session_id,
            message_id: Some(&step.message_id),
            usage: &step.usage,
            created_at_ms: step.created_at_ms,
            updated_at_ms: step.updated_at_ms,
            model: None,
            parent_id: None,
            text: None,
            parts: None,
            types: Some(&step.types),
            reason: step.reason.as_deref(),
        },
    )
}

struct UsageRecord<'a> {
    id: &'a str,
    session_id: &'a str,
    message_id: Option<&'a str>,
    usage: &'a Usage,
    created_at_ms: i64,
    updated_at_ms: i64,
    model: Option<(&'a str, &'a str, Option<&'a str>)>,
    parent_id: Option<&'a str>,
    text: Option<&'a str>,
    parts: Option<&'a str>,
    types: Option<&'a Vec<String>>,
    reason: Option<&'a str>,
}

fn upsert_usage_record(
    connection: &Connection,
    table: &str,
    source: &str,
    record: UsageRecord<'_>,
) -> Result<(), Error> {
    let (provider_id, model_id, variant) = record.model.unwrap_or(("", "", None));
    let input_tokens = sqlite_token_count(record.usage.input_tokens)?;
    let output_tokens = sqlite_token_count(record.usage.output_tokens)?;
    let reasoning_tokens = sqlite_token_count(record.usage.reasoning_tokens)?;
    let cache_read_tokens = sqlite_token_count(record.usage.cache_read_tokens)?;
    let cache_write_tokens = sqlite_token_count(record.usage.cache_write_tokens)?;
    let total_tokens = record
        .usage
        .total_tokens
        .map(sqlite_token_count)
        .transpose()?;
    let types = record
        .types
        .map(|values| values.join("\u{1f}"))
        .unwrap_or_default();
    let sql = if table == "assistant_message" {
        "INSERT INTO assistant_message (source, id, session_id, parent_id, provider_id, model_id, variant, cost, input_tokens, output_tokens, reasoning_tokens, cache_read_tokens, cache_write_tokens, total_tokens, created_at_ms, updated_at_ms, text, parts)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
         ON CONFLICT(source, id) DO UPDATE SET session_id=excluded.session_id, parent_id=excluded.parent_id, provider_id=excluded.provider_id, model_id=excluded.model_id, variant=excluded.variant, cost=excluded.cost, input_tokens=excluded.input_tokens, output_tokens=excluded.output_tokens, reasoning_tokens=excluded.reasoning_tokens, cache_read_tokens=excluded.cache_read_tokens, cache_write_tokens=excluded.cache_write_tokens, total_tokens=excluded.total_tokens, created_at_ms=excluded.created_at_ms, updated_at_ms=excluded.updated_at_ms, text=excluded.text, parts=excluded.parts"
    } else {
        "INSERT INTO completed_step (source, id, message_id, session_id, types, reason, cost, input_tokens, output_tokens, reasoning_tokens, cache_read_tokens, cache_write_tokens, total_tokens, created_at_ms, updated_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
         ON CONFLICT(source, id) DO UPDATE SET message_id=excluded.message_id, session_id=excluded.session_id, types=excluded.types, reason=excluded.reason, cost=excluded.cost, input_tokens=excluded.input_tokens, output_tokens=excluded.output_tokens, reasoning_tokens=excluded.reasoning_tokens, cache_read_tokens=excluded.cache_read_tokens, cache_write_tokens=excluded.cache_write_tokens, total_tokens=excluded.total_tokens, created_at_ms=excluded.created_at_ms, updated_at_ms=excluded.updated_at_ms"
    };
    if table == "assistant_message" {
        connection.execute(
            sql,
            params![
                source,
                record.id,
                record.session_id,
                record.parent_id,
                provider_id,
                model_id,
                variant,
                record.usage.cost,
                input_tokens,
                output_tokens,
                reasoning_tokens,
                cache_read_tokens,
                cache_write_tokens,
                total_tokens,
                record.created_at_ms,
                record.updated_at_ms,
                record.text,
                record.parts
            ],
        )?;
    } else {
        connection.execute(
            sql,
            params![
                source,
                record.id,
                record.message_id,
                record.session_id,
                types,
                record.reason,
                record.usage.cost,
                input_tokens,
                output_tokens,
                reasoning_tokens,
                cache_read_tokens,
                cache_write_tokens,
                total_tokens,
                record.created_at_ms,
                record.updated_at_ms
            ],
        )?;
    }
    Ok(())
}

fn sqlite_token_count(value: u64) -> Result<i64, Error> {
    i64::try_from(value).map_err(|_| Error::TokenCount(value))
}

fn ensure_column(
    connection: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), Error> {
    let exists = connection
        .prepare(&format!("PRAGMA table_info({table})"))?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .any(|name| name == column);
    if !exists {
        connection.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::tempdir;

    use super::*;
    use crate::{Model, ParseIssue, Project};

    fn usage(cost: f64, input_tokens: u64) -> Usage {
        Usage {
            cost: Some(cost),
            input_tokens,
            output_tokens: 3,
            reasoning_tokens: 1,
            cache_read_tokens: 2,
            cache_write_tokens: 1,
            total_tokens: Some(input_tokens + 7),
        }
    }

    fn session(id: &str) -> Session {
        Session {
            id: id.into(),
            parent_id: None,
            project: Project {
                id: "project-1".into(),
                name: Some("ocstats".into()),
                worktree: "/work/ocstats".into(),
            },
            title: id.into(),
            model: Some(Model {
                provider_id: "openai".into(),
                model_id: "gpt-5".into(),
                variant: None,
            }),
            usage: usage(99.0, 99),
            created_at_ms: 1,
            updated_at_ms: 2,
        }
    }

    #[test]
    fn imports_idempotently_and_prefers_steps_over_messages() {
        let directory = tempdir().unwrap();
        let source = PathBuf::from("/data/opencode.db");
        let mut extraction = Extraction {
            source: source.clone(),
            sessions: vec![session("session-steps"), session("session-message")],
            assistant_messages: vec![
                AssistantMessage {
                    id: "message-steps".into(),
                    session_id: "session-steps".into(),
                    parent_id: Some("user-steps".into()),
                    model: Model {
                        provider_id: "openai".into(),
                        model_id: "gpt-5".into(),
                        variant: None,
                    },
                    usage: usage(1.0, 10),
                    text: "first response".into(),
                    parts: vec![],
                    created_at_ms: 1,
                    updated_at_ms: 2,
                },
                AssistantMessage {
                    id: "message-only".into(),
                    session_id: "session-message".into(),
                    parent_id: None,
                    model: Model {
                        provider_id: "openai".into(),
                        model_id: "gpt-5".into(),
                        variant: None,
                    },
                    usage: usage(2.0, 20),
                    text: "second response".into(),
                    parts: vec![],
                    created_at_ms: 1,
                    updated_at_ms: 2,
                },
            ],
            user_messages: vec![],
            steps: vec![CompletedStep {
                id: "step-1".into(),
                message_id: "message-steps".into(),
                session_id: "session-steps".into(),
                types: vec!["reasoning".into(), "bash".into()],
                reason: Some("tool-calls".into()),
                usage: usage(3.0, 30),
                created_at_ms: 1,
                updated_at_ms: 2,
            }],
            issues: vec![ParseIssue {
                record_type: "part".into(),
                id: "bad-part".into(),
                reason: "invalid JSON".into(),
            }],
        };
        let mut store = AnalyticsStore::open(directory.path().join("analytics.db")).unwrap();

        store.import(&extraction).unwrap();
        extraction.steps[0].usage = usage(4.0, 40);
        store.import(&extraction).unwrap();

        assert_eq!(store.source_schema_version(&source).unwrap(), Some(3));
        let detail = store
            .session_detail(&source.to_string_lossy(), "session-steps")
            .unwrap()
            .unwrap();
        assert_eq!(detail.turns.len(), 1);
        assert_eq!(detail.created_at_ms, 1);
        assert_eq!(detail.turns[0].types, vec!["reasoning", "bash"]);
        assert_eq!(detail.turns[0].reason.as_deref(), Some("tool-calls"));
        assert_eq!(
            store.session_usage().unwrap(),
            vec![
                SessionUsage {
                    source: source.to_string_lossy().into_owned(),
                    session_id: "session-message".into(),
                    project_id: "project-1".into(),
                    title: "session-message".into(),
                    created_at_ms: 1,
                    parent_id: None,
                    usage: usage(2.0, 20),
                    models: vec![ModelUsage {
                        provider_id: "openai".into(),
                        model_id: "gpt-5".into(),
                        variant: None,
                        usage: usage(2.0, 20),
                    }],
                    source_kind: "messages".into(),
                },
                SessionUsage {
                    source: source.to_string_lossy().into_owned(),
                    session_id: "session-steps".into(),
                    project_id: "project-1".into(),
                    title: "session-steps".into(),
                    created_at_ms: 1,
                    parent_id: None,
                    usage: usage(4.0, 40),
                    models: vec![ModelUsage {
                        provider_id: "openai".into(),
                        model_id: "gpt-5".into(),
                        variant: None,
                        usage: usage(1.0, 10),
                    }],
                    source_kind: "steps".into(),
                },
            ]
        );

        let filter = UsageFilter {
            project_id: Some("project-1".into()),
            provider_id: Some("openai".into()),
            model_id: Some("gpt-5".into()),
            start_at_ms: Some(0),
            end_at_ms: Some(10),
        };
        assert_eq!(store.session_usage_filtered(&filter).unwrap().len(), 2);
        assert_eq!(
            store.period_usage(&filter, 10).unwrap(),
            vec![PeriodUsage {
                source: source.to_string_lossy().into_owned(),
                start_at_ms: 0,
                sessions: 2,
                usage: Usage {
                    cost: Some(6.0),
                    input_tokens: 60,
                    output_tokens: 6,
                    reasoning_tokens: 2,
                    cache_read_tokens: 4,
                    cache_write_tokens: 2,
                    total_tokens: Some(74),
                },
            }]
        );
        assert!(matches!(
            store.period_usage(&filter, 0),
            Err(Error::InvalidPeriod)
        ));

        let reconciliations = store.reconcile(&filter).unwrap();
        assert_eq!(reconciliations.len(), 2);
        assert!(reconciliations.iter().all(Reconciliation::has_mismatch));
        assert_eq!(
            reconciliations[1]
                .completed_steps
                .as_ref()
                .unwrap()
                .input_tokens,
            40
        );
    }

    #[test]
    fn session_usage_is_sorted_by_creation_time() {
        let directory = tempdir().unwrap();
        let source = PathBuf::from("/data/opencode.db");
        let mut older = session("session-older");
        older.created_at_ms = 10;
        let mut newer = session("session-newer");
        newer.created_at_ms = 20;
        let extraction = Extraction {
            source: source.clone(),
            sessions: vec![newer, older],
            ..Extraction::default()
        };
        let mut store = AnalyticsStore::open(directory.path().join("analytics.db")).unwrap();

        store.import(&extraction).unwrap();

        let sessions = store.session_usage().unwrap();
        assert_eq!(
            sessions
                .iter()
                .map(|session| session.session_id.as_str())
                .collect::<Vec<_>>(),
            vec!["session-older", "session-newer"]
        );
        assert_eq!(sessions[0].created_at_ms, 10);
        assert_eq!(sessions[1].created_at_ms, 20);
    }

    #[test]
    fn persists_and_updates_session_parent_id() {
        let directory = tempdir().unwrap();
        let source = PathBuf::from("/data/opencode.db");
        let mut parent = session("session-parent");
        parent.created_at_ms = 10;
        let mut child = session("session-child");
        child.created_at_ms = 20;
        child.parent_id = Some(parent.id.clone());
        let mut extraction = Extraction {
            source: source.clone(),
            sessions: vec![parent, child],
            ..Extraction::default()
        };
        let mut store = AnalyticsStore::open(directory.path().join("analytics.db")).unwrap();

        store.import(&extraction).unwrap();

        let usage = store.session_usage().unwrap();
        assert_eq!(usage[1].parent_id.as_deref(), Some("session-parent"));
        assert_eq!(
            store
                .session_detail(&source.to_string_lossy(), "session-child")
                .unwrap()
                .unwrap()
                .parent_id
                .as_deref(),
            Some("session-parent")
        );

        extraction.sessions[1].parent_id = None;
        store.import(&extraction).unwrap();

        assert_eq!(store.session_usage().unwrap()[1].parent_id, None);
        assert_eq!(
            store
                .session_detail(&source.to_string_lossy(), "session-child")
                .unwrap()
                .unwrap()
                .parent_id,
            None
        );
    }

    #[test]
    fn migrates_existing_analytics_session_table() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("analytics.db");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE source (
                   path TEXT PRIMARY KEY, schema_version INTEGER NOT NULL,
                   schema_signature TEXT NOT NULL, imported_at_ms INTEGER NOT NULL
                 );
                 CREATE TABLE session (
                   source TEXT NOT NULL, id TEXT NOT NULL, project_id TEXT NOT NULL,
                   project_name TEXT, project_worktree TEXT NOT NULL, title TEXT NOT NULL,
                   provider_id TEXT, model_id TEXT, variant TEXT, cost REAL,
                   input_tokens INTEGER NOT NULL, output_tokens INTEGER NOT NULL,
                   reasoning_tokens INTEGER NOT NULL, cache_read_tokens INTEGER NOT NULL,
                   cache_write_tokens INTEGER NOT NULL, created_at_ms INTEGER NOT NULL,
                   updated_at_ms INTEGER NOT NULL, PRIMARY KEY (source, id)
                 );
                 INSERT INTO source VALUES ('/data/opencode.db', 2, 'legacy', 1);
                 INSERT INTO session VALUES ('/data/opencode.db', 'session-1', 'project-1',
                   'project', '/work', 'legacy', NULL, NULL, NULL, NULL, 0, 0, 0, 0, 0, 1, 2);",
            )
            .unwrap();
        drop(connection);

        let store = AnalyticsStore::open(&path).unwrap();
        let columns = store
            .connection
            .prepare("PRAGMA table_info(session)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(columns.iter().any(|column| column == "parent_id"));
        assert_eq!(store.session_usage().unwrap().len(), 1);
    }

    #[test]
    fn session_usage_is_sorted_by_creation_time_across_sources() {
        let directory = tempdir().unwrap();
        let mut older = session("session-older");
        older.created_at_ms = 10;
        let mut newer = session("session-newer");
        newer.created_at_ms = 20;
        let mut store = AnalyticsStore::open(directory.path().join("analytics.db")).unwrap();

        store
            .import(&Extraction {
                source: PathBuf::from("/data/a.db"),
                sessions: vec![newer],
                ..Extraction::default()
            })
            .unwrap();
        store
            .import(&Extraction {
                source: PathBuf::from("/data/z.db"),
                sessions: vec![older],
                ..Extraction::default()
            })
            .unwrap();

        let sessions = store.session_usage().unwrap();
        assert_eq!(sessions[0].session_id, "session-older");
        assert_eq!(sessions[0].source, "/data/z.db");
        assert_eq!(sessions[1].session_id, "session-newer");
        assert_eq!(sessions[1].source, "/data/a.db");
    }
}
