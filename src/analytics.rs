use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    AssistantMessage, CompletedStep, Error, Extraction, Session, Usage, default_analytics_path,
};

const EXTRACTOR_SCHEMA_VERSION: u32 = 1;
const EXTRACTOR_SCHEMA_SIGNATURE: &str = "project(id,worktree,name);session(id,project_id,title,model,cost,tokens_input,tokens_output,tokens_reasoning,tokens_cache_read,tokens_cache_write,time_created,time_updated);message(id,session_id,data,time_created,time_updated);part(id,message_id,session_id,data,time_created,time_updated)";

/// Application-owned SQLite store populated from read-only OpenCode extractions.
pub struct AnalyticsStore {
    connection: Connection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSummary {
    pub sessions: usize,
    pub assistant_messages: usize,
    pub steps: usize,
    pub issues: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionUsage {
    pub source: String,
    pub session_id: String,
    pub usage: Usage,
    pub source_kind: String,
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
        let mut statement = self.connection.prepare(
            "SELECT source, session_id, cost, input_tokens, output_tokens, reasoning_tokens,
                    cache_read_tokens, cache_write_tokens, total_tokens, source_kind
             FROM session_usage ORDER BY source, session_id",
        )?;
        statement
            .query_map([], |row| {
                Ok(SessionUsage {
                    source: row.get(0)?,
                    session_id: row.get(1)?,
                    usage: Usage {
                        cost: row.get(2)?,
                        input_tokens: row.get::<_, i64>(3)? as u64,
                        output_tokens: row.get::<_, i64>(4)? as u64,
                        reasoning_tokens: row.get::<_, i64>(5)? as u64,
                        cache_read_tokens: row.get::<_, i64>(6)? as u64,
                        cache_write_tokens: row.get::<_, i64>(7)? as u64,
                        total_tokens: row.get::<_, Option<i64>>(8)?.map(|value| value as u64),
                    },
                    source_kind: row.get(9)?,
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
               cost REAL, input_tokens INTEGER NOT NULL, output_tokens INTEGER NOT NULL,
               reasoning_tokens INTEGER NOT NULL, cache_read_tokens INTEGER NOT NULL,
               cache_write_tokens INTEGER NOT NULL, created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL,
               PRIMARY KEY (source, id)
             );
             CREATE TABLE IF NOT EXISTS assistant_message (
               source TEXT NOT NULL REFERENCES source(path), id TEXT NOT NULL, session_id TEXT NOT NULL,
               provider_id TEXT NOT NULL, model_id TEXT NOT NULL, variant TEXT,
               cost REAL, input_tokens INTEGER NOT NULL, output_tokens INTEGER NOT NULL,
               reasoning_tokens INTEGER NOT NULL, cache_read_tokens INTEGER NOT NULL, cache_write_tokens INTEGER NOT NULL,
               total_tokens INTEGER, created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL,
               PRIMARY KEY (source, id)
             );
             CREATE TABLE IF NOT EXISTS completed_step (
               source TEXT NOT NULL REFERENCES source(path), id TEXT NOT NULL, message_id TEXT NOT NULL, session_id TEXT NOT NULL,
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
        Ok(())
    }
}

fn upsert_session(connection: &Connection, source: &str, session: &Session) -> Result<(), Error> {
    let input_tokens = sqlite_token_count(session.usage.input_tokens)?;
    let output_tokens = sqlite_token_count(session.usage.output_tokens)?;
    let reasoning_tokens = sqlite_token_count(session.usage.reasoning_tokens)?;
    let cache_read_tokens = sqlite_token_count(session.usage.cache_read_tokens)?;
    let cache_write_tokens = sqlite_token_count(session.usage.cache_write_tokens)?;
    connection.execute(
        "INSERT INTO session VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
         ON CONFLICT(source, id) DO UPDATE SET project_id=excluded.project_id, project_name=excluded.project_name,
         project_worktree=excluded.project_worktree, title=excluded.title, provider_id=excluded.provider_id,
         model_id=excluded.model_id, variant=excluded.variant, cost=excluded.cost, input_tokens=excluded.input_tokens,
         output_tokens=excluded.output_tokens, reasoning_tokens=excluded.reasoning_tokens,
         cache_read_tokens=excluded.cache_read_tokens, cache_write_tokens=excluded.cache_write_tokens,
         created_at_ms=excluded.created_at_ms, updated_at_ms=excluded.updated_at_ms",
        params![source, session.id, session.project.id, session.project.name, session.project.worktree,
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
        },
    )
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
    let sql = if table == "assistant_message" {
        "INSERT INTO assistant_message VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
         ON CONFLICT(source, id) DO UPDATE SET session_id=excluded.session_id, provider_id=excluded.provider_id, model_id=excluded.model_id, variant=excluded.variant, cost=excluded.cost, input_tokens=excluded.input_tokens, output_tokens=excluded.output_tokens, reasoning_tokens=excluded.reasoning_tokens, cache_read_tokens=excluded.cache_read_tokens, cache_write_tokens=excluded.cache_write_tokens, total_tokens=excluded.total_tokens, created_at_ms=excluded.created_at_ms, updated_at_ms=excluded.updated_at_ms"
    } else {
        "INSERT INTO completed_step VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT(source, id) DO UPDATE SET message_id=excluded.message_id, session_id=excluded.session_id, cost=excluded.cost, input_tokens=excluded.input_tokens, output_tokens=excluded.output_tokens, reasoning_tokens=excluded.reasoning_tokens, cache_read_tokens=excluded.cache_read_tokens, cache_write_tokens=excluded.cache_write_tokens, total_tokens=excluded.total_tokens, created_at_ms=excluded.created_at_ms, updated_at_ms=excluded.updated_at_ms"
    };
    if table == "assistant_message" {
        connection.execute(
            sql,
            params![
                source,
                record.id,
                record.session_id,
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
                record.updated_at_ms
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
            project: Project {
                id: "project-1".into(),
                name: Some("ocstats".into()),
                worktree: "/work/ocstats".into(),
            },
            title: id.into(),
            model: None,
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
                    model: Model {
                        provider_id: "openai".into(),
                        model_id: "gpt-5".into(),
                        variant: None,
                    },
                    usage: usage(1.0, 10),
                    created_at_ms: 1,
                    updated_at_ms: 2,
                },
                AssistantMessage {
                    id: "message-only".into(),
                    session_id: "session-message".into(),
                    model: Model {
                        provider_id: "openai".into(),
                        model_id: "gpt-5".into(),
                        variant: None,
                    },
                    usage: usage(2.0, 20),
                    created_at_ms: 1,
                    updated_at_ms: 2,
                },
            ],
            steps: vec![CompletedStep {
                id: "step-1".into(),
                message_id: "message-steps".into(),
                session_id: "session-steps".into(),
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

        assert_eq!(store.source_schema_version(&source).unwrap(), Some(1));
        assert_eq!(
            store.session_usage().unwrap(),
            vec![
                SessionUsage {
                    source: source.to_string_lossy().into_owned(),
                    session_id: "session-message".into(),
                    usage: usage(2.0, 20),
                    source_kind: "messages".into(),
                },
                SessionUsage {
                    source: source.to_string_lossy().into_owned(),
                    session_id: "session-steps".into(),
                    usage: usage(4.0, 40),
                    source_kind: "steps".into(),
                },
            ]
        );
    }
}
