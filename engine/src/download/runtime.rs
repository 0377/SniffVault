use crate::download::ffmpeg::BundledFfmpegLocator;
use crate::download::worker::{run_worker, DownloadCommand, WorkerConfig};
use crate::error::EngineError;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::JoinHandle;

pub struct DownloadRuntime {
    pub(crate) handle: JoinHandle<()>,
    pub(crate) cmd_tx: mpsc::Sender<DownloadCommand>,
}

impl DownloadRuntime {
    pub fn spawn(config: WorkerConfig) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("download tokio runtime");
            rt.block_on(run_worker(config, cmd_rx));
        });
        Self { handle, cmd_tx }
    }

    pub fn send_command(&self, cmd: DownloadCommand) -> Result<(), EngineError> {
        self.cmd_tx
            .send(cmd)
            .map_err(|_| EngineError::Message("download worker channel closed".into()))
    }

    pub fn stop_and_join(self) -> Result<(), EngineError> {
        let (ack_tx, ack_rx) = mpsc::channel();
        self.send_command(DownloadCommand::Stop { ack: ack_tx })?;
        ack_rx
            .recv()
            .map_err(|_| EngineError::Message("download worker stop ack dropped".into()))?;
        self.handle
            .join()
            .map_err(|_| EngineError::Message("download worker thread panicked".into()))?;
        Ok(())
    }
}

pub fn worker_config(
    data_dir: PathBuf,
    media_dir: PathBuf,
    max_concurrency: u32,
    user_agent: Option<String>,
    default_quality_label: Option<String>,
) -> WorkerConfig {
    WorkerConfig {
        data_dir,
        media_dir,
        max_concurrency,
        user_agent,
        default_quality_label,
        ffmpeg: Arc::new(BundledFfmpegLocator),
    }
}
