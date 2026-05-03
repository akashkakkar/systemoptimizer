//! macOS startup item discovery: LaunchAgents + LaunchDaemons (plist files).

use std::path::PathBuf;

use lso_core::{SensorError, StartupImpact, StartupItem, StartupType};

pub fn scan_startup_items() -> Result<Vec<StartupItem>, SensorError> {
    let mut items = Vec::new();

    let home = std::env::var("HOME").unwrap_or_default();

    let dirs = [
        (
            PathBuf::from(&home).join("Library/LaunchAgents"),
            StartupType::LaunchAgent,
        ),
        (
            PathBuf::from("/Library/LaunchAgents"),
            StartupType::LaunchAgent,
        ),
        (
            PathBuf::from("/Library/LaunchDaemons"),
            StartupType::LaunchDaemon,
        ),
    ];

    for (dir, item_type) in &dirs {
        if !dir.exists() {
            continue;
        }
        match scan_plist_dir(dir, *item_type) {
            Ok(found) => items.extend(found),
            Err(e) => tracing::warn!(dir = %dir.display(), "plist scan failed: {e}"),
        }
    }

    Ok(items)
}

fn scan_plist_dir(
    dir: &std::path::Path,
    item_type: StartupType,
) -> Result<Vec<StartupItem>, SensorError> {
    let entries = std::fs::read_dir(dir).map_err(SensorError::Io)?;
    let mut items = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "plist" {
            continue;
        }
        if let Some(item) = parse_plist_file(&path, item_type) {
            items.push(item);
        }
    }

    Ok(items)
}

fn parse_plist_file(path: &std::path::Path, item_type: StartupType) -> Option<StartupItem> {
    let content = std::fs::read_to_string(path).ok()?;

    let label = extract_plist_string(&content, "Label")?;
    let program = extract_plist_string(&content, "Program")
        .or_else(|| extract_plist_array_first(&content, "ProgramArguments"))
        .unwrap_or_default();

    let disabled = content.contains("<key>Disabled</key>")
        && plist_bool_after_key(&content, "Disabled");

    Some(StartupItem {
        name: label.clone(),
        item_type,
        command: program,
        enabled: !disabled,
        publisher: None,
        impact: StartupImpact::Unknown,
        platform_id: label,
    })
}

fn extract_plist_string(content: &str, key: &str) -> Option<String> {
    let key_tag = format!("<key>{key}</key>");
    let pos = content.find(&key_tag)?;
    let after = &content[pos + key_tag.len()..];
    let start = after.find("<string>")? + 8;
    let end = after[start..].find("</string>")?;
    Some(after[start..start + end].to_string())
}

fn extract_plist_array_first(content: &str, key: &str) -> Option<String> {
    let key_tag = format!("<key>{key}</key>");
    let pos = content.find(&key_tag)?;
    let after = &content[pos + key_tag.len()..];
    let array_start = after.find("<array>")?;
    let after_array = &after[array_start..];
    let start = after_array.find("<string>")? + 8;
    let end = after_array[start..].find("</string>")?;
    Some(after_array[start..start + end].to_string())
}

fn plist_bool_after_key(content: &str, key: &str) -> bool {
    let key_tag = format!("<key>{key}</key>");
    if let Some(pos) = content.find(&key_tag) {
        let after = &content[pos + key_tag.len()..];
        let trimmed = after.trim_start();
        return trimmed.starts_with("<true/>");
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plist_label() {
        let plist = r#"<?xml version="1.0"?>
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.example.agent</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/bin/example</string>
        <string>--flag</string>
    </array>
</dict>
</plist>"#;

        let item = parse_plist_file(std::path::Path::new("test.plist"), StartupType::LaunchAgent);
        // Can't test without a real file; test the parser helpers instead
        let label = extract_plist_string(plist, "Label").unwrap();
        assert_eq!(label, "com.example.agent");

        let prog = extract_plist_array_first(plist, "ProgramArguments").unwrap();
        assert_eq!(prog, "/usr/bin/example");
    }

    #[test]
    fn disabled_detection() {
        let plist_disabled = r#"<key>Disabled</key>
        <true/>"#;
        assert!(plist_bool_after_key(plist_disabled, "Disabled"));

        let plist_enabled = r#"<key>Disabled</key>
        <false/>"#;
        assert!(!plist_bool_after_key(plist_enabled, "Disabled"));
    }
}
