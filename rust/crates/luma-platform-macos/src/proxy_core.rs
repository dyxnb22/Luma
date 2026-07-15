//! Mihomo controller adapter. It only connects to loopback or the configured Unix socket.
//!
//! No raw controller response is returned to callers and no authentication material is included
//! in errors or tracing output.

use async_trait::async_trait;
use luma_application::{
    ExternalControllerStatus, KeychainPort, ProxyCoreError, ProxyCorePort, ProxyGroup, ProxyMode,
    ProxyNode, ProxyPorts, ProxyStatus,
};
use serde_json::Value;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UnixStream};

const DEFAULT_UNIX_SOCKET: &str = "/tmp/verge/verge-mihomo.sock";
const DEFAULT_EFFECTIVE_CONFIG: &str =
    "Library/Application Support/io.github.clash-verge-rev.clash-verge-rev/clash-verge.yaml";
const DEFAULT_CONTROLLER: SocketAddr =
    SocketAddr::new(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 9097);
const MAX_RESPONSE_BYTES: usize = 512 * 1024;
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_CONFIG_BYTES: u64 = 512 * 1024;

#[derive(Clone, Debug)]
enum Endpoint {
    Tcp(SocketAddr),
    Unix(PathBuf),
}

impl Endpoint {
    fn label(&self) -> String {
        match self {
            Self::Tcp(addr) => addr.to_string(),
            Self::Unix(path) => format!("unix:{}", path.display()),
        }
    }
}

pub struct MacMihomoProxyCore {
    endpoints: Vec<Endpoint>,
    secret: Option<String>,
    secret_keychain: Option<std::sync::Arc<dyn KeychainPort>>,
    secret_account: Option<String>,
    configuration_error: Option<String>,
    timeout: Duration,
    effective_config: Option<PathBuf>,
    profiles_manifest: Option<PathBuf>,
}

impl Default for MacMihomoProxyCore {
    fn default() -> Self {
        Self::new_default()
    }
}

impl MacMihomoProxyCore {
    /// Prefer the existing Clash Verge Rev Unix socket, then its loopback TCP controller.
    pub fn new_default() -> Self {
        Self {
            endpoints: vec![
                Endpoint::Unix(PathBuf::from(DEFAULT_UNIX_SOCKET)),
                Endpoint::Tcp(DEFAULT_CONTROLLER),
            ],
            secret: None,
            secret_keychain: None,
            secret_account: None,
            configuration_error: None,
            timeout: Duration::from_millis(900),
            effective_config: default_effective_config_path(),
            profiles_manifest: default_profiles_manifest_path(),
        }
    }

    pub fn with_unix_socket(path: PathBuf) -> Self {
        Self {
            endpoints: vec![Endpoint::Unix(path)],
            secret: None,
            secret_keychain: None,
            secret_account: None,
            configuration_error: None,
            timeout: Duration::from_millis(900),
            effective_config: default_effective_config_path(),
            profiles_manifest: default_profiles_manifest_path(),
        }
    }

    /// Only loopback addresses are accepted. This constructor is intentionally not a generic
    /// host/URL parser, so LAN and public controller endpoints cannot slip into the adapter.
    pub fn with_loopback_controller(addr: SocketAddr) -> Result<Self, ProxyCoreError> {
        if !addr.ip().is_loopback() {
            return Err(ProxyCoreError::SecurityDenied(
                "external controller must be loopback or a Unix socket".into(),
            ));
        }
        Ok(Self {
            endpoints: vec![Endpoint::Tcp(addr)],
            secret: None,
            secret_keychain: None,
            secret_account: None,
            configuration_error: None,
            timeout: Duration::from_millis(900),
            effective_config: default_effective_config_path(),
            profiles_manifest: default_profiles_manifest_path(),
        })
    }

    /// Test/configuration hook for the read-only effective-config fallback.
    pub fn with_effective_config_path(mut self, path: Option<PathBuf>) -> Self {
        self.effective_config = path;
        self
    }

    pub fn with_profiles_manifest_path(mut self, path: Option<PathBuf>) -> Self {
        self.profiles_manifest = path;
        self
    }

    pub fn with_secret(mut self, secret: Option<String>) -> Self {
        self.secret = secret;
        self
    }

