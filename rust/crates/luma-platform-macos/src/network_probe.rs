//! On-demand local network probes for the Proxy surface only.

use async_trait::async_trait;
use luma_application::{NetworkProbePort, NetworkProbeState, NetworkProbeStep};
use std::time::Duration;
use tokio::process::Command;

const PROBE_TIMEOUT: Duration = Duration::from_millis(900);
pub struct MacNetworkProbe;

async fn command_ok(program: &str, args: &[&str]) -> Result<bool, String> {
    let output = tokio::time::timeout(PROBE_TIMEOUT, Command::new(program).args(args).output())
        .await
        .map_err(|_| "timed out".to_string())?
        .map_err(|error| error.to_string())?;
    Ok(output.status.success())
}

#[async_trait]
impl NetworkProbePort for MacNetworkProbe {
    async fn base_checks(&self) -> Vec<NetworkProbeStep> {
        let route = command_ok("/sbin/route", &["-n", "get", "default"]).await;
        let dns = command_ok(
            "/usr/bin/dscacheutil",
            &["-q", "host", "-a", "name", "example.com"],
        )
        .await;
        vec![
            step(
                "Default route",
                route,
                "Connect to a network with a default route",
            ),
            step(
                "DNS resolution",
                dns,
                "Check DNS settings or the selected network service",
            ),
        ]
    }
    async fn loopback_listener(&self, port: u16) -> NetworkProbeStep {
        let address = format!("127.0.0.1:{port}");
        let result =
            tokio::time::timeout(PROBE_TIMEOUT, tokio::net::TcpStream::connect(&address)).await;
        let passed = matches!(result, Ok(Ok(_)));
        NetworkProbeStep {
            name: format!("Local listener {address}"),
            state: if passed {
                NetworkProbeState::Pass
            } else {
                NetworkProbeState::Fail
            },
            detail: if passed {
                "accepting connections".into()
            } else {
                "not accepting connections".into()
            },
            remediation: "Start Mihomo or choose a configured local proxy port".into(),
        }
    }
}

fn step(name: &str, result: Result<bool, String>, remediation: &str) -> NetworkProbeStep {
    match result {
        Ok(true) => NetworkProbeStep {
            name: name.into(),
            state: NetworkProbeState::Pass,
            detail: "available".into(),
            remediation: remediation.into(),
        },
        Ok(false) => NetworkProbeStep {
            name: name.into(),
            state: NetworkProbeState::Fail,
            detail: "not available".into(),
            remediation: remediation.into(),
        },
        Err(detail) => NetworkProbeStep {
            name: name.into(),
            state: NetworkProbeState::Fail,
            detail,
            remediation: remediation.into(),
        },
    }
}
