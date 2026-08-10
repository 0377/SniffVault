use crate::error::EngineError;
use crate::library::schema::LIBRARY_SCHEMA;
use crate::tasks::schema::DB_PRAGMAS;
use crate::types::{LibraryEpisode, LibraryItem, LibraryItemKind};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub struct LibraryStore {
    conn: Connection,
}

impl LibraryStore {
    pub fn open(db_path: &Path) -> Result<Self, EngineError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        conn.execute_batch(DB_PRAGMAS)?;
        conn.execute_batch(LIBRARY_SCHEMA)?;
        Ok(Self { conn })
    }

    fn kind_to_str(k: &LibraryItemKind) -> &'static str {
        match k {
            LibraryItemKind::Single => "single",
            LibraryItemKind::Series => "series",
        }
    }

    fn kind_from_str(s: &str) -> Result<LibraryItemKind, EngineError> {
        match s {
            "single" => Ok(LibraryItemKind::Single),
            "series" => Ok(LibraryItemKind::Series),
            other => Err(EngineError::InvalidArg(format!("unknown kind: {other}"))),
        }
    }

    pub fn upsert_item(&self, item: &LibraryItem) -> Result<(), EngineError> {
        self.conn.execute(
            r#"INSERT INTO library_items (id, kind, title, season, poster_path, created_at_ms)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)
               ON CONFLICT(id) DO UPDATE SET
                 kind=excluded.kind,
                 title=excluded.title,
                 season=excluded.season,
                 poster_path=excluded.poster_path"#,
            params![
                item.id,
                Self::kind_to_str(&item.kind),
                item.title,
                item.season.map(|s| s as i64),
                item.poster_path,
                item.created_at_ms,
            ],
        )?;
        Ok(())
    }

    pub fn upsert_episode(&self, ep: &LibraryEpisode) -> Result<(), EngineError> {
        self.conn.execute(
            r#"INSERT INTO library_episodes
               (id, item_id, idx, title, file_path, duration_ms, position_ms, source_url)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
               ON CONFLICT(item_id, idx) DO UPDATE SET
                 title=excluded.title,
                 file_path=excluded.file_path,
                 duration_ms=excluded.duration_ms,
                 source_url=excluded.source_url"#,
            params![
                ep.id,
                ep.item_id,
                ep.index as i64,
                ep.title,
                ep.file_path,
                ep.duration_ms,
                ep.position_ms,
                ep.source_url,
            ],
        )?;
        Ok(())
    }

    pub fn get_item(&self, id: &str) -> Result<LibraryItem, EngineError> {
        self.conn
            .query_row(
                "SELECT id, kind, title, season, poster_path, created_at_ms FROM library_items WHERE id=?1",
                params![id],
                |row| {
                    Ok(LibraryItem {
                        id: row.get(0)?,
                        kind: Self::kind_from_str(&row.get::<_, String>(1)?)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())),
                    ))?,
                        title: row.get(2)?,
                        season: row.get::<_, Option<i64>>(3)?.map(|v| v as u32),
                        poster_path: row.get(4)?,
                        created_at_ms: row.get(5)?,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    EngineError::NotFound(format!("library item {id}"))
                }
                other => EngineError::Db(other),
            })
    }

    pub fn list_items(&self) -> Result<Vec<LibraryItem>, EngineError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, title, season, poster_path, created_at_ms FROM library_items ORDER BY created_at_ms DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(LibraryItem {
                id: row.get(0)?,
                kind: match row.get::<_, String>(1)?.as_str() {
                    "single" => LibraryItemKind::Single,
                    "series" => LibraryItemKind::Series,
                    other => {
                        return Err(rusqlite::Error::FromSqlConversionFailure(
                            1,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("unknown kind: {other}"),
                            )),
                        ));
                    }
                },
                title: row.get(2)?,
                season: row.get::<_, Option<i64>>(3)?.map(|v| v as u32),
                poster_path: row.get(4)?,
                created_at_ms: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn list_episodes(&self, item_id: &str) -> Result<Vec<LibraryEpisode>, EngineError> {
        let mut stmt = self.conn.prepare(
            r#"SELECT id, item_id, idx, title, file_path, duration_ms, position_ms, source_url
               FROM library_episodes WHERE item_id=?1 ORDER BY idx ASC"#,
        )?;
        let rows = stmt.query_map(params![item_id], |row| {
            Ok(LibraryEpisode {
                id: row.get(0)?,
                item_id: row.get(1)?,
                index: row.get::<_, i64>(2)? as u32,
                title: row.get(3)?,
                file_path: row.get(4)?,
                duration_ms: row.get(5)?,
                position_ms: row.get(6)?,
                source_url: row.get(7)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn set_position(&self, episode_id: &str, position_ms: i64) -> Result<(), EngineError> {
        let n = self.conn.execute(
            "UPDATE library_episodes SET position_ms=?1 WHERE id=?2",
            params![position_ms, episode_id],
        )?;
        if n == 0 {
            return Err(EngineError::NotFound(format!("episode {episode_id}")));
        }
        Ok(())
    }

    pub fn find_series_by_title_season(
        &self,
        title: &str,
        season: Option<u32>,
    ) -> Result<Option<LibraryItem>, EngineError> {
        let season_i = season.map(|s| s as i64);
        let row = self
            .conn
            .query_row(
                r#"SELECT id, kind, title, season, poster_path, created_at_ms FROM library_items
                   WHERE kind='series' AND title=?1 AND (
                     (season IS NULL AND ?2 IS NULL) OR season=?2
                   )
                   LIMIT 1"#,
                params![title, season_i],
                |row| {
                    Ok(LibraryItem {
                        id: row.get(0)?,
                        kind: LibraryItemKind::Series,
                        title: row.get(2)?,
                        season: row.get::<_, Option<i64>>(3)?.map(|v| v as u32),
                        poster_path: row.get(4)?,
                        created_at_ms: row.get(5)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    pub fn get_episode_by_item_index(
        &self,
        item_id: &str,
        index: u32,
    ) -> Result<Option<LibraryEpisode>, EngineError> {
        let row = self
            .conn
            .query_row(
                r#"SELECT id, item_id, idx, title, file_path, duration_ms, position_ms, source_url
                   FROM library_episodes WHERE item_id=?1 AND idx=?2"#,
                params![item_id, index as i64],
                |row| {
                    Ok(LibraryEpisode {
                        id: row.get(0)?,
                        item_id: row.get(1)?,
                        index: row.get::<_, i64>(2)? as u32,
                        title: row.get(3)?,
                        file_path: row.get(4)?,
                        duration_ms: row.get(5)?,
                        position_ms: row.get(6)?,
                        source_url: row.get(7)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    pub fn remove_item(&self, id: &str) -> Result<(), EngineError> {
        self.conn
            .execute("DELETE FROM library_items WHERE id=?1", params![id])?;
        Ok(())
    }
}
