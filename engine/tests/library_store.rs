use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::tempdir;
use video_sniffing_engine::library::LibraryStore;
use video_sniffing_engine::{LibraryEpisode, LibraryItem, LibraryItemKind};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

#[test]
fn upsert_series_episodes_and_progress() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("library.db");
    let store = LibraryStore::open(&db).expect("open");

    let item = LibraryItem {
        id: "series-1".into(),
        kind: LibraryItemKind::Series,
        title: "示意剧".into(),
        season: Some(1),
        poster_path: None,
        created_at_ms: now_ms(),
    };
    store.upsert_item(&item).unwrap();

    let ep = LibraryEpisode {
        id: "ep-1".into(),
        item_id: item.id.clone(),
        index: 1,
        title: "第1集".into(),
        file_path: dir.path().join("ep1.mp4").to_string_lossy().into(),
        duration_ms: Some(600_000),
        position_ms: 0,
        source_url: Some("https://example.com/ep1.m3u8".into()),
    };
    store.upsert_episode(&ep).unwrap();
    store.set_position("ep-1", 12_345).unwrap();

    let listed = store.list_items().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].title, "示意剧");

    let eps = store.list_episodes("series-1").unwrap();
    assert_eq!(eps.len(), 1);
    assert_eq!(eps[0].position_ms, 12_345);

    let found = store
        .find_series_by_title_season("示意剧", Some(1))
        .unwrap();
    assert_eq!(found.unwrap().id, "series-1");
}

#[test]
fn remove_item_cascades_episodes() {
    let dir = tempdir().unwrap();
    let store = LibraryStore::open(&dir.path().join("library.db")).unwrap();
    store
        .upsert_item(&LibraryItem {
            id: "s".into(),
            kind: LibraryItemKind::Single,
            title: "单片".into(),
            season: None,
            poster_path: None,
            created_at_ms: 1,
        })
        .unwrap();
    store
        .upsert_episode(&LibraryEpisode {
            id: "e".into(),
            item_id: "s".into(),
            index: 1,
            title: "单片".into(),
            file_path: "/tmp/x.mp4".into(),
            duration_ms: None,
            position_ms: 0,
            source_url: None,
        })
        .unwrap();
    store.remove_item("s").unwrap();
    assert!(store.list_items().unwrap().is_empty());
    assert!(store.list_episodes("s").unwrap().is_empty());
}

#[test]
fn remove_episode_deletes_one_row() {
    let dir = tempdir().unwrap();
    let store = LibraryStore::open(&dir.path().join("library.db")).unwrap();
    store
        .upsert_item(&LibraryItem {
            id: "s".into(),
            kind: LibraryItemKind::Series,
            title: "剧".into(),
            season: Some(1),
            poster_path: None,
            created_at_ms: 1,
        })
        .unwrap();
    store
        .upsert_episode(&LibraryEpisode {
            id: "e1".into(),
            item_id: "s".into(),
            index: 1,
            title: "第1集".into(),
            file_path: "/tmp/e1.mp4".into(),
            duration_ms: None,
            position_ms: 0,
            source_url: None,
        })
        .unwrap();
    store
        .upsert_episode(&LibraryEpisode {
            id: "e2".into(),
            item_id: "s".into(),
            index: 2,
            title: "第2集".into(),
            file_path: "/tmp/e2.mp4".into(),
            duration_ms: None,
            position_ms: 0,
            source_url: None,
        })
        .unwrap();

    store.remove_episode("e1").unwrap();
    assert_eq!(store.count_episodes("s").unwrap(), 1);
    assert!(store.get_episode("e1").unwrap().is_none());
    assert!(store.get_episode("e2").unwrap().is_some());
}

#[test]
fn remove_episode_missing_returns_not_found() {
    let dir = tempdir().unwrap();
    let store = LibraryStore::open(&dir.path().join("library.db")).unwrap();
    let err = store.remove_episode("nope").unwrap_err();
    assert!(err.to_string().contains("not found"));
}