    pub fn with_keychain_secret(
        mut self,
        keychain: std::sync::Arc<dyn KeychainPort>,
        account: impl Into<String>,
    ) -> Self {
        self.secret_keychain = Some(keychain);
        self.secret_account = Some(account.into());
        self
    }

    /// Build the controller adapter from Luma settings. Invalid configured addresses never
    /// broaden the connection policy: they leave the adapter unavailable instead.
    pub fn from_settings(
        settings: &luma_storage::LumaSettings,
        keychain: std::sync::Arc<dyn KeychainPort>,
    ) -> Self {
        let mut endpoints = Vec::new();
        let mut configuration_error = None;
        if let Some(path) = settings
            .proxy_controller_unix_socket
            .as_deref()
            .filter(|path| !path.is_empty())
        {
            endpoints.push(Endpoint::Unix(PathBuf::from(path)));
        } else if settings.proxy_controller_unix_socket.is_none() {
            endpoints.push(Endpoint::Unix(PathBuf::from(DEFAULT_UNIX_SOCKET)));
        }
        if let Some(address) = settings
            .proxy_controller_address
            .as_deref()
            .filter(|address| !address.is_empty())
        {
            match address.parse::<SocketAddr>() {
                Ok(addr) if addr.ip().is_loopback() => endpoints.push(Endpoint::Tcp(addr)),
                _ => {
                    configuration_error =
                        Some("controller address must be a loopback socket address".into())
                }
            }
        } else if settings.proxy_controller_address.is_none() {
            endpoints.push(Endpoint::Tcp(DEFAULT_CONTROLLER));
        }
        Self {
            endpoints,
            secret: None,
            secret_keychain: settings
                .proxy_controller_secret_account
                .as_ref()
                .map(|_| keychain),
            secret_account: settings.proxy_controller_secret_account.clone(),
            configuration_error,
            timeout: Duration::from_millis(900),
            effective_config: default_effective_config_path(),
            profiles_manifest: default_profiles_manifest_path(),
        }
    }

    fn effective_profile(&self) -> Option<String> {
        let value = self
            .effective_config
            .as_ref()
            .and_then(|path| {
                let metadata = std::fs::metadata(path).ok()?;
                (metadata.len() <= MAX_CONFIG_BYTES).then_some(())?;
                let raw = std::fs::read_to_string(path).ok()?;
                serde_yaml::from_str(&raw).ok()
            })
            .unwrap_or(serde_yaml::Value::Null);
        let direct = value
            .get("profile")
            .and_then(|profile| {
                profile.as_str().map(str::to_string).or_else(|| {
                    profile
                        .get("name")
                        .and_then(serde_yaml::Value::as_str)
                        .map(str::to_string)
                })
            })
            .or_else(|| {
                value
                    .get("profile-name")
                    .and_then(serde_yaml::Value::as_str)
                    .map(str::to_string)
            })
            .or_else(|| {
                value
                    .get("name")
                    .and_then(serde_yaml::Value::as_str)
                    .map(str::to_string)
            })
            .or_else(|| self.profile_from_manifest(&value));
        direct
    }

    fn profile_from_manifest(&self, effective: &serde_yaml::Value) -> Option<String> {
        let path = self.profiles_manifest.as_ref()?;
        let metadata = std::fs::metadata(path).ok()?;
        if metadata.len() > MAX_CONFIG_BYTES {
            return None;
        }
        let raw = std::fs::read_to_string(path).ok()?;
        let manifest: serde_yaml::Value = serde_yaml::from_str(&raw).ok()?;
        let current = manifest
            .get("current")
            .and_then(serde_yaml::Value::as_str)
            .or_else(|| effective.get("current").and_then(serde_yaml::Value::as_str))
            .or_else(|| {
                effective
                    .get("profile")
                    .and_then(|profile| profile.get("current"))
                    .and_then(serde_yaml::Value::as_str)
            })?;
        let items = manifest
            .get("items")
            .or_else(|| manifest.get("profiles"))
            .and_then(serde_yaml::Value::as_sequence)?;
        items.iter().find_map(|item| {
            let uid = item
                .get("uid")
                .or_else(|| item.get("id"))
                .and_then(serde_yaml::Value::as_str)?;
            (uid == current)
                .then(|| item.get("name").and_then(serde_yaml::Value::as_str))
                .flatten()
                .map(str::to_string)
        })
    }

