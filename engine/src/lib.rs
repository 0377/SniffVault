pub(crate) mod download;
pub mod engine;
pub mod error;
pub mod library;
pub mod settings;
pub mod tasks;
pub mod types;

pub use engine::Engine;
pub use error::EngineError;
pub use types::*;

#[doc(hidden)]
pub mod test_api {
    pub use crate::download::checkpoint::{Checkpoint, CheckpointBody};
    pub use crate::download::mp4::{download_mp4, Mp4Context};

    pub async fn download_mp4_with_new_client(
        url: &str,
        temp_dir: &std::path::Path,
        output_mp4: &std::path::Path,
        checkpoint: Option<Checkpoint>,
    ) -> Result<(std::path::PathBuf, u64), crate::EngineError> {
        let http = crate::download::http::HttpClient::new(None)?;
        let ctx = Mp4Context {
            http: &http,
            temp_dir,
        };
        download_mp4(&ctx, url, output_mp4, checkpoint).await
    }
}
