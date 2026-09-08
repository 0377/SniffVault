use crate::error::EngineError;
use crate::lan::TrustedPeer;
use crate::tasks::schema::DB_PRAGMAS;
use rusqlite::{params, Connection};
use std::path::Path;

const TRUST_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS trusted_peers (
  peer_device_id TEXT PRIMARY KEY NOT NULL,
  peer_name TEXT NOT NULL,
  peer_host TEXT NOT NULL,
  peer_port INTEGER NOT NULL,
  paired_at_ms INTEGER NOT NULL,
  session_secret BLOB
);
"#;

pub struct TrustStore {
    conn: Connection,
}

impl TrustStore {
    pub fn open(db_path: &Path) -> Result<Self, EngineError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        conn.execute_batch(DB_PRAGMAS)?;
        conn.execute_batch(TRUST_SCHEMA)?;
        Ok(Self { conn })
    }

    pub fn upsert_peer(&self, peer: &TrustedPeer) -> Result<(), EngineError> {
        self.conn.execute(
            r#"INSERT INTO trusted_peers
               (peer_device_id, peer_name, peer_host, peer_port, paired_at_ms, session_secret)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)
               ON CONFLICT(peer_device_id) DO UPDATE SET
                 peer_name=excluded.peer_name,
                 peer_host=excluded.peer_host,
                 peer_port=excluded.peer_port,
                 paired_at_ms=excluded.paired_at_ms,
                 session_secret=excluded.session_secret"#,
            params![
                peer.peer_device_id,
                peer.peer_name,
                peer.peer_host,
                peer.peer_port,
                peer.paired_at_ms,
                peer.session_secret.as_ref(),
            ],
        )?;
        Ok(())
    }

    pub fn remove_peer(&self, peer_device_id: &str) -> Result<bool, EngineError> {
        let deleted = self.conn.execute(
            "DELETE FROM trusted_peers WHERE peer_device_id = ?1",
            params![peer_device_id],
        )?;
        Ok(deleted > 0)
    }

    pub fn list_peers(&self) -> Result<Vec<TrustedPeer>, EngineError> {
        let mut stmt = self.conn.prepare(
            r#"SELECT peer_device_id, peer_name, peer_host, peer_port, paired_at_ms, session_secret
               FROM trusted_peers
               ORDER BY paired_at_ms ASC"#,
        )?;
        let rows = stmt.query_map([], |row| {
            let secret_blob: Option<Vec<u8>> = row.get(5)?;
            Ok(TrustedPeer {
                peer_device_id: row.get(0)?,
                peer_name: row.get(1)?,
                peer_host: row.get(2)?,
                peer_port: row.get::<_, i64>(3)? as u16,
                paired_at_ms: row.get(4)?,
                session_secret: secret_from_blob(secret_blob)?,
            })
        })?;
        let mut peers = Vec::new();
        for row in rows {
            peers.push(row?);
        }
        Ok(peers)
    }
}

fn secret_from_blob(blob: Option<Vec<u8>>) -> Result<Option<[u8; 32]>, rusqlite::Error> {
    match blob {
        None => Ok(None),
        Some(bytes) if bytes.len() == 32 => {
            let mut secret = [0u8; 32];
            secret.copy_from_slice(&bytes);
            Ok(Some(secret))
        }
        Some(bytes) => Err(rusqlite::Error::FromSqlConversionFailure(
            5,
            rusqlite::types::Type::Blob,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("session_secret must be 32 bytes, got {}", bytes.len()),
            )),
        )),
    }
}