    async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
    ) -> Result<Vec<u8>, ProxyCoreError> {
        if let Some(reason) = &self.configuration_error {
            return Err(ProxyCoreError::InvalidInput {
                field: "proxy_controller_address".into(),
                message: reason.clone(),
            });
        }
        let secret = self.controller_secret().await?;
        let mut last = None;
        for endpoint in &self.endpoints {
            match self
                .request_endpoint(endpoint, method, path, body, secret.as_deref())
                .await
            {
                Ok(body) => return Ok(body),
                Err(err @ ProxyCoreError::Timeout(_))
                | Err(err @ ProxyCoreError::Unavailable(_)) => last = Some(err),
                Err(err) => return Err(err),
            }
        }
        Err(last.unwrap_or_else(|| {
            ProxyCoreError::Unavailable("no controller endpoint configured".into())
        }))
    }

    async fn controller_secret(&self) -> Result<Option<String>, ProxyCoreError> {
        if let Some(secret) = &self.secret {
            return Ok(Some(secret.clone()));
        }
        let (Some(keychain), Some(account)) = (&self.secret_keychain, &self.secret_account) else {
            return Ok(None);
        };
        keychain
            .copy_password(account)
            .await
            .map(Some)
            .map_err(|_| ProxyCoreError::Unavailable("controller secret is unavailable".into()))
    }

    async fn request_endpoint(
        &self,
        endpoint: &Endpoint,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
        secret: Option<&str>,
    ) -> Result<Vec<u8>, ProxyCoreError> {
        let body = body.unwrap_or_default();
        let mut request = format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nAccept: application/json\r\n"
        );
        if let Some(secret) = secret {
            request.push_str("Authorization: Bearer ");
            request.push_str(secret);
            request.push_str("\r\n");
        }
        if !body.is_empty() {
            request.push_str("Content-Type: application/json\r\nContent-Length: ");
            request.push_str(&body.len().to_string());
            request.push_str("\r\n");
        }
        request.push_str("\r\n");

        let operation = format!("{method} {path}");
        let response = tokio::time::timeout(self.timeout, async {
            match endpoint {
                Endpoint::Tcp(addr) => {
                    let mut stream = TcpStream::connect(addr).await.map_err(|e| e.to_string())?;
                    stream
                        .write_all(request.as_bytes())
                        .await
                        .map_err(|e| e.to_string())?;
                    stream.write_all(body).await.map_err(|e| e.to_string())?;
                    read_response(&mut stream).await
                }
                Endpoint::Unix(path) => {
                    let mut stream = UnixStream::connect(path).await.map_err(|e| e.to_string())?;
                    stream
                        .write_all(request.as_bytes())
                        .await
                        .map_err(|e| e.to_string())?;
                    stream.write_all(body).await.map_err(|e| e.to_string())?;
                    read_response(&mut stream).await
                }
            }
        })
        .await
        .map_err(|_| ProxyCoreError::Timeout(operation.clone()))?
        .map_err(|reason| {
            ProxyCoreError::Unavailable(format!("controller connection failed: {reason}"))
        })?;

        let (status, body) = response;
        if !(200..300).contains(&status) {
            return Err(match status {
                401 | 403 => ProxyCoreError::PermissionRequired(
                    "Mihomo controller rejected the request; check controller permissions".into(),
                ),
                404 => ProxyCoreError::NotFound("controller endpoint".into()),
                _ => ProxyCoreError::Unavailable(format!("controller returned HTTP {status}")),
            });
        }
        Ok(body)
    }

    async fn json(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, ProxyCoreError> {
        let bytes = self
            .request(
                method,
                path,
                body.map(|v| serde_json::to_vec(&v).unwrap_or_default())
                    .as_deref(),
            )
            .await?;
        serde_json::from_slice(&bytes)
            .map_err(|_| ProxyCoreError::Unavailable("Mihomo returned an invalid response".into()))
    }

    async fn patch(&self, path: &str, body: Value) -> Result<(), ProxyCoreError> {
        self.request(
            "PATCH",
            path,
            Some(&serde_json::to_vec(&body).map_err(|_| {
                ProxyCoreError::Unavailable("could not encode controller request".into())
            })?),
        )
        .await
        .map(|_| ())
    }

    async fn put(&self, path: &str, body: Value) -> Result<(), ProxyCoreError> {
        self.request(
            "PUT",
            path,
            Some(&serde_json::to_vec(&body).map_err(|_| {
                ProxyCoreError::Unavailable("could not encode controller request".into())
            })?),
        )
        .await
        .map(|_| ())
    }
}

