//! Finding a VIRB camera on the local network.
//!
//! Firmware 4.20 does not announce itself (no mDNS/Bonjour, no SSDP), so
//! discovery tries, in order, stopping at the first camera found:
//!
//! 1. the given candidate addresses (last used address, then 192.168.0.1,
//!    the camera's address when the device is on the camera's own Wi-Fi);
//! 2. a scan of the local /24 network(s): a quick TCP check of port 80 on
//!    every host, then a `deviceInfo` request only to hosts that answered.
//!
//! Only private IPv4 networks of this device's own interfaces are scanned.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures_util::{stream, StreamExt};
use serde::Serialize;
use tokio::net::TcpStream;

use crate::activity::{Reporter, Step};
use crate::camera::CameraClient;
use crate::virb::{GarminVirb360Client, VirbClientConfig};

/// Parallel TCP checks during a scan.
const SCAN_CONCURRENCY: usize = 64;
const PORT_CHECK_TIMEOUT: Duration = Duration::from_millis(500);
/// Report scan progress every this many hosts.
const PROGRESS_EVERY: u64 = 16;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredCamera {
    pub address: String,
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub device_id: Option<String>,
}

/// Runs discovery. Returns the cameras found (empty if none).
pub async fn discover(candidates: &[String], report: Reporter<'_>) -> Vec<DiscoveredCamera> {
    for address in unique(candidates) {
        report(Step::info("tryingAddress").param("address", address.as_str()));
        if let Some(camera) = probe(&address).await {
            report(found_step(&camera));
            return vec![camera];
        }
        report(Step::warning("addressNoCamera").param("address", address.as_str()));
    }

    let networks = local_networks();
    if networks.is_empty() {
        report(Step::warning("noLocalNetwork"));
        return Vec::new();
    }

    let mut found = Vec::new();
    for (own_ip, hosts) in networks {
        let network = format!("{}.0/24", prefix(own_ip));
        let total = hosts.len() as u64;
        report(
            Step::info("scanningNetwork")
                .param("network", network.as_str())
                .param("hosts", total)
                .progress(0, total),
        );

        let open = open_port_hosts(hosts, report).await;
        report(
            Step::info("hostsResponding")
                .param("count", open.len() as u64)
                .param("network", network.as_str()),
        );

        for ip in open {
            let address = ip.to_string();
            report(Step::info("probingHost").param("address", address.as_str()));
            if let Some(camera) = probe(&address).await {
                report(found_step(&camera));
                found.push(camera);
            }
        }
        // Cameras on further networks are unlikely; don't make the user wait.
        if !found.is_empty() {
            break;
        }
    }

    if found.is_empty() {
        report(Step::warning("noCameraFound"));
    }
    found
}

fn found_step(camera: &DiscoveredCamera) -> Step {
    Step::success("cameraFound")
        .param("address", camera.address.as_str())
        .param("model", camera.model.clone().unwrap_or_default())
}

/// Asks `deviceInfo` with short timeouts; only VIRB cameras qualify.
async fn probe(address: &str) -> Option<DiscoveredCamera> {
    let config = VirbClientConfig {
        connect_timeout: Duration::from_millis(800),
        command_timeout: Duration::from_millis(2500),
        ..VirbClientConfig::default()
    };
    let client = GarminVirb360Client::with_config(address, config).ok()?;
    let info = client.device_info().await.ok()?;
    let is_virb = info
        .model
        .as_deref()
        .is_some_and(|m| m.to_ascii_lowercase().contains("virb"));
    is_virb.then(|| DiscoveredCamera {
        address: address.to_string(),
        model: info.model,
        firmware: info.firmware,
        device_id: info.device_id,
    })
}

async fn open_port_hosts(hosts: Vec<Ipv4Addr>, report: Reporter<'_>) -> Vec<Ipv4Addr> {
    let total = hosts.len() as u64;
    let done = AtomicU64::new(0);
    let mut open: Vec<Ipv4Addr> = stream::iter(hosts)
        .map(|ip| {
            let done = &done;
            async move {
                let addr = SocketAddr::new(IpAddr::V4(ip), 80);
                let reachable = matches!(
                    tokio::time::timeout(PORT_CHECK_TIMEOUT, TcpStream::connect(addr)).await,
                    Ok(Ok(_))
                );
                let checked = done.fetch_add(1, Ordering::Relaxed) + 1;
                if checked % PROGRESS_EVERY == 0 || checked == total {
                    report(
                        Step::info("scanProgress")
                            .param("done", checked)
                            .param("total", total)
                            .progress(checked, total),
                    );
                }
                reachable.then_some(ip)
            }
        })
        .buffer_unordered(SCAN_CONCURRENCY)
        .filter_map(|ip| async move { ip })
        .collect()
        .await;
    open.sort();
    open
}

