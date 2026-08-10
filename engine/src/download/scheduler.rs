use crate::error::EngineError;
use crate::tasks::TaskStore;
use crate::types::DownloadTask;

pub struct Scheduler {
    max_concurrency: u32,
    paused_globally: bool,
}

impl Scheduler {
    pub fn new(max_concurrency: u32) -> Self {
        Self {
            max_concurrency,
            paused_globally: false,
        }
    }

    pub fn pause_globally(&mut self) {
        self.paused_globally = true;
    }

    pub fn available_slots(&self, active: usize) -> usize {
        if self.paused_globally {
            return 0;
        }
        let max = self.max_concurrency as usize;
        max.saturating_sub(active)
    }

    pub fn pick_next(
        &self,
        store: &TaskStore,
        active: usize,
        limit: usize,
    ) -> Result<Vec<DownloadTask>, EngineError> {
        let slots = self.available_slots(active);
        if slots == 0 {
            return Ok(Vec::new());
        }
        let take = slots.min(limit);
        store.list_runnable_tasks(take)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TaskStatus;
    use tempfile::tempdir;

    fn sample(id: &str, parent: Option<&str>, url: &str, status: TaskStatus) -> DownloadTask {
        DownloadTask {
            id: id.into(),
            parent_id: parent.map(|p| p.into()),
            season: Some(1),
            title: id.into(),
            source_url: url.into(),
            quality_label: None,
            status,
            progress_bytes: 0,
            total_bytes: None,
            error_message: None,
            output_path: None,
            library_item_id: None,
            episode_index: if parent.is_some() { Some(1) } else { None },
            created_at_ms: 1,
            updated_at_ms: 1,
        }
    }

    #[test]
    fn pick_next_respects_concurrency_slots() {
        let dir = tempdir().unwrap();
        let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();
        store
            .upsert(&sample("parent", None, "", TaskStatus::Queued))
            .unwrap();
        for i in 1..=4 {
            let id = format!("c{i}");
            store
                .upsert(&sample(
                    &id,
                    Some("parent"),
                    &format!("https://ex/{i}.mp4"),
                    TaskStatus::Queued,
                ))
                .unwrap();
        }

        let scheduler = Scheduler::new(2);
        let picked = scheduler.pick_next(&store, 0, 10).unwrap();
        assert_eq!(picked.len(), 2);

        let picked2 = scheduler.pick_next(&store, 2, 10).unwrap();
        assert_eq!(picked2.len(), 0);

        let picked3 = scheduler.pick_next(&store, 1, 10).unwrap();
        assert_eq!(picked3.len(), 1);
    }

    #[test]
    fn pick_next_skips_when_globally_paused() {
        let dir = tempdir().unwrap();
        let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();
        store
            .upsert(&sample(
                "single",
                None,
                "https://ex/a.mp4",
                TaskStatus::Queued,
            ))
            .unwrap();

        let mut scheduler = Scheduler::new(2);
        scheduler.pause_globally();
        let picked = scheduler.pick_next(&store, 0, 10).unwrap();
        assert!(picked.is_empty());
    }

    #[test]
    fn pick_next_includes_enqueue_single_tasks() {
        let dir = tempdir().unwrap();
        let store = TaskStore::open(&dir.path().join("tasks.db")).unwrap();
        store
            .upsert(&sample(
                "single",
                None,
                "https://ex/movie.mp4",
                TaskStatus::Queued,
            ))
            .unwrap();

        let scheduler = Scheduler::new(1);
        let picked = scheduler.pick_next(&store, 0, 10).unwrap();
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].id, "single");
    }
}
