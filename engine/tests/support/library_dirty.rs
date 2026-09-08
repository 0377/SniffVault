use video_sniffing_engine::test_api::LibraryStore;
use video_sniffing_engine::Engine;

pub fn inject_outside_file_path(engine: &Engine, item_id: &str, outside_path: &str) {
    let media_dir = engine.media_dir();
    let data_dir = media_dir.parent().expect("media dir parent");
    let store = LibraryStore::open(&data_dir.join("library.db")).unwrap();
    let episodes = store.list_episodes(item_id).unwrap();
    let ep = episodes.first().expect("episode exists");
    let mut dirty = ep.clone();
    dirty.file_path = outside_path.to_string();
    store.upsert_episode(&dirty).unwrap();
}