/// This device's private IPv4 addresses, each with the other hosts of its
/// /24 network. Larger networks are limited to the /24 around the address.
fn local_networks() -> Vec<(Ipv4Addr, Vec<Ipv4Addr>)> {
    let Ok(interfaces) = if_addrs::get_if_addrs() else {
        return Vec::new();
    };
    let mut networks: Vec<(Ipv4Addr, Vec<Ipv4Addr>)> = Vec::new();
    for interface in interfaces {
        let if_addrs::IfAddr::V4(v4) = interface.addr else {
            continue;
        };
        let ip = v4.ip;
        if ip.is_loopback() || !ip.is_private() || is_virtual_interface(&interface.name) {
            continue;
        }
        if networks.iter().any(|(own, _)| prefix(*own) == prefix(ip)) {
            continue;
        }
        networks.push((ip, hosts_of(ip)));
    }
    networks
}

/// Container, VM and VPN interfaces never lead to a camera.
fn is_virtual_interface(name: &str) -> bool {
    const PREFIXES: [&str; 10] = [
        "docker", "br-", "veth", "virbr", "vmnet", "vboxnet", "lxc", "lxd", "tun", "tap",
    ];
    let name = name.to_ascii_lowercase();
    PREFIXES.iter().any(|p| name.starts_with(p))
}

fn prefix(ip: Ipv4Addr) -> String {
    let [a, b, c, _] = ip.octets();
    format!("{a}.{b}.{c}")
}

/// Hosts .1–.254 of the /24 containing `own`, excluding `own`.
fn hosts_of(own: Ipv4Addr) -> Vec<Ipv4Addr> {
    let [a, b, c, _] = own.octets();
    (1..=254u8)
        .map(|d| Ipv4Addr::new(a, b, c, d))
        .filter(|ip| *ip != own)
        .collect()
}

fn unique(candidates: &[String]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for candidate in candidates {
        let trimmed = candidate.trim().to_string();
        if !trimmed.is_empty() && !seen.contains(&trimmed) {
            seen.push(trimmed);
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use serde_json::json;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[test]
    fn hosts_exclude_own_address() {
        let hosts = hosts_of(Ipv4Addr::new(192, 168, 1, 132));
        assert_eq!(hosts.len(), 253);
        assert!(!hosts.contains(&Ipv4Addr::new(192, 168, 1, 132)));
        assert_eq!(hosts[0], Ipv4Addr::new(192, 168, 1, 1));
    }

    #[test]
    fn skips_virtual_interfaces() {
        assert!(is_virtual_interface("virbr0"));
        assert!(is_virtual_interface("docker0"));
        assert!(!is_virtual_interface("wlp2s0"));
        assert!(!is_virtual_interface("wlan0"));
        assert!(!is_virtual_interface("eth0"));
    }

    #[test]
    fn candidates_are_deduplicated() {
        let list = unique(&[
            " 192.168.0.1".into(),
            "192.168.0.1".into(),
            "".into(),
            "10.0.0.2".into(),
        ]);
        assert_eq!(
            list,
            vec!["192.168.0.1".to_string(), "10.0.0.2".to_string()]
        );
    }

    #[tokio::test]
    async fn finds_a_virb_among_candidates_and_reports_steps() {
        let other = MockServer::start().await; // e.g. a router answering 404
        let virb = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/virb"))
            .and(body_json(json!({ "command": "deviceInfo" })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "deviceInfo": [{ "model": "VIRB 360", "firmware": 420, "deviceId": 1 }],
                "result": 1
            })))
            .mount(&virb)
            .await;
        let steps = Mutex::new(Vec::new());

        let found = discover(&[other.uri(), virb.uri()], &|step| {
            steps.lock().unwrap().push(step.code)
        })
        .await;

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].model.as_deref(), Some("VIRB 360"));
        assert_eq!(found[0].firmware.as_deref(), Some("4.20"));
        assert_eq!(
            steps.into_inner().unwrap(),
            vec![
                "tryingAddress",
                "addressNoCamera",
                "tryingAddress",
                "cameraFound"
            ]
        );
    }
}