async fn read_response<S: AsyncReadExt + Unpin>(stream: &mut S) -> Result<(u16, Vec<u8>), String> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    let header_end;
    loop {
        let n = stream.read(&mut buffer).await.map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("controller closed before sending a complete response".into());
        }
        bytes.extend_from_slice(&buffer[..n]);
        if bytes.len() > MAX_RESPONSE_BYTES + MAX_HEADER_BYTES {
            return Err("controller response exceeded the size limit".into());
        }
        if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            header_end = pos + 4;
            break;
        }
        if bytes.len() > MAX_HEADER_BYTES {
            return Err("controller headers exceeded the size limit".into());
        }
    }
    let header = String::from_utf8_lossy(&bytes[..header_end]);
    let status = header
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u16>().ok())
        .ok_or_else(|| "invalid controller status line".to_string())?;
    let content_length = header.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("content-length")
            .then(|| value.trim().parse::<usize>().ok())
            .flatten()
    });
    let chunked = header.lines().any(|line| {
        let (name, value) = line.split_once(':').unwrap_or(("", ""));
        name.eq_ignore_ascii_case("transfer-encoding")
            && value
                .split(',')
                .any(|encoding| encoding.trim().eq_ignore_ascii_case("chunked"))
    });
    if matches!(status, 100..=199 | 204 | 304) {
        return Ok((status, Vec::new()));
    }
    if let Some(want) = content_length {
        if want > MAX_RESPONSE_BYTES {
            return Err("controller response exceeded the size limit".into());
        }
        while bytes.len() - header_end < want {
            let n = stream.read(&mut buffer).await.map_err(|e| e.to_string())?;
            if n == 0 {
                return Err("controller response body was truncated".into());
            }
            bytes.extend_from_slice(&buffer[..n]);
            if bytes.len() - header_end > MAX_RESPONSE_BYTES {
                return Err("controller response exceeded the size limit".into());
            }
        }
        return Ok((status, bytes[header_end..header_end + want].to_vec()));
    }
    loop {
        let n = stream.read(&mut buffer).await.map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..n]);
        if bytes.len() - header_end > MAX_RESPONSE_BYTES {
            return Err("controller response exceeded the size limit".into());
        }
    }
    let body = &bytes[header_end..];
    if chunked {
        decode_chunked(body).map(|body| (status, body))
    } else {
        Ok((status, body.to_vec()))
    }
}

fn decode_chunked(raw: &[u8]) -> Result<Vec<u8>, String> {
    let mut cursor = 0;
    let mut decoded = Vec::new();
    loop {
        let Some(line_end) = raw[cursor..].windows(2).position(|w| w == b"\r\n") else {
            return Err("chunked response was truncated".into());
        };
        let line_end = cursor + line_end;
        let size_text = String::from_utf8_lossy(&raw[cursor..line_end]);
        let size_text = size_text.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_text, 16)
            .map_err(|_| "invalid chunked response size".to_string())?;
        cursor = line_end + 2;
        if size == 0 {
            return Ok(decoded);
        }
        if size > MAX_RESPONSE_BYTES || decoded.len() + size > MAX_RESPONSE_BYTES {
            return Err("controller response exceeded the size limit".into());
        }
        if raw.len() < cursor + size + 2 || &raw[cursor + size..cursor + size + 2] != b"\r\n" {
            return Err("chunked response was truncated".into());
        }
        decoded.extend_from_slice(&raw[cursor..cursor + size]);
        cursor += size + 2;
    }
}

fn path_segment(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn u16_field(value: &Value, key: &str) -> Option<u16> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|v| u16::try_from(v).ok())
        .filter(|v| *v != 0)
}

fn node_from_value(name: &str, value: &Value, group: Option<&str>, selected: bool) -> ProxyNode {
    let delay_ms = value
        .get("history")
        .and_then(Value::as_array)
        .and_then(|history| {
            history
                .iter()
                .rev()
                .find_map(|v| v.get("delay").and_then(Value::as_u64))
        })
        .and_then(|delay| u32::try_from(delay).ok());
    ProxyNode {
        name: name.to_string(),
        kind: string_field(value, "type").unwrap_or_else(|| "proxy".into()),
        delay_ms,
        selected,
        group: group.map(str::to_string),
    }
}

