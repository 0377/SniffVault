pub mod handle;
pub mod json_api;

pub use handle::{
    engine_destroy, engine_free_string, engine_last_error, engine_open, EngineHandle,
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
