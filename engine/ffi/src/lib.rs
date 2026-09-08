pub mod async_resolve;
pub mod cast_events;
pub mod events;
pub mod handle;
pub mod json_api;
pub mod sync_dispatch;

pub use async_resolve::{engine_resolve_qualities_async, engine_resolve_url_async};
pub use cast_events::{engine_subscribe_cast_events, engine_unsubscribe_cast_events};
pub use events::{engine_subscribe_task_events, engine_unsubscribe_task_events};
pub use handle::{
    engine_destroy, engine_free_string, engine_last_error, engine_open, EngineHandle,
};
pub use sync_dispatch::{
    engine_apply_lan_settings, engine_begin_pairing, engine_cancel_task, engine_cast_episode,
    engine_discover_peers, engine_enqueue_episodes, engine_enqueue_single, engine_list_episodes,
    engine_list_library, engine_list_tasks, engine_list_trusted_peers, engine_pair_peer,
    engine_pairing_pin, engine_pause_task, engine_remove_trusted_peer, engine_resume_task,
    engine_save_settings, engine_set_episode_position, engine_settings, engine_sniff_urls,
    engine_spawn_download_worker, engine_start_downloads, engine_start_lan, engine_stop_cast,
    engine_stop_downloads, engine_stop_lan,
};

#[cfg(test)]
mod tests {
    use video_sniffing_engine::EngineError;

    #[test]
    fn crate_links() {
        let _ = EngineError::Message("ffi scaffold".into());
        let _ = serde_json::json!({ "ok": true });
        let _ = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("tokio runtime");
        let _ = allo_isolate::Isolate::new(0);
    }
}
