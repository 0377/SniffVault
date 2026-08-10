pub const DB_PRAGMAS: &str =
    "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;";

pub const TASK_SCHEMA_VERSION: i64 = 2;

pub const TASK_MIGRATION_V2: &str = "ALTER TABLE download_tasks ADD COLUMN checkpoint_json TEXT;";

pub const TASK_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS download_tasks (
  id TEXT PRIMARY KEY NOT NULL,
  parent_id TEXT,
  season INTEGER,
  title TEXT NOT NULL,
  source_url TEXT NOT NULL,
  quality_label TEXT,
  status TEXT NOT NULL,
  progress_bytes INTEGER NOT NULL DEFAULT 0,
  total_bytes INTEGER,
  error_message TEXT,
  output_path TEXT,
  library_item_id TEXT,
  episode_index INTEGER,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_tasks_parent ON download_tasks(parent_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON download_tasks(status);
"#;
