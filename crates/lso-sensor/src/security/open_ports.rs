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
        _ => {
            tracing::info!("open ports probe: stubbed on {platform}");
            Ok(Vec::new())
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
