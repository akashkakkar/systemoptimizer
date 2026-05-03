//! Linux startup item discovery: systemd services + XDG autostart.

use std::path::PathBuf;
use std::process::Command;

use lso_core::{SensorError, StartupImpact, StartupItem, StartupType};

pub fn scan_startup_items() -> Result<Vec<StartupItem>, SensorError> {
    let mut items = Vec::new();

    match scan_systemd() {
        Ok(systemd_items) => items.extend(systemd_items),
        Err(e) => tracing::warn!("systemd scan failed: {e}"),
    }

    match scan_xdg_autostart() {
        Ok(autostart_items) => items.extend(autostart_items),
        Err(e) => tracing::warn!("XDG autostart scan failed: {e}"),
    }

    Ok(items)
}

fn scan_systemd() -> Result<Vec<StartupItem>, SensorError> {
    let output = Command::new("systemctl")
        .args(["list-unit-files", "--type=service", "--state=enabled", "--no-pager", "--no-legend"])
        .output()
        .map_err(|e| SensorError::ProbeFailed {
            probe: "startup.systemd".into(),
            reason: format!("failed to run systemctl: {e}"),
        })?;

    if !output.status.success() {
        return Err(SensorError::ProbeFailed {
            probe: "startup.systemd".into(),
            reason: format!("systemctl exited with {}", output.status),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().filter_map(parse_systemd_line).collect())
}

fn parse_systemd_line(line: &str) -> Option<StartupItem> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let unit_name = parts[0];
    let name = unit_name.strip_suffix(".service").unwrap_or(unit_name);

    Some(StartupItem {
        name: name.into(),
        item_type: StartupType::SystemdService,
        command: format!("systemctl start {unit_name}"),
        enabled: true,
        publisher: publisher_from_unit(unit_name),
        impact: StartupImpact::Unknown,
        platform_id: unit_name.into(),
    })
}

fn publisher_from_unit(unit_name: &str) -> Option<String> {
    let unit_path = PathBuf::from("/usr/lib/systemd/system").join(unit_name);
    let fallback = PathBuf::from("/etc/systemd/system").join(unit_name);

    let path = if unit_path.exists() {
        unit_path
    } else if fallback.exists() {
        fallback
    } else {
        return None;
    };

    let content = std::fs::read_to_string(path).ok()?;
    content
        .lines()
        .find_map(|l| l.trim().strip_prefix("Description=").map(String::from))
}

fn scan_xdg_autostart() -> Result<Vec<StartupItem>, SensorError> {
    let home = std::env::var("HOME").map_err(|_| SensorError::ProbeFailed {
        probe: "startup.autostart".into(),
        reason: "$HOME not set".into(),
    })?;

    let autostart_dir = PathBuf::from(home).join(".config/autostart");
    if !autostart_dir.exists() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    for entry in std::fs::read_dir(&autostart_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
            continue;
        }
        if let Some(item) = parse_desktop_file(&path) {
            items.push(item);
        }
    }

    Ok(items)
}

fn parse_desktop_file(path: &std::path::Path) -> Option<StartupItem> {
    let content = std::fs::read_to_string(path).ok()?;

    let mut name = None;
    let mut exec = None;
    let mut hidden = false;
    let mut comment = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(val) = trimmed.strip_prefix("Name=") {
            name = Some(val.to_string());
        } else if let Some(val) = trimmed.strip_prefix("Exec=") {
            exec = Some(val.to_string());
        } else if let Some(val) = trimmed.strip_prefix("Hidden=") {
            hidden = val.eq_ignore_ascii_case("true");
        } else if let Some(val) = trimmed.strip_prefix("Comment=") {
            comment = Some(val.to_string());
        }
    }

    let file_stem = path.file_stem()?.to_string_lossy().to_string();
    let display_name = name.unwrap_or_else(|| file_stem.clone());

    Some(StartupItem {
        name: display_name,
        item_type: StartupType::DesktopAutostart,
        command: exec.unwrap_or_default(),
        enabled: !hidden,
        publisher: comment,
        impact: StartupImpact::Unknown,
        platform_id: file_stem,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_systemd_line_valid() {
        let item = parse_systemd_line("docker.service enabled enabled").unwrap();
        assert_eq!(item.name, "docker");
        assert_eq!(item.item_type, StartupType::SystemdService);
        assert!(item.enabled);
    }

    #[test]
    fn parse_systemd_line_too_short() {
        assert!(parse_systemd_line("").is_none());
        assert!(parse_systemd_line("only_one").is_none());
    }

    #[test]
    fn parse_desktop_file_roundtrip() {
        let dir = std::env::temp_dir().join("lso_test_startup");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test-app.desktop");
        std::fs::write(
            &path,
            "[Desktop Entry]\nName=TestApp\nExec=/usr/bin/testapp\nHidden=false\nComment=Publisher\n",
        )
        .unwrap();

        let item = parse_desktop_file(&path).unwrap();
        assert_eq!(item.name, "TestApp");
        assert_eq!(item.command, "/usr/bin/testapp");
        assert!(item.enabled);
        assert_eq!(item.publisher.as_deref(), Some("Publisher"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
