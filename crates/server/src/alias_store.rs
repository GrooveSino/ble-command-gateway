use std::path::{Path, PathBuf};

use crate::device_name::DEFAULT_ALIAS;

const DEFAULT_ALIAS_FILE: &str = "/var/lib/yundrone/ble-alias";
const MAX_ALIAS_LEN: usize = 8;

pub fn alias_path() -> PathBuf {
    std::env::var("YUNDRONE_BLE_ALIAS_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_ALIAS_FILE))
}

pub fn parse_alias_input(raw: &str) -> Result<Option<String>, String> {
    let alias = raw.trim();
    if alias.is_empty() || alias == DEFAULT_ALIAS {
        return Ok(None);
    }
    validate_stored_alias(alias)?;
    Ok(Some(alias.to_string()))
}

pub fn load_alias(path: &Path, serial: &str) -> Option<String> {
    let contents = std::fs::read_to_string(path).ok()?;
    let mut lines = contents.lines();
    let alias = lines.next()?.trim();
    let bound = lines.next()?.trim();
    if bound != serial || validate_stored_alias(alias).is_err() {
        let _ = std::fs::remove_file(path);
        return None;
    }
    Some(alias.to_string())
}

pub fn save_alias(path: &Path, alias: &str, serial: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, format!("{alias}\n{serial}\n"))?;
    std::fs::rename(tmp, path)
}

pub fn clear_alias(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

fn validate_stored_alias(alias: &str) -> Result<(), String> {
    if alias == DEFAULT_ALIAS || alias == crate::device_name::NULL_DEVICE_SERIAL {
        return Err("alias is reserved".to_string());
    }
    if alias.is_empty() || alias.len() > MAX_ALIAS_LEN {
        return Err(format!("alias must be 1 to {MAX_ALIAS_LEN} characters"));
    }
    if !alias
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
    {
        return Err("alias must contain only lowercase ASCII letters and digits".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{clear_alias, load_alias, parse_alias_input, save_alias};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("yundrone-alias-{label}-{nanos}"))
    }

    #[test]
    fn parse_alias_input_clears_empty_and_default() {
        assert_eq!(parse_alias_input("").unwrap(), None);
        assert_eq!(parse_alias_input("  bleinit ").unwrap(), None);
        assert_eq!(parse_alias_input("lab1").unwrap(), Some("lab1".to_string()));
        assert!(parse_alias_input("null").is_err());
        assert!(parse_alias_input("lab-1").is_err());
        assert!(parse_alias_input("hangar001").is_err());
        assert!(parse_alias_input("Lab1").is_err());
    }

    #[test]
    fn load_alias_keeps_matching_file_and_resets_mismatch() {
        let path = temp_path("keep");
        save_alias(&path, "lab1", "12abcd").unwrap();
        assert_eq!(load_alias(&path, "12abcd").as_deref(), Some("lab1"));
        assert!(path.exists());

        let mismatch = temp_path("mismatch");
        save_alias(&mismatch, "lab1", "12abcd").unwrap();
        assert_eq!(load_alias(&mismatch, "34ef56"), None);
        assert!(!mismatch.exists());
    }

    #[test]
    fn load_alias_deletes_invalid_or_reserved_files() {
        let path = temp_path("invalid");
        std::fs::write(&path, "bleinit\n12abcd\n").unwrap();
        assert_eq!(load_alias(&path, "12abcd"), None);
        assert!(!path.exists());

        let dashed = temp_path("dashed");
        std::fs::write(&dashed, "lab-1\n12abcd\n").unwrap();
        assert_eq!(load_alias(&dashed, "12abcd"), None);
        assert!(!dashed.exists());
    }

    #[test]
    fn clear_alias_removes_existing_file() {
        let path = temp_path("clear");
        save_alias(&path, "lab1", "12abcd").unwrap();
        clear_alias(&path).unwrap();
        assert!(!path.exists());
        clear_alias(&path).unwrap();
    }
}
