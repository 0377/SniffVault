use crate::error::EngineError;
use crate::ingest;
use crate::library::delete;
use crate::library::LibraryStore;
use crate::types::{LibraryItem, LibraryItemKind};
use std::path::Path;

pub(crate) fn validate_merge_pair(
    source: &LibraryItem,
    target: &LibraryItem,
) -> Result<(), EngineError> {
    if source.kind != LibraryItemKind::Series || target.kind != LibraryItemKind::Series {
        return Err(EngineError::InvalidArg(
            "merge requires both items to be series".into(),
        ));
    }
    if source.title != target.title {
        return Err(EngineError::InvalidArg(
            "merge requires matching title".into(),
        ));
    }
    if source.season != target.season {
        return Err(EngineError::InvalidArg(
            "merge requires matching season".into(),
        ));
    }
    Ok(())
}

pub(crate) fn merge_items(
    library: &LibraryStore,
    media_dir: &Path,
    source: &LibraryItem,
    target: &LibraryItem,
    delete_orphan_files: bool,
) -> Result<(), EngineError> {
    let source_eps = library.list_episodes(&source.id)?;

    let mut migrate: Vec<(String, String)> = Vec::new();
    let mut orphan_episode_ids: Vec<String> = Vec::new();
    let mut orphan_paths = Vec::new();

    for ep in &source_eps {
        if library
            .get_episode_by_item_index(&target.id, ep.index)?
            .is_some()
        {
            orphan_episode_ids.push(ep.id.clone());
            if delete_orphan_files {
                orphan_paths.push(ingest::ensure_path_in_media_dir(media_dir, &ep.file_path)?);
            }
        } else {
            migrate.push((ep.id.clone(), target.id.clone()));
        }
    }

    if delete_orphan_files && !orphan_paths.is_empty() {
        delete::delete_files(&orphan_paths)?;
    }

    library.apply_merge_in_tx(&migrate, &orphan_episode_ids, &source.id)
}
