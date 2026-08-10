use crate::error::EngineError;
use crate::types::EngineSettings;
use std::path::Path;

pub fn validate_media_dir(name: &str) -> Result<(), EngineError> {
    if name.is_empty() {
        return Err(EngineError::InvalidArg(
            "media_dir must not be empty".into(),
        ));
    }
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err(EngineError::InvalidArg(
            "media_dir must be a single relative directory name".into(),
        ));
    }
    Ok(())
}

pub fn load_or_default(path: &Path) -> Result<EngineSettings, EngineError> {
    if path.exists() {
        let raw = std::fs::read_to_string(path)?;
        let settings: EngineSettings = serde_json::from_str(&raw)?;
        validate_media_dir(&settings.media_dir)?;
        Ok(settings)
    } else {
        let settings = EngineSettings::default();
        save(path, &settings)?;
        Ok(settings)
    }
}

pub fn save(path: &Path, settings: &EngineSettings) -> Result<(), EngineError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(settings)?;
    std::fs::write(path, raw)?;
    Ok(())
}
