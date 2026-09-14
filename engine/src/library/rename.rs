use crate::error::EngineError;

const MAX_DISPLAY_TITLE_LEN: usize = 512;

pub fn validate_display_title(title: &str) -> Result<String, EngineError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(EngineError::InvalidArg("title must not be empty".into()));
    }
    if trimmed.len() > MAX_DISPLAY_TITLE_LEN {
        return Err(EngineError::InvalidArg(format!(
            "title must be at most {MAX_DISPLAY_TITLE_LEN} characters"
        )));
    }
    Ok(trimmed.to_string())
}