fn group_type(kind: &str) -> bool {
    matches!(
        kind,
        "Selector" | "URLTest" | "Fallback" | "LoadBalance" | "Relay" | "Compatible"
    )
}

#[async_trait]
impl ProxyCorePort for MacMihomoProxyCore {
    async fn get_status(&self) -> Result<ProxyStatus, ProxyCoreError> {
        let value = self.json("GET", "/configs", None).await?;
        let mode = match string_field(&value, "mode").as_deref() {
            Some("global") => ProxyMode::Global,
            _ => ProxyMode::Rule,
        };
        let profile = value
            .get("profile")
            .and_then(|p| {
                p.as_str()
                    .map(str::to_string)
                    .or_else(|| string_field(p, "name"))
            })
            .or_else(|| self.effective_profile());
        let tun_enabled = value
            .get("tun")
            .and_then(|tun| tun.get("enable").and_then(Value::as_bool))
            .unwrap_or(false);
        Ok(ProxyStatus {
            running: true,
            mode,
            profile,
            ports: ProxyPorts {
                http: u16_field(&value, "port"),
                mixed: u16_field(&value, "mixed-port"),
                socks: u16_field(&value, "socks-port"),
            },
            allow_lan: value
                .get("allow-lan")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            tun_enabled,
        })
    }

    async fn get_mode(&self) -> Result<ProxyMode, ProxyCoreError> {
        Ok(self.get_status().await?.mode)
    }

    async fn set_mode(&self, mode: ProxyMode) -> Result<(), ProxyCoreError> {
        self.patch("/configs", serde_json::json!({"mode": mode.as_str()}))
            .await
    }

