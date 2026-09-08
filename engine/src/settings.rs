use crate::error::EngineError;
use crate::types::EngineSettings;
use std::path::Path;
use uuid::Uuid;

pub fn validate_media_dir(name: &str) -> Result<(), EngineError> {
    if name.is_empty() {
        return Err(EngineError::InvalidArg(
            "media_dir must not be empty".into(),
        ));
    }
    if name == "." {
        return Err(EngineError::InvalidArg("media_dir must not be '.'".into()));
    }
    if Path::new(name).is_absolute() {
        return Err(EngineError::InvalidArg(
            "media_dir must not be an absolute path".into(),
        ));
    }
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err(EngineError::InvalidArg(
            "media_dir must be a single relative directory name".into(),
        ));
    }
    Ok(())
}

fn ensure_device_id(settings: &mut EngineSettings) -> bool {
    if settings.device_id.is_empty() {
        settings.device_id = Uuid::new_v4().to_string();
        true
    } else {
        false
    }
}

pub fn load_or_default(path: &Path) -> Result<EngineSettings, EngineError> {
    if path.exists() {
        let raw = std::fs::read_to_string(path)?;
        let mut settings: EngineSettings = serde_json::from_str(&raw)?;
        validate_media_dir(&settings.media_dir)?;
        let migrated = ensure_device_id(&mut settings);
        if migrated {
            save(path, &settings)?;
        }
        Ok(settings)
    } else {
        let mut settings = EngineSettings::default();
        ensure_device_id(&mut settings);
        save(path, &settings)?;
        Ok(settings)
    }
}

pub fn save(path: &Path, settings: &EngineSettings) -> Result<(), EngineError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(settings)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, raw)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}
