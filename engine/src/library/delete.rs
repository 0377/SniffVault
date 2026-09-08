use crate::error::EngineError;
use crate::ingest;
use crate::library::LibraryStore;
use crate::types::LibraryItem;
use std::path::{Path, PathBuf};

pub(crate) fn resolve_deletion_path(
    media_dir: &Path,
    file_path: &str,
) -> Result<PathBuf, EngineError> {
    match ingest::ensure_path_in_media_dir(media_dir, file_path) {
        Ok(path) => Ok(path),
        Err(EngineError::InvalidArg(msg))
            if msg.contains("not accessible") && !PathBuf::from(file_path).exists() =>
        {
            let path = PathBuf::from(file_path);
            let media = media_dir
                .canonicalize()
                .unwrap_or_else(|_| media_dir.to_path_buf());
            let parent = path
                .parent()
                .ok_or_else(|| EngineError::InvalidArg("file path has no parent".into()))?;
            let parent_canon = parent
                .canonicalize()
                .map_err(|e| EngineError::InvalidArg(format!("file path not accessible: {e}")))?;
            let resolved = parent_canon.join(
                path.file_name()
                    .ok_or_else(|| EngineError::InvalidArg("file path has no file name".into()))?,
            );
            if !resolved.starts_with(&media) {
                return Err(EngineError::InvalidArg(
                    "file path must be under media_dir".into(),
                ));
            }
            Ok(resolved)
        }
        Err(e) => Err(e),
    }
}

pub(crate) fn collect_deletion_paths(
    library: &LibraryStore,
    item: &LibraryItem,
    media_dir: &Path,
) -> Result<Vec<PathBuf>, EngineError> {
    let mut paths = Vec::new();
    for ep in library.list_episodes(&item.id)? {
        paths.push(resolve_deletion_path(media_dir, &ep.file_path)?);
    }
    if let Some(poster) = &item.poster_path {
        paths.push(resolve_deletion_path(media_dir, poster)?);
    }
    Ok(paths)
}

pub(crate) fn delete_files(paths: &[PathBuf]) -> Result<(), EngineError> {
    for path in paths {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(EngineError::Io(e)),
        }
    }
    Ok(())
}

pub(crate) fn remove_item_record(library: &LibraryStore, item_id: &str) -> Result<(), EngineError> {
    library.get_item(item_id)?;
    let tx = library.conn().unchecked_transaction()?;
    library.remove_item_in_tx(&tx, item_id)?;
    tx.commit()?;
    Ok(())
}
