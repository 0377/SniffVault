pub mod handle;
pub mod json_api;
pub mod sync_dispatch;

pub use handle::{
    engine_destroy, engine_free_string, engine_last_error, engine_open, EngineHandle,
};
pub use sync_dispatch::{
    engine_cancel_task, engine_enqueue_episodes, engine_enqueue_single, engine_list_episodes,
    engine_list_library, engine_list_tasks, engine_pause_task, engine_resume_task,
    engine_save_settings, engine_set_episode_position, engine_settings, engine_sniff_urls,
    engine_start_downloads, engine_stop_downloads,
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
