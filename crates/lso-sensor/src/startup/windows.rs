//! Windows startup item discovery: Registry Run keys + shell:startup folder.

use lso_core::{SensorError, StartupImpact, StartupItem, StartupType};

#[cfg(target_os = "windows")]
pub fn scan_startup_items() -> Result<Vec<StartupItem>, SensorError> {
    use windows_sys::Win32::System::Registry::*;
    use windows_sys::Win32::Foundation::*;

    let mut items = Vec::new();

    let keys = [
        (HKEY_CURRENT_USER, "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run"),
        (HKEY_LOCAL_MACHINE, "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run"),
    ];

    for (root, subkey) in &keys {
        match read_run_key(*root, subkey) {
            Ok(found) => items.extend(found),
            Err(e) => tracing::warn!(subkey, "registry scan failed: {e}"),
        }
    }

    if let Ok(found) = scan_startup_folder() {
        items.extend(found);
    }

    Ok(items)
}

#[cfg(target_os = "windows")]
fn read_run_key(
    root: windows_sys::Win32::System::Registry::HKEY,
    subkey: &str,
) -> Result<Vec<StartupItem>, SensorError> {
    use windows_sys::Win32::System::Registry::*;
    use windows_sys::Win32::Foundation::*;

    let subkey_wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    let mut hkey: HKEY = 0;

    let ret = unsafe {
        RegOpenKeyExW(root, subkey_wide.as_ptr(), 0, KEY_READ, &mut hkey)
    };
    if ret != ERROR_SUCCESS {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    let mut index: u32 = 0;
    let mut name_buf = [0u16; 256];
    let mut data_buf = [0u8; 1024];

    loop {
        let mut name_len = name_buf.len() as u32;
        let mut data_len = data_buf.len() as u32;
        let mut value_type: u32 = 0;

        let ret = unsafe {
            RegEnumValueW(
                hkey,
                index,
                name_buf.as_mut_ptr(),
                &mut name_len,
                std::ptr::null_mut(),
                &mut value_type,
                data_buf.as_mut_ptr(),
                &mut data_len,
            )
        };

        if ret != ERROR_SUCCESS {
            break;
        }

        let name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
        let command = if value_type == REG_SZ || value_type == REG_EXPAND_SZ {
            let words = data_len as usize / 2;
            let data_u16: Vec<u16> = data_buf[..words * 2]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            let end = data_u16.iter().position(|&c| c == 0).unwrap_or(data_u16.len());
            String::from_utf16_lossy(&data_u16[..end])
        } else {
            String::new()
        };

        items.push(StartupItem {
            name: name.clone(),
            item_type: StartupType::RegistryRun,
            command,
            enabled: true,
            publisher: None,
            impact: StartupImpact::Unknown,
            platform_id: name,
        });

        index += 1;
    }

    unsafe { RegCloseKey(hkey) };
    Ok(items)
}

#[cfg(target_os = "windows")]
fn scan_startup_folder() -> Result<Vec<StartupItem>, SensorError> {
    let startup_dir = dirs::config_dir()
        .map(|d| {
            d.parent()
                .unwrap_or(&d)
                .join("Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\Startup")
        })
        .ok_or_else(|| SensorError::ProbeFailed {
            probe: "startup.folder".into(),
            reason: "could not locate startup folder".into(),
        })?;

    if !startup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    for entry in std::fs::read_dir(&startup_dir)?.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        items.push(StartupItem {
            name: name.clone(),
            item_type: StartupType::ScheduledTask,
            command: path.display().to_string(),
            enabled: true,
            publisher: None,
            impact: StartupImpact::Unknown,
            platform_id: name,
        });
    }

    Ok(items)
}

#[cfg(not(target_os = "windows"))]
pub fn scan_startup_items() -> Result<Vec<StartupItem>, SensorError> {
    Ok(Vec::new())
}
