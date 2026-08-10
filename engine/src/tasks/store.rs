use crate::download::checkpoint::Checkpoint;
use crate::error::EngineError;
use crate::tasks::schema::{DB_PRAGMAS, TASK_MIGRATION_V2, TASK_SCHEMA, TASK_SCHEMA_VERSION};
use crate::types::{DownloadTask, TaskStatus};
use rusqlite::{params, Connection};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const V1_COLUMNS: &[&str] = &[
    "id",
    "parent_id",
    "season",
    "title",
    "source_url",
    "quality_label",
    "status",
    "progress_bytes",
    "total_bytes",
    "error_message",
    "output_path",
    "library_item_id",
    "episode_index",
    "created_at_ms",
    "updated_at_ms",
];

pub struct TaskStore {
    conn: Connection,
}

impl TaskStore {
    pub fn open(db_path: &Path) -> Result<Self, EngineError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        conn.execute_batch(DB_PRAGMAS)?;
        conn.execute_batch(TASK_SCHEMA)?;
        Self::migrate(&conn)?;
        Ok(Self { conn })
    }

    fn migrate(conn: &Connection) -> Result<(), EngineError> {
        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > TASK_SCHEMA_VERSION {
            return Err(EngineError::Message(format!(
                "unsupported tasks.db schema version {version}, expected <= {TASK_SCHEMA_VERSION}"
            )));
        }
        if version < TASK_SCHEMA_VERSION {
            if version == 0 {
                Self::ensure_v1_columns(conn)?;
            }
            let tx = conn.unchecked_transaction()?;
            if let Err(e) = tx.execute_batch(TASK_MIGRATION_V2) {
                let msg = e.to_string();
                if !msg.contains("duplicate column name") {
                    return Err(EngineError::Db(e));
                }
            }
            tx.execute(&format!("PRAGMA user_version = {TASK_SCHEMA_VERSION}"), [])?;
            tx.commit()?;
        }
        Ok(())
    }

    fn ensure_v1_columns(conn: &Connection) -> Result<(), EngineError> {
        let mut stmt = conn.prepare("PRAGMA table_info(download_tasks)")?;
        let mut columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?;
        columns.sort();
        let mut expected: Vec<&str> = V1_COLUMNS.to_vec();
        expected.sort();
        if columns.len() != expected.len()
            || columns.iter().zip(expected.iter()).any(|(a, b)| a != *b)
        {
            return Err(EngineError::Message(
                "download_tasks column set does not match schema version 1".into(),
            ));
        }
        Ok(())
    }

    fn status_to_str(s: TaskStatus) -> &'static str {
        match s {
            TaskStatus::Queued => "queued",
            TaskStatus::Running => "running",
            TaskStatus::Paused => "paused",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
        }
    }

    fn status_from_str(s: &str) -> Result<TaskStatus, EngineError> {
        match s {
            "queued" => Ok(TaskStatus::Queued),
            "running" => Ok(TaskStatus::Running),
            "paused" => Ok(TaskStatus::Paused),
            "completed" => Ok(TaskStatus::Completed),
            "failed" => Ok(TaskStatus::Failed),
            "cancelled" => Ok(TaskStatus::Cancelled),
            other => Err(EngineError::InvalidArg(format!("unknown status: {other}"))),
        }
    }

    fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }

    fn row_to_task(row: &rusqlite::Row<'_>) -> Result<DownloadTask, rusqlite::Error> {
        let status_raw: String = row.get(6)?;
        let status = TaskStore::status_from_str(&status_raw).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                )),
            )
        })?;
        Ok(DownloadTask {
            id: row.get(0)?,
            parent_id: row.get(1)?,
            season: row.get::<_, Option<i64>>(2)?.map(|v| v as u32),
            title: row.get(3)?,
            source_url: row.get(4)?,
            quality_label: row.get(5)?,
            status,
            progress_bytes: row.get::<_, i64>(7)? as u64,
            total_bytes: row.get::<_, Option<i64>>(8)?.map(|v| v as u64),
            error_message: row.get(9)?,
            output_path: row.get(10)?,
            library_item_id: row.get(11)?,
            episode_index: row.get::<_, Option<i64>>(12)?.map(|v| v as u32),
            created_at_ms: row.get(13)?,
            updated_at_ms: row.get(14)?,
        })
    }

    pub fn upsert(&self, task: &DownloadTask) -> Result<(), EngineError> {
        self.conn.execute(
            r#"INSERT INTO download_tasks (
                 id, parent_id, season, title, source_url, quality_label, status,
                 progress_bytes, total_bytes, error_message, output_path,
                 library_item_id, episode_index, created_at_ms, updated_at_ms
               ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)
               ON CONFLICT(id) DO UPDATE SET
                 parent_id=excluded.parent_id,
                 season=excluded.season,
                 title=excluded.title,
                 source_url=excluded.source_url,
                 quality_label=excluded.quality_label,
                 status=excluded.status,
                 progress_bytes=excluded.progress_bytes,
                 total_bytes=excluded.total_bytes,
                 error_message=excluded.error_message,
                 output_path=excluded.output_path,
                 library_item_id=excluded.library_item_id,
                 episode_index=excluded.episode_index,
                 updated_at_ms=excluded.updated_at_ms"#,
            params![
                task.id,
                task.parent_id,
                task.season.map(|v| v as i64),
                task.title,
                task.source_url,
                task.quality_label,
                Self::status_to_str(task.status),
                task.progress_bytes as i64,
                task.total_bytes.map(|v| v as i64),
                task.error_message,
                task.output_path,
                task.library_item_id,
                task.episode_index.map(|v| v as i64),
                task.created_at_ms,
                task.updated_at_ms,
            ],
        )?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<DownloadTask, EngineError> {
        self.conn
            .query_row(
                r#"SELECT id, parent_id, season, title, source_url, quality_label, status,
                          progress_bytes, total_bytes, error_message, output_path,
                          library_item_id, episode_index, created_at_ms, updated_at_ms
                   FROM download_tasks WHERE id=?1"#,
                params![id],
                Self::row_to_task,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => EngineError::NotFound(format!("task {id}")),
                other => EngineError::Db(other),
            })
    }

    pub fn list_all(&self) -> Result<Vec<DownloadTask>, EngineError> {
        let mut stmt = self.conn.prepare(
            r#"SELECT id, parent_id, season, title, source_url, quality_label, status,
                      progress_bytes, total_bytes, error_message, output_path,
                      library_item_id, episode_index, created_at_ms, updated_at_ms
               FROM download_tasks ORDER BY created_at_ms DESC"#,
        )?;
        let rows = stmt.query_map([], Self::row_to_task)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn list_children(&self, parent_id: &str) -> Result<Vec<DownloadTask>, EngineError> {
        let mut stmt = self.conn.prepare(
            r#"SELECT id, parent_id, season, title, source_url, quality_label, status,
                      progress_bytes, total_bytes, error_message, output_path,
                      library_item_id, episode_index, created_at_ms, updated_at_ms
               FROM download_tasks WHERE parent_id=?1 ORDER BY episode_index ASC"#,
        )?;
        let rows = stmt.query_map(params![parent_id], Self::row_to_task)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn update_progress(
        &self,
        id: &str,
        progress_bytes: u64,
        total_bytes: Option<u64>,
        status: TaskStatus,
    ) -> Result<(), EngineError> {
        let n = self.conn.execute(
            r#"UPDATE download_tasks
               SET progress_bytes=?1, total_bytes=?2, status=?3, updated_at_ms=?4
               WHERE id=?5"#,
            params![
                progress_bytes as i64,
                total_bytes.map(|v| v as i64),
                Self::status_to_str(status),
                Self::now_ms(),
                id,
            ],
        )?;
        if n == 0 {
            return Err(EngineError::NotFound(format!("task {id}")));
        }
        Ok(())
    }

    pub fn mark_failed(&self, id: &str, message: &str) -> Result<(), EngineError> {
        let n = self.conn.execute(
            r#"UPDATE download_tasks
               SET status='failed', error_message=?1, updated_at_ms=?2
               WHERE id=?3"#,
            params![message, Self::now_ms(), id],
        )?;
        if n == 0 {
            return Err(EngineError::NotFound(format!("task {id}")));
        }
        Ok(())
    }

    pub fn parent_progress(&self, parent_id: &str) -> Result<(u32, u32), EngineError> {
        let total: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM download_tasks WHERE parent_id=?1",
            params![parent_id],
            |row| row.get::<_, i64>(0).map(|v| v as u32),
        )?;
        let done: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM download_tasks WHERE parent_id=?1 AND status='completed'",
            params![parent_id],
            |row| row.get::<_, i64>(0).map(|v| v as u32),
        )?;
        Ok((done, total))
    }

    pub fn save_checkpoint(
        &mut self,
        id: &str,
        checkpoint: &Checkpoint,
    ) -> Result<(), EngineError> {
        let json = serde_json::to_string(checkpoint)?;
        let n = self.conn.execute(
            "UPDATE download_tasks SET checkpoint_json=?1, updated_at_ms=?2 WHERE id=?3",
            params![json, Self::now_ms(), id],
        )?;
        if n == 0 {
            return Err(EngineError::NotFound(format!("task {id}")));
        }
        Ok(())
    }

    pub fn load_checkpoint(&self, id: &str) -> Result<Option<Checkpoint>, EngineError> {
        let json: Option<String> = self
            .conn
            .query_row(
                "SELECT checkpoint_json FROM download_tasks WHERE id=?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => EngineError::NotFound(format!("task {id}")),
                other => EngineError::Db(other),
            })?;
        match json {
            Some(raw) => Ok(Some(serde_json::from_str(&raw)?)),
            None => Ok(None),
        }
    }

    pub fn clear_checkpoint(&mut self, id: &str) -> Result<(), EngineError> {
        let n = self.conn.execute(
            "UPDATE download_tasks SET checkpoint_json=NULL, updated_at_ms=?1 WHERE id=?2",
            params![Self::now_ms(), id],
        )?;
        if n == 0 {
            return Err(EngineError::NotFound(format!("task {id}")));
        }
        Ok(())
    }

    pub fn update_progress_and_checkpoint(
        &mut self,
        id: &str,
        progress_bytes: u64,
        total_bytes: Option<u64>,
        checkpoint: &Checkpoint,
    ) -> Result<(), EngineError> {
        let json = serde_json::to_string(checkpoint)?;
        let tx = self.conn.unchecked_transaction()?;
        let n = tx.execute(
            r#"UPDATE download_tasks
               SET progress_bytes=?1, total_bytes=?2, checkpoint_json=?3, updated_at_ms=?4
               WHERE id=?5"#,
            params![
                progress_bytes as i64,
                total_bytes.map(|v| v as i64),
                json,
                Self::now_ms(),
                id,
            ],
        )?;
        if n == 0 {
            return Err(EngineError::NotFound(format!("task {id}")));
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_runnable_tasks(&self, limit: usize) -> Result<Vec<DownloadTask>, EngineError> {
        let mut stmt = self.conn.prepare(
            r#"SELECT id, parent_id, season, title, source_url, quality_label, status,
                      progress_bytes, total_bytes, error_message, output_path,
                      library_item_id, episode_index, created_at_ms, updated_at_ms
               FROM download_tasks
               WHERE status='queued'
                 AND source_url != ''
                 AND (parent_id IS NOT NULL OR episode_index IS NULL)
               ORDER BY created_at_ms ASC
               LIMIT ?1"#,
        )?;
        let rows = stmt.query_map(params![limit as i64], Self::row_to_task)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn set_task_status(
        &self,
        id: &str,
        status: TaskStatus,
        error_message: Option<&str>,
    ) -> Result<(), EngineError> {
        let n = self.conn.execute(
            r#"UPDATE download_tasks
               SET status=?1, error_message=?2, updated_at_ms=?3
               WHERE id=?4"#,
            params![
                Self::status_to_str(status),
                error_message,
                Self::now_ms(),
                id,
            ],
        )?;
        if n == 0 {
            return Err(EngineError::NotFound(format!("task {id}")));
        }
        Ok(())
    }

    pub fn set_output_path(&self, id: &str, path: &str) -> Result<(), EngineError> {
        let n = self.conn.execute(
            r#"UPDATE download_tasks SET output_path=?1, updated_at_ms=?2 WHERE id=?3"#,
            params![path, Self::now_ms(), id],
        )?;
        if n == 0 {
            return Err(EngineError::NotFound(format!("task {id}")));
        }
        Ok(())
    }

    pub fn set_library_item_id(&self, id: &str, library_item_id: &str) -> Result<(), EngineError> {
        let n = self.conn.execute(
            r#"UPDATE download_tasks SET library_item_id=?1, updated_at_ms=?2 WHERE id=?3"#,
            params![library_item_id, Self::now_ms(), id],
        )?;
        if n == 0 {
            return Err(EngineError::NotFound(format!("task {id}")));
        }
        Ok(())
    }

    pub fn sync_parent_status(&self, parent_id: &str) -> Result<(), EngineError> {
        let children = self.list_children(parent_id)?;
        if children.is_empty() {
            return Ok(());
        }

        let has_running = children.iter().any(|t| t.status == TaskStatus::Running);
        let has_queued = children.iter().any(|t| t.status == TaskStatus::Queued);
        let all_completed = children.iter().all(|t| t.status == TaskStatus::Completed);
        let all_terminal = children.iter().all(|t| {
            matches!(
                t.status,
                TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
            )
        });
        let any_failed = children.iter().any(|t| t.status == TaskStatus::Failed);

        let parent_status = if has_running {
            TaskStatus::Running
        } else if all_completed {
            TaskStatus::Completed
        } else if all_terminal && any_failed {
            TaskStatus::Failed
        } else if has_queued {
            TaskStatus::Queued
        } else {
            TaskStatus::Running
        };

        self.set_task_status(parent_id, parent_status, None)?;
        Ok(())
    }

    pub fn count_by_status(&self, status: TaskStatus) -> Result<u32, EngineError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM download_tasks WHERE status=?1",
            params![Self::status_to_str(status)],
            |row| row.get(0),
        )?;
        Ok(count as u32)
    }
}
