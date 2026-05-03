//! Open ports probe — discovers listening TCP/UDP ports.
//!
//! Linux: parses /proc/net/tcp and /proc/net/tcp6.
//! macOS / Windows: stubbed.

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use lso_core::{
    OpenPort, Platform, PrivilegeLevel, ProbeResult, SensorError, SystemMetric, SystemProbe,
};

pub const PROBE_ID: &str = "sensor.security.open_ports";

pub struct OpenPortsProbe {
    platform: Platform,
}

impl OpenPortsProbe {
    pub fn new(platform: Platform) -> Self {
        Self { platform }
    }

    pub fn for_host() -> Result<Self, SensorError> {
        let platform = Platform::detect().ok_or_else(|| SensorError::ProbeFailed {
            probe: PROBE_ID.into(),
            reason: format!("unknown platform: {}", std::env::consts::OS),
        })?;
        Ok(Self::new(platform))
    }
}

#[async_trait]
impl SystemProbe for OpenPortsProbe {
    fn probe_id(&self) -> &str {
        PROBE_ID
    }

    fn description(&self) -> &str {
        "Listening network ports"
    }

    fn required_privilege(&self) -> PrivilegeLevel {
        PrivilegeLevel::Unprivileged
    }

    async fn collect(&self) -> Result<ProbeResult, SensorError> {
        let ports = collect_open_ports(self.platform)?;
        let now = Utc::now();
        let platform_label = self.platform.to_string();

        let mut metrics: Vec<SystemMetric> = ports
            .iter()
            .map(|p| {
                let process = p.process_name.as_deref().unwrap_or("unknown");
                SystemMetric {
                    id: Uuid::new_v4(),
                    probe_id: PROBE_ID.into(),
                    name: format!("{}:{}::is_open", p.port, process),
                    value: 1.0,
                    unit: None,
                    collected_at: now,
                    platform: platform_label.clone(),
                }
            })
            .collect();

        let external_count = ports.iter().filter(|p| p.bind_address == "0.0.0.0" || p.bind_address == "::").count();

        metrics.push(SystemMetric {
            id: Uuid::new_v4(),
            probe_id: PROBE_ID.into(),
            name: "total::port_count".into(),
            value: ports.len() as f64,
            unit: None,
            collected_at: now,
            platform: platform_label.clone(),
        });
        metrics.push(SystemMetric {
            id: Uuid::new_v4(),
            probe_id: PROBE_ID.into(),
            name: "total::external_count".into(),
            value: external_count as f64,
            unit: None,
            collected_at: now,
            platform: platform_label,
        });

        Ok(ProbeResult {
            probe_id: PROBE_ID.into(),
            metrics,
            collected_at: now,
            platform: self.platform,
        })
    }
}

fn collect_open_ports(platform: Platform) -> Result<Vec<OpenPort>, SensorError> {
    match platform {
        Platform::Linux => {
            #[cfg(target_os = "linux")]
            {
                use lso_core::NetProtocol;
                let mut ports = Vec::new();
                ports.extend(parse_proc_net("/proc/net/tcp", NetProtocol::Tcp)?);
                ports.extend(parse_proc_net("/proc/net/tcp6", NetProtocol::Tcp6)?);
                Ok(ports)
            }
            #[cfg(not(target_os = "linux"))]
            {
                Ok(Vec::new())
            }
        }
        Platform::MacOS => {
            #[cfg(target_os = "macos")]
            {
                collect_open_ports_macos()
            }
            #[cfg(not(target_os = "macos"))]
            {
                tracing::info!("open ports probe: cross-compiled stub for macOS");
                Ok(Vec::new())
            }
        }
        Platform::Windows => {
            #[cfg(target_os = "windows")]
            {
                collect_open_ports_windows()
            }
            #[cfg(not(target_os = "windows"))]
            {
                tracing::info!("open ports probe: cross-compiled stub for Windows");
                Ok(Vec::new())
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn parse_proc_net(path: &str, protocol: NetProtocol) -> Result<Vec<OpenPort>, SensorError> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };

    Ok(content
        .lines()
        .skip(1)
        .filter_map(|line| parse_proc_net_line(line, protocol))
        .collect())
}

#[cfg(target_os = "linux")]
fn parse_proc_net_line(line: &str, protocol: NetProtocol) -> Option<OpenPort> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 4 || fields[3] != "0A" {
        return None;
    }

    let (hex_ip, hex_port) = fields[1].rsplit_once(':')?;
    let port = u16::from_str_radix(hex_port, 16).ok()?;
    let bind_address = decode_hex_ip(hex_ip, &protocol);

    Some(OpenPort {
        port,
        protocol,
        pid: None,
        process_name: None,
        bind_address,
    })
}

#[cfg(target_os = "linux")]
fn decode_hex_ip(hex: &str, protocol: &NetProtocol) -> String {
    match protocol {
        NetProtocol::Tcp | NetProtocol::Udp => {
            if let Ok(ip_u32) = u32::from_str_radix(hex, 16) {
                let b = ip_u32.to_le_bytes();
                return format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]);
            }
            hex.into()
        }
        NetProtocol::Tcp6 | NetProtocol::Udp6 => {
            if hex == "00000000000000000000000000000000" {
                return "::".into();
            }
            if hex == "00000000000000000000000001000000" {
                return "::1".into();
            }
            format!("ipv6:{hex}")
        }
    }
}

