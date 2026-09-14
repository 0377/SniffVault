use crate::error::EngineError;
use crate::ingest;
use crate::library::delete;
use crate::library::LibraryStore;
use crate::types::{LibraryEpisode, LibraryItem, LibraryItemKind};
use std::collections::HashSet;
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

pub(crate) fn orphan_episode_ids_for_merge(
    library: &LibraryStore,
    source_eps: &[LibraryEpisode],
    target_item_id: &str,
) -> Result<Vec<String>, EngineError> {
    let mut ids = Vec::new();
    for ep in source_eps {
        if library
            .get_episode_by_item_index(target_item_id, ep.index)?
            .is_some()
        {
            ids.push(ep.id.clone());
        }
    }
    Ok(ids)
}

pub(crate) fn merge_items(
    library: &LibraryStore,
    media_dir: &Path,
    source: &LibraryItem,
    target: &LibraryItem,
    delete_orphan_files: bool,
) -> Result<(), EngineError> {
    let source_eps = library.list_episodes(&source.id)?;
    let orphan_episode_ids = orphan_episode_ids_for_merge(library, &source_eps, &target.id)?;
    let orphan_ids: HashSet<_> = orphan_episode_ids.iter().cloned().collect();

    let mut migrate: Vec<(String, String)> = Vec::new();
    let mut orphan_paths = Vec::new();

    for ep in &source_eps {
        if orphan_ids.contains(&ep.id) {
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

    library.apply_merge_in_tx(&migrate, &orphan_episode_ids, &source.id, &target.id)
}
