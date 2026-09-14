use crate::types::DownloadTask;

pub fn sanitize_filename(title: &str) -> String {
    let mut out: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.len() > 120 {
        out.truncate(120);
    }
    if out.is_empty() {
        "download".into()
    } else {
        out
    }
}

pub fn output_filename(task: &DownloadTask) -> String {
    let base = sanitize_filename(&task.title);
    if task.parent_id.is_some() {
        if let Some(index) = task.episode_index {
            if let Some(season) = task.season {
                return format!("{base}_S{season}E{index}.mp4");
            }
            return format!("{base}_E{index}.mp4");
        }
    }
    format!("{base}.mp4")
}