// ---------------------------------------------------------------------------
// macOS — parse `lsof -nP -iTCP -sTCP:LISTEN`
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn collect_open_ports_macos() -> Result<Vec<OpenPort>, SensorError> {
    use lso_core::NetProtocol;
    use std::process::Command;

    let output = match Command::new("lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN", "-F", "pcn"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return Ok(Vec::new()),
    };

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut ports = Vec::new();
    let mut current_pid: Option<u32> = None;
    let mut current_name: Option<String> = None;

    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix('p') {
            current_pid = rest.parse().ok();
        } else if let Some(rest) = line.strip_prefix('c') {
            current_name = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix('n') {
            if let Some(port_info) = parse_lsof_name(rest) {
                ports.push(OpenPort {
                    port: port_info.0,
                    protocol: if port_info.1 { NetProtocol::Tcp6 } else { NetProtocol::Tcp },
                    pid: current_pid,
                    process_name: current_name.clone(),
                    bind_address: port_info.2,
                });
            }
        }
    }

    Ok(ports)
}

#[cfg(target_os = "macos")]
fn parse_lsof_name(name: &str) -> Option<(u16, bool, String)> {
    // Format: "addr:port" or "[addr]:port" or "*:port"
    let is_v6 = name.starts_with('[') || name.contains("::") || name.contains("IPv6");

    let (addr, port_str) = if let Some(bracket_end) = name.find("]:") {
        let addr = &name[1..bracket_end];
        let port = &name[bracket_end + 2..];
        (addr.to_string(), port)
    } else if let Some(colon_pos) = name.rfind(':') {
        let addr = &name[..colon_pos];
        let port = &name[colon_pos + 1..];
        (
            if addr == "*" {
                "0.0.0.0".to_string()
            } else {
                addr.to_string()
            },
            port,
        )
    } else {
        return None;
    };

    let port: u16 = port_str.parse().ok()?;
    Some((port, is_v6, addr))
}

// ---------------------------------------------------------------------------
// Windows — parse `netstat -ano` output
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn collect_open_ports_windows() -> Result<Vec<OpenPort>, SensorError> {
    use lso_core::NetProtocol;
    use std::process::Command;

    let output = match Command::new("netstat").args(["-ano"]).output() {
        Ok(o) => o,
        Err(_) => return Ok(Vec::new()),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut ports = Vec::new();

    for line in stdout.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 4 {
            continue;
        }
        if fields[3] != "LISTENING" {
            continue;
        }

        let proto = match fields[0] {
            "TCP" => NetProtocol::Tcp,
            _ => continue,
        };

        if let Some((addr, port_str)) = fields[1].rsplit_once(':') {
            if let Ok(port) = port_str.parse::<u16>() {
                let pid = fields.get(4).and_then(|s| s.parse::<u32>().ok());
                let bind = if addr.contains(':') { "::" } else { addr };
                ports.push(OpenPort {
                    port,
                    protocol: if addr.contains(':') { NetProtocol::Tcp6 } else { proto },
                    pid,
                    process_name: None,
                    bind_address: bind.to_string(),
                });
            }
        }
    }

    Ok(ports)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_id_correct() {
        let probe = OpenPortsProbe::new(Platform::Linux);
        assert_eq!(probe.probe_id(), PROBE_ID);
    }

    #[tokio::test]
    async fn collect_returns_summary_metrics() {
        let probe = OpenPortsProbe::new(Platform::detect().unwrap_or(Platform::Linux));
        let result = probe.collect().await.unwrap();
        let names: Vec<&str> = result.metrics.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"total::port_count"));
        assert!(names.contains(&"total::external_count"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parse_listen_line() {
        let line = "   0: 00000000:0016 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 12345 1 0000000000000000 100 0 0 10 0";
        let port = parse_proc_net_line(line, NetProtocol::Tcp).unwrap();
        assert_eq!(port.port, 22);
        assert_eq!(port.bind_address, "0.0.0.0");
    }
}
