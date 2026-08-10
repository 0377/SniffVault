use crate::download::ffmpeg::run_concat;
use crate::error::EngineError;
use std::path::{Path, PathBuf};

pub fn write_concat_list(segment_paths: &[PathBuf], list_path: &Path) -> Result<(), EngineError> {
    if segment_paths.is_empty() {
        return Err(EngineError::InvalidArg(
            "cannot write concat list for zero segments".into(),
        ));
    }

    let mut lines = String::new();
    for path in segment_paths {
        let escaped = path
            .to_string_lossy()
            .replace('\'', "'\\''");
        lines.push_str("file '");
        lines.push_str(&escaped);
        lines.push_str("'\n");
    }
    std::fs::write(list_path, lines)?;
    Ok(())
}

pub fn merge_segments_to_mp4(
    ffmpeg: &Path,
    segment_paths: &[PathBuf],
    temp_dir: &Path,
    output_mp4: &Path,
) -> Result<(), EngineError> {
    if let Some(parent) = output_mp4.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let concat_list = temp_dir.join("concat.txt");
    write_concat_list(segment_paths, &concat_list)?;
    run_concat(ffmpeg, &concat_list, output_mp4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn write_concat_list_escapes_single_quotes() {
        let dir = tempdir().unwrap();
        let segment = dir.path().join("it's.ts");
        let list_path = dir.path().join("concat.txt");

        write_concat_list(std::slice::from_ref(&segment), &list_path).unwrap();

        let content = std::fs::read_to_string(list_path).unwrap();
        assert!(content.contains("file '"));
        assert!(content.contains("it'\\''s.ts"));
    }
}
