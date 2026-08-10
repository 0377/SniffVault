pub const LIBRARY_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS library_items (
  id TEXT PRIMARY KEY NOT NULL,
  kind TEXT NOT NULL,
  title TEXT NOT NULL,
  season INTEGER,
  poster_path TEXT,
  created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS library_episodes (
  id TEXT PRIMARY KEY NOT NULL,
  item_id TEXT NOT NULL,
  idx INTEGER NOT NULL,
  title TEXT NOT NULL,
  file_path TEXT NOT NULL,
  duration_ms INTEGER,
  position_ms INTEGER NOT NULL DEFAULT 0,
  source_url TEXT,
  FOREIGN KEY(item_id) REFERENCES library_items(id) ON DELETE CASCADE,
  UNIQUE(item_id, idx)
);

CREATE INDEX IF NOT EXISTS idx_episodes_item ON library_episodes(item_id);
CREATE INDEX IF NOT EXISTS idx_items_title_season ON library_items(title, season);
"#;