    async fn list_proxy_groups(&self) -> Result<Vec<ProxyGroup>, ProxyCoreError> {
        let value = self.json("GET", "/proxies", None).await?;
        let Some(entries) = value.get("proxies").and_then(Value::as_object) else {
            return Err(ProxyCoreError::Unavailable(
                "Mihomo proxy list was missing".into(),
            ));
        };
        let mut groups = Vec::new();
        for (name, group) in entries {
            let kind = string_field(group, "type").unwrap_or_default();
            if !group_type(&kind) {
                continue;
            }
            let selected = string_field(group, "now");
            let nodes = group
                .get("all")
                .or_else(|| group.get("proxies"))
                .and_then(Value::as_array)
                .map(|names| {
                    names
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|node_name| {
                            let node_value = entries.get(node_name).unwrap_or(&Value::Null);
                            node_from_value(
                                node_name,
                                node_value,
                                Some(name),
                                selected.as_deref() == Some(node_name),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            groups.push(ProxyGroup {
                name: name.clone(),
                selected,
                nodes,
            });
        }
        groups.sort_by_key(|g| g.name.to_lowercase());
        Ok(groups)
    }

    async fn list_proxies(&self) -> Result<Vec<ProxyNode>, ProxyCoreError> {
        let value = self.json("GET", "/proxies", None).await?;
        let Some(entries) = value.get("proxies").and_then(Value::as_object) else {
            return Err(ProxyCoreError::Unavailable(
                "Mihomo proxy list was missing".into(),
            ));
        };
        let mut nodes = entries
            .iter()
            .filter(|(_, value)| !group_type(&string_field(value, "type").unwrap_or_default()))
            .map(|(name, value)| node_from_value(name, value, None, false))
            .collect::<Vec<_>>();
        nodes.sort_by_key(|node| node.name.to_lowercase());
        Ok(nodes)
    }

    async fn select_proxy(&self, group: &str, proxy: &str) -> Result<(), ProxyCoreError> {
        self.put(
            &format!("/proxies/{}", path_segment(group)),
            serde_json::json!({"name": proxy}),
        )
        .await
    }

    async fn refresh_provider(&self) -> Result<(), ProxyCoreError> {
        let value = self.json("GET", "/providers/proxies", None).await?;
        let Some(providers) = value.get("providers").and_then(Value::as_object) else {
            return Err(ProxyCoreError::NotConfigured(
                "current configuration has no proxy providers".into(),
            ));
        };
        if providers.is_empty() {
            return Err(ProxyCoreError::NotConfigured(
                "current configuration has no proxy providers".into(),
            ));
        }
        for name in providers.keys() {
            self.request(
                "PUT",
                &format!("/providers/proxies/{}", path_segment(name)),
                None,
            )
            .await?;
        }
        Ok(())
    }

    async fn get_external_controller_status(
        &self,
    ) -> Result<ExternalControllerStatus, ProxyCoreError> {
        self.json("GET", "/version", None).await?;
        Ok(ExternalControllerStatus {
            connected: true,
            endpoint: self
                .endpoints
                .first()
                .map(Endpoint::label)
                .unwrap_or_else(|| "controller".into()),
        })
    }
}

fn default_effective_config_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(DEFAULT_EFFECTIVE_CONFIG))
}

fn default_profiles_manifest_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home).join(
            "Library/Application Support/io.github.clash-verge-rev.clash-verge-rev/profiles.yaml",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tokio::io::{duplex, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn wire_response(raw: &'static [u8]) -> Result<(u16, Vec<u8>), String> {
        let (mut server, mut client) = duplex(4096);
        tokio::spawn(async move {
            server.write_all(raw).await.unwrap();
            server.shutdown().await.unwrap();
        });
        read_response(&mut client).await
    }

    #[test]
    fn rejects_non_loopback_controller() {
        let addr: SocketAddr = "192.168.1.2:9097".parse().unwrap();
        assert!(matches!(
            MacMihomoProxyCore::with_loopback_controller(addr),
            Err(ProxyCoreError::SecurityDenied(_))
        ));
    }

    #[test]
    fn encodes_path_segments_without_leaking_raw_names() {
        assert_eq!(path_segment("AI/VPS"), "AI%2FVPS");
    }

    #[test]
    fn reads_only_profile_name_from_effective_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("clash-verge.yaml");
        fs::write(
            &path,
            "profile:\n  name: V2Box AI Split\nsecret: never-returned\n",
        )
        .unwrap();
        let core = MacMihomoProxyCore::with_unix_socket(path.clone())
            .with_effective_config_path(Some(path));
        assert_eq!(core.effective_profile().as_deref(), Some("V2Box AI Split"));
    }

    #[test]
    fn maps_current_uid_to_profile_name_without_reading_profile_values() {
        let dir = tempfile::tempdir().unwrap();
        let effective = dir.path().join("clash-verge.yaml");
        let manifest = dir.path().join("profiles.yaml");
        fs::write(&effective, "profile:\n  store-selected: true\n").unwrap();
        fs::write(
            &manifest,
            "current: uid-1\nitems:\n  - uid: uid-1\n    name: V2Box AI Split\n    file: profiles/uid-1.yaml\n",
        )
        .unwrap();
        let core = MacMihomoProxyCore::with_unix_socket(effective.clone())
            .with_effective_config_path(Some(effective))
            .with_profiles_manifest_path(Some(manifest));
        assert_eq!(core.effective_profile().as_deref(), Some("V2Box AI Split"));
    }

    #[tokio::test]
    async fn accepts_204_without_content_length() {
        assert_eq!(
            wire_response(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n")
                .await
                .unwrap(),
            (204, Vec::new())
        );
    }

    #[tokio::test]
    async fn accepts_zero_content_length() {
        assert_eq!(
            wire_response(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n")
                .await
                .unwrap(),
            (204, Vec::new())
        );
    }

    #[tokio::test]
    async fn decodes_chunked_response() {
        let response = wire_response(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\nWiki\r\n5\r\npedia\r\n0\r\n\r\n",
        )
        .await
        .unwrap();
        assert_eq!(response, (200, b"Wikipedia".to_vec()));
    }

    #[tokio::test]
    async fn rejects_truncated_content_length_body() {
        let error = wire_response(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nno")
            .await
            .unwrap_err();
        assert!(error.contains("truncated"));
    }

    #[tokio::test]
    async fn mode_update_uses_patch_and_accepts_204() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = vec![0_u8; 1024];
            let n = stream.read(&mut request).await.unwrap();
            let request = String::from_utf8_lossy(&request[..n]);
            assert!(request.starts_with("PATCH /configs HTTP/1.1"));
            assert!(request.contains("\"mode\":\"global\""));
            stream
                .write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n")
                .await
                .unwrap();
        });
        MacMihomoProxyCore::with_loopback_controller(address)
            .unwrap()
            .set_mode(ProxyMode::Global)
            .await
            .unwrap();
        server.await.unwrap();
    }
}
