use std::fs;
use uuid::Uuid;
use video_sniffing_engine::test_api::LibraryStore;
use video_sniffing_engine::{Engine, LibraryEpisode, LibraryItem, LibraryItemKind};

pub struct DuplicateSeriesSeed {
    pub source_item_id: String,
    pub target_item_id: String,
}

pub fn seed_duplicate_series(
    engine: &Engine,
    title: &str,
    season: Option<u32>,
) -> DuplicateSeriesSeed {
    let media_dir = engine.media_dir();
    let data_dir = media_dir.parent().unwrap();
    let store = LibraryStore::open(&data_dir.join("library.db")).unwrap();
    let source_item_id = Uuid::new_v4().to_string();
    let target_item_id = Uuid::new_v4().to_string();
    store
        .upsert_item(&LibraryItem {
            id: source_item_id.clone(),
            kind: LibraryItemKind::Series,
            title: title.into(),
            season,
            poster_path: None,
            created_at_ms: 1,
        })
        .unwrap();
    store
        .upsert_item(&LibraryItem {
            id: target_item_id.clone(),
            kind: LibraryItemKind::Series,
            title: title.into(),
            season,
            poster_path: None,
            created_at_ms: 2,
        })
        .unwrap();
    DuplicateSeriesSeed {
        source_item_id,
        target_item_id,
    }
}

pub fn add_episode(
    engine: &Engine,
    item_id: &str,
    index: u32,
    title: &str,
    file_name: &str,
    position_ms: i64,
) -> LibraryEpisode {
    let media = engine.media_dir().join(file_name);
    fs::write(&media, b"x").unwrap();
    let store = LibraryStore::open(
        &engine
            .media_dir()
            .parent()
            .unwrap()
            .join("library.db"),
    )
    .unwrap();
    let ep = LibraryEpisode {
        id: Uuid::new_v4().to_string(),
        item_id: item_id.into(),
        index,
        title: title.into(),
        file_path: media.to_string_lossy().into(),
        duration_ms: Some(10_000),
        position_ms,
        source_url: None,
    };
    store.upsert_episode(&ep).unwrap();
    ep
}
