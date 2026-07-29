//! Convention `proxy.yaml` recipe → Mihomo Profile compiler.
//!
//! Agents and users fill only required VPS fields; Luma expands presets into a full Profile
//! with a single `PROXY` select group and `MATCH,PROXY`. Credentials must never appear in
//! error messages — only field names.

use luma_application::ProfileStoreError;
use serde::Deserialize;
use serde_yaml_ng::{Mapping, Value};
use std::collections::HashSet;
use std::net::IpAddr;

use super::MAX_PROFILE_BYTES;

/// Stable Luma-owned Profile ID for the convention recipe. Repeat sync updates this entry.
pub(super) const CONVENTION_PROFILE_ID: &str = "p-c0ffee0000000000000001";

const MAX_NODES: usize = 2000;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProxyRecipe {
    kind: String,
    version: u32,
    name: String,
    nodes: Vec<RecipeNode>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "preset", deny_unknown_fields)]
enum RecipeNode {
    #[serde(rename = "ss")]
    Ss {
        name: String,
        server: String,
        port: u16,
        cipher: String,
        password: String,
    },
    #[serde(rename = "trojan-tls")]
    TrojanTls {
        name: String,
        server: String,
        port: u16,
        password: String,
        #[serde(default)]
        sni: Option<String>,
    },
    #[serde(rename = "vless-tls")]
    VlessTls {
        name: String,
        server: String,
        port: u16,
        uuid: String,
        #[serde(default)]
        sni: Option<String>,
    },
    #[serde(rename = "vless-reality")]
    VlessReality {
        name: String,
        server: String,
        port: u16,
        uuid: String,
        #[serde(default)]
        sni: Option<String>,
        #[serde(rename = "public-key")]
        public_key: String,
        #[serde(rename = "short-id")]
        short_id: String,
    },
    #[serde(rename = "vless-ws-tls")]
    VlessWsTls {
        name: String,
        server: String,
        port: u16,
        uuid: String,
        #[serde(default)]
        sni: Option<String>,
        host: String,
        path: String,
    },
    #[serde(rename = "vless-grpc-tls")]
    VlessGrpcTls {
        name: String,
        server: String,
        port: u16,
        uuid: String,
        #[serde(default)]
        sni: Option<String>,
        #[serde(rename = "service-name")]
        service_name: String,
    },
    #[serde(rename = "hysteria2")]
    Hysteria2 {
        name: String,
        server: String,
        port: u16,
        password: String,
        #[serde(default)]
        sni: Option<String>,
        #[serde(default)]
        obfs: Option<String>,
        #[serde(default, rename = "obfs-password")]
        obfs_password: Option<String>,
        /// Port hopping range (Mihomo `ports`), e.g. `"443-8443"`.
        #[serde(default)]
        ports: Option<String>,
    },
    #[serde(rename = "tuic-v5")]
    TuicV5 {
        name: String,
        server: String,
        port: u16,
        uuid: String,
        password: String,
        #[serde(default)]
        sni: Option<String>,
    },
}

impl RecipeNode {
    fn name(&self) -> &str {
        match self {
            Self::Ss { name, .. }
            | Self::TrojanTls { name, .. }
            | Self::VlessTls { name, .. }
            | Self::VlessReality { name, .. }
            | Self::VlessWsTls { name, .. }
            | Self::VlessGrpcTls { name, .. }
            | Self::Hysteria2 { name, .. }
            | Self::TuicV5 { name, .. } => name,
        }
    }
}

fn invalid(field: &str, message: &str) -> ProfileStoreError {
    ProfileStoreError::InvalidInput {
        field: field.into(),
        message: message.into(),
    }
}

fn is_ip_literal(server: &str) -> bool {
    server.parse::<IpAddr>().is_ok()
}

fn resolve_sni(server: &str, sni: Option<&str>, field: &str) -> Result<String, ProfileStoreError> {
    if let Some(sni) = sni.map(str::trim).filter(|value| !value.is_empty()) {
        return Ok(sni.to_string());
    }
    if is_ip_literal(server) {
        return Err(invalid(
            field,
            "sni is required when server is an IP address",
        ));
    }
    let server = server.trim();
    if server.is_empty() {
        return Err(invalid("server", "server must not be empty"));
    }
    Ok(server.to_string())
}

fn require_non_empty(field: &str, value: &str) -> Result<(), ProfileStoreError> {
    if value.trim().is_empty() {
        return Err(invalid(field, &format!("{field} must not be empty")));
    }
    Ok(())
}

fn require_port(port: u16) -> Result<(), ProfileStoreError> {
    if port == 0 {
        return Err(invalid("port", "port must be 1..=65535"));
    }
    Ok(())
}

fn require_uuid(uuid: &str) -> Result<(), ProfileStoreError> {
    let uuid = uuid.trim();
    // 8-4-4-4-12 hex, case-insensitive. Do not echo the value.
    let ok = uuid.len() == 36
        && uuid.as_bytes().get(8) == Some(&b'-')
        && uuid.as_bytes().get(13) == Some(&b'-')
        && uuid.as_bytes().get(18) == Some(&b'-')
        && uuid.as_bytes().get(23) == Some(&b'-')
        && uuid
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 8 | 13 | 18 | 23) || byte.is_ascii_hexdigit());
    if !ok {
        return Err(invalid("uuid", "uuid must be a valid UUID"));
    }
    Ok(())
}

fn require_reality_public_key(value: &str) -> Result<(), ProfileStoreError> {
    let value = value.trim();
    // REALITY public keys are typically 43-char URL-safe base64 (32 bytes).
    let ok = (32..=64).contains(&value.len())
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'-' | b'_' | b'=')
        });
    if !ok {
        return Err(invalid(
            "public-key",
            "public-key format is not a reasonable REALITY key",
        ));
    }
    Ok(())
}

fn require_reality_short_id(value: &str) -> Result<(), ProfileStoreError> {
    let value = value.trim();
    let ok = (1..=16).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_hexdigit());
    if !ok {
        return Err(invalid(
            "short-id",
            "short-id must be 1..=16 hex characters",
        ));
    }
    Ok(())
}

fn insert_string(map: &mut Mapping, key: &str, value: impl Into<String>) {
    map.insert(Value::String(key.into()), Value::String(value.into()));
}

fn insert_bool(map: &mut Mapping, key: &str, value: bool) {
    map.insert(Value::String(key.into()), Value::Bool(value));
}

fn insert_u16(map: &mut Mapping, key: &str, value: u16) {
    map.insert(Value::String(key.into()), Value::Number(value.into()));
}

fn proxy_base(kind: &str, name: &str) -> Mapping {
    let mut map = Mapping::new();
    insert_string(&mut map, "name", name);
    insert_string(&mut map, "type", kind);
    map
}

fn compile_ss(
    name: &str,
    server: &str,
    port: u16,
    cipher: &str,
    password: &str,
) -> Result<Value, ProfileStoreError> {
    require_non_empty("name", name)?;
    require_non_empty("server", server)?;
    require_port(port)?;
    require_non_empty("cipher", cipher)?;
    require_non_empty("password", password)?;
    let mut map = proxy_base("ss", name.trim());
    insert_string(&mut map, "server", server.trim());
    insert_u16(&mut map, "port", port);
    insert_string(&mut map, "cipher", cipher.trim());
    insert_string(&mut map, "password", password.trim());
    insert_bool(&mut map, "udp", true);
    Ok(Value::Mapping(map))
}

fn compile_trojan_tls(
    name: &str,
    server: &str,
    port: u16,
    password: &str,
    sni: Option<&str>,
) -> Result<Value, ProfileStoreError> {
    require_non_empty("name", name)?;
    require_non_empty("server", server)?;
    require_port(port)?;
    require_non_empty("password", password)?;
    let sni = resolve_sni(server, sni, "sni")?;
    let mut map = proxy_base("trojan", name.trim());
    insert_string(&mut map, "server", server.trim());
    insert_u16(&mut map, "port", port);
    insert_string(&mut map, "password", password.trim());
    insert_string(&mut map, "sni", sni);
    insert_bool(&mut map, "udp", true);
    insert_bool(&mut map, "skip-cert-verify", false);
    insert_string(&mut map, "network", "tcp");
    Ok(Value::Mapping(map))
}

fn compile_vless_tls(
    name: &str,
    server: &str,
    port: u16,
    uuid: &str,
    sni: Option<&str>,
) -> Result<Value, ProfileStoreError> {
    require_non_empty("name", name)?;
    require_non_empty("server", server)?;
    require_port(port)?;
    require_uuid(uuid)?;
    let sni = resolve_sni(server, sni, "sni")?;
    let mut map = proxy_base("vless", name.trim());
    insert_string(&mut map, "server", server.trim());
    insert_u16(&mut map, "port", port);
    insert_string(&mut map, "uuid", uuid.trim());
    insert_bool(&mut map, "udp", true);
    insert_bool(&mut map, "tls", true);
    insert_string(&mut map, "servername", sni);
    insert_string(&mut map, "encryption", "");
    insert_string(&mut map, "network", "tcp");
    insert_bool(&mut map, "skip-cert-verify", false);
    Ok(Value::Mapping(map))
}

fn compile_vless_reality(
    name: &str,
    server: &str,
    port: u16,
    uuid: &str,
    sni: Option<&str>,
    public_key: &str,
    short_id: &str,
) -> Result<Value, ProfileStoreError> {
    require_non_empty("name", name)?;
    require_non_empty("server", server)?;
    require_port(port)?;
    require_uuid(uuid)?;
    require_reality_public_key(public_key)?;
    require_reality_short_id(short_id)?;
    let sni = resolve_sni(server, sni, "sni")?;
    let mut map = proxy_base("vless", name.trim());
    insert_string(&mut map, "server", server.trim());
    insert_u16(&mut map, "port", port);
    insert_string(&mut map, "uuid", uuid.trim());
    insert_bool(&mut map, "udp", true);
    insert_bool(&mut map, "tls", true);
    insert_string(&mut map, "servername", sni);
    insert_string(&mut map, "client-fingerprint", "chrome");
    insert_string(&mut map, "flow", "xtls-rprx-vision");
    insert_string(&mut map, "encryption", "");
    insert_string(&mut map, "network", "tcp");
    let mut reality = Mapping::new();
    insert_string(&mut reality, "public-key", public_key.trim());
    insert_string(&mut reality, "short-id", short_id.trim());
    map.insert(
        Value::String("reality-opts".into()),
        Value::Mapping(reality),
    );
    Ok(Value::Mapping(map))
}

fn compile_vless_ws_tls(
    name: &str,
    server: &str,
    port: u16,
    uuid: &str,
    sni: Option<&str>,
    host: &str,
    path: &str,
) -> Result<Value, ProfileStoreError> {
    require_non_empty("name", name)?;
    require_non_empty("server", server)?;
    require_port(port)?;
    require_uuid(uuid)?;
    require_non_empty("host", host)?;
    require_non_empty("path", path)?;
    let sni = resolve_sni(server, sni, "sni")?;
    let mut map = proxy_base("vless", name.trim());
    insert_string(&mut map, "server", server.trim());
    insert_u16(&mut map, "port", port);
    insert_string(&mut map, "uuid", uuid.trim());
    insert_bool(&mut map, "udp", true);
    insert_bool(&mut map, "tls", true);
    insert_string(&mut map, "servername", sni);
    insert_string(&mut map, "encryption", "");
    insert_string(&mut map, "network", "ws");
    insert_bool(&mut map, "skip-cert-verify", false);
    let mut headers = Mapping::new();
    insert_string(&mut headers, "Host", host.trim());
    let mut ws = Mapping::new();
    insert_string(&mut ws, "path", path.trim());
    ws.insert(Value::String("headers".into()), Value::Mapping(headers));
    map.insert(Value::String("ws-opts".into()), Value::Mapping(ws));
    Ok(Value::Mapping(map))
}

fn compile_vless_grpc_tls(
    name: &str,
    server: &str,
    port: u16,
    uuid: &str,
    sni: Option<&str>,
    service_name: &str,
) -> Result<Value, ProfileStoreError> {
    require_non_empty("name", name)?;
    require_non_empty("server", server)?;
    require_port(port)?;
    require_uuid(uuid)?;
    require_non_empty("service-name", service_name)?;
    let sni = resolve_sni(server, sni, "sni")?;
    let mut map = proxy_base("vless", name.trim());
    insert_string(&mut map, "server", server.trim());
    insert_u16(&mut map, "port", port);
    insert_string(&mut map, "uuid", uuid.trim());
    insert_bool(&mut map, "udp", true);
    insert_bool(&mut map, "tls", true);
    insert_string(&mut map, "servername", sni);
    insert_string(&mut map, "encryption", "");
    insert_string(&mut map, "network", "grpc");
    insert_bool(&mut map, "skip-cert-verify", false);
    let mut grpc = Mapping::new();
    insert_string(&mut grpc, "grpc-service-name", service_name.trim());
    map.insert(Value::String("grpc-opts".into()), Value::Mapping(grpc));
    Ok(Value::Mapping(map))
}

#[allow(clippy::too_many_arguments)]
fn compile_hysteria2(
    name: &str,
    server: &str,
    port: u16,
    password: &str,
    sni: Option<&str>,
    obfs: Option<&str>,
    obfs_password: Option<&str>,
    ports: Option<&str>,
) -> Result<Value, ProfileStoreError> {
    require_non_empty("name", name)?;
    require_non_empty("server", server)?;
    require_port(port)?;
    require_non_empty("password", password)?;
    let sni = resolve_sni(server, sni, "sni")?;
    if obfs.is_some() != obfs_password.is_some() {
        return Err(invalid(
            "obfs",
            "obfs and obfs-password must both be set or both omitted",
        ));
    }
    let mut map = proxy_base("hysteria2", name.trim());
    insert_string(&mut map, "server", server.trim());
    insert_u16(&mut map, "port", port);
    insert_string(&mut map, "password", password.trim());
    insert_string(&mut map, "sni", sni);
    insert_bool(&mut map, "skip-cert-verify", false);
    if let Some(obfs) = obfs {
        require_non_empty("obfs", obfs)?;
        require_non_empty("obfs-password", obfs_password.unwrap_or(""))?;
        insert_string(&mut map, "obfs", obfs.trim());
        insert_string(&mut map, "obfs-password", obfs_password.unwrap().trim());
    }
    if let Some(ports) = ports {
        require_non_empty("ports", ports)?;
        insert_string(&mut map, "ports", ports.trim());
    }
    Ok(Value::Mapping(map))
}

fn compile_tuic_v5(
    name: &str,
    server: &str,
    port: u16,
    uuid: &str,
    password: &str,
    sni: Option<&str>,
) -> Result<Value, ProfileStoreError> {
    require_non_empty("name", name)?;
    require_non_empty("server", server)?;
    require_port(port)?;
    require_uuid(uuid)?;
    require_non_empty("password", password)?;
    let sni = resolve_sni(server, sni, "sni")?;
    let mut map = proxy_base("tuic", name.trim());
    insert_string(&mut map, "server", server.trim());
    insert_u16(&mut map, "port", port);
    insert_string(&mut map, "uuid", uuid.trim());
    insert_string(&mut map, "password", password.trim());
    insert_string(&mut map, "sni", sni);
    map.insert(Value::String("version".into()), Value::Number(5.into()));
    insert_string(&mut map, "udp-relay-mode", "native");
    insert_string(&mut map, "congestion-controller", "bbr");
    map.insert(
        Value::String("alpn".into()),
        Value::Sequence(vec![Value::String("h3".into())]),
    );
    insert_bool(&mut map, "skip-cert-verify", false);
    Ok(Value::Mapping(map))
}

fn compile_node(node: &RecipeNode) -> Result<Value, ProfileStoreError> {
    match node {
        RecipeNode::Ss {
            name,
            server,
            port,
            cipher,
            password,
        } => compile_ss(name, server, *port, cipher, password),
        RecipeNode::TrojanTls {
            name,
            server,
            port,
            password,
            sni,
        } => compile_trojan_tls(name, server, *port, password, sni.as_deref()),
        RecipeNode::VlessTls {
            name,
            server,
            port,
            uuid,
            sni,
        } => compile_vless_tls(name, server, *port, uuid, sni.as_deref()),
        RecipeNode::VlessReality {
            name,
            server,
            port,
            uuid,
            sni,
            public_key,
            short_id,
        } => compile_vless_reality(
            name,
            server,
            *port,
            uuid,
            sni.as_deref(),
            public_key,
            short_id,
        ),
        RecipeNode::VlessWsTls {
            name,
            server,
            port,
            uuid,
            sni,
            host,
            path,
        } => compile_vless_ws_tls(name, server, *port, uuid, sni.as_deref(), host, path),
        RecipeNode::VlessGrpcTls {
            name,
            server,
            port,
            uuid,
            sni,
            service_name,
        } => compile_vless_grpc_tls(name, server, *port, uuid, sni.as_deref(), service_name),
        RecipeNode::Hysteria2 {
            name,
            server,
            port,
            password,
            sni,
            obfs,
            obfs_password,
            ports,
        } => compile_hysteria2(
            name,
            server,
            *port,
            password,
            sni.as_deref(),
            obfs.as_deref(),
            obfs_password.as_deref(),
            ports.as_deref(),
        ),
        RecipeNode::TuicV5 {
            name,
            server,
            port,
            uuid,
            password,
            sni,
        } => compile_tuic_v5(name, server, *port, uuid, password, sni.as_deref()),
    }
}

fn validate_recipe(recipe: &ProxyRecipe) -> Result<(), ProfileStoreError> {
    if recipe.kind != "luma-proxy" {
        return Err(invalid("kind", "kind must be luma-proxy"));
    }
    if recipe.version != 1 {
        return Err(invalid("version", "version must be 1"));
    }
    require_non_empty("name", &recipe.name)?;
    if recipe.name.trim().len() > 120 || recipe.name.chars().any(|c| c.is_control()) {
        return Err(invalid("name", "Profile name is invalid"));
    }
    if recipe.nodes.is_empty() {
        return Err(invalid("nodes", "nodes must not be empty"));
    }
    if recipe.nodes.len() > MAX_NODES {
        return Err(ProfileStoreError::SecurityDenied(
            "recipe contains too many nodes".into(),
        ));
    }
    let mut seen = HashSet::new();
    for node in &recipe.nodes {
        let name = node.name().trim();
        require_non_empty("name", name)?;
        if !seen.insert(name.to_string()) {
            return Err(invalid("name", "node names must be unique"));
        }
    }
    Ok(())
}

fn build_profile(name: &str, proxies: Vec<Value>) -> Result<Vec<u8>, ProfileStoreError> {
    let names: Vec<Value> = proxies
        .iter()
        .filter_map(|proxy| {
            proxy
                .as_mapping()
                .and_then(|map| map.get(Value::String("name".into())))
                .cloned()
        })
        .collect();
    let mut group = Mapping::new();
    insert_string(&mut group, "name", "PROXY");
    insert_string(&mut group, "type", "select");
    group.insert(Value::String("proxies".into()), Value::Sequence(names));

    let mut root = Mapping::new();
    insert_string(&mut root, "name", name.trim());
    root.insert(Value::String("proxies".into()), Value::Sequence(proxies));
    root.insert(
        Value::String("proxy-groups".into()),
        Value::Sequence(vec![Value::Mapping(group)]),
    );
    root.insert(
        Value::String("rules".into()),
        Value::Sequence(vec![Value::String("MATCH,PROXY".into())]),
    );
    let raw = serde_yaml_ng::to_string(&Value::Mapping(root)).map_err(|_| {
        ProfileStoreError::Unavailable("recipe could not be serialized to YAML".into())
    })?;
    if raw.len() as u64 > MAX_PROFILE_BYTES {
        return Err(ProfileStoreError::SecurityDenied(
            "compiled profile exceeds the size limit".into(),
        ));
    }
    Ok(raw.into_bytes())
}

/// Parse and compile a convention `proxy.yaml` into Clash/Mihomo Profile YAML bytes.
pub(super) fn compile_convention_recipe(bytes: &[u8]) -> Result<Vec<u8>, ProfileStoreError> {
    if bytes.len() as u64 > MAX_PROFILE_BYTES {
        return Err(ProfileStoreError::SecurityDenied(
            "recipe exceeds the size limit".into(),
        ));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("yaml", "recipe is not UTF-8"))?;
    let recipe: ProxyRecipe = serde_yaml_ng::from_str(text).map_err(|error| {
        // Serde unknown-field errors mention the field name — keep that, never echo values.
        let message = error.to_string();
        if message.contains("unknown field") {
            invalid("yaml", "recipe contains an unknown field")
        } else {
            invalid("yaml", "recipe is not valid luma-proxy YAML")
        }
    })?;
    validate_recipe(&recipe)?;
    let mut proxies = Vec::with_capacity(recipe.nodes.len());
    for node in &recipe.nodes {
        proxies.push(compile_node(node)?);
    }
    build_profile(&recipe.name, proxies)
}

/// Compile a single supported node URI into a Mihomo proxy mapping (Phase 3 URI→preset reuse).
pub(super) fn compile_proxy_from_supported_uri(uri: &str) -> Result<Value, ProfileStoreError> {
    if let Some(rest) = uri.strip_prefix("ss://") {
        return compile_ss_uri(rest);
    }
    if let Some(rest) = uri.strip_prefix("trojan://") {
        return compile_trojan_uri(rest);
    }
    if let Some(rest) = uri.strip_prefix("vless://") {
        return compile_vless_uri(rest);
    }
    Err(invalid(
        "subscription",
        "subscription contains an unsupported node format",
    ))
}

fn split_uri_userinfo(rest: &str) -> Result<(String, String, String, String), ProfileStoreError> {
    let (authority, query_and_name) = rest.split_once('?').unwrap_or((rest, ""));
    let (authority, frag_name) = authority
        .split_once('#')
        .map_or((authority, ""), |(authority, name)| (authority, name));
    let (query, fragment) = query_and_name
        .split_once('#')
        .map_or((query_and_name, ""), |(query, name)| (query, name));
    let (userinfo, hostport) = authority
        .rsplit_once('@')
        .ok_or_else(|| invalid("subscription", "invalid node URI"))?;
    let name = percent_decode(if fragment.is_empty() {
        frag_name
    } else {
        fragment
    });
    Ok((
        percent_decode(userinfo),
        hostport.to_string(),
        query.to_string(),
        name,
    ))
}

fn split_host_port(value: &str) -> Result<(String, u16), ProfileStoreError> {
    let (host, port) = if let Some(value) = value.strip_prefix('[') {
        let (host, port) = value
            .split_once(']')
            .ok_or_else(|| invalid("server", "invalid host"))?;
        let port = port
            .strip_prefix(':')
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| invalid("port", "port must be 1..=65535"))?;
        (host.to_string(), port)
    } else {
        let (host, port) = value
            .rsplit_once(':')
            .ok_or_else(|| invalid("port", "port must be 1..=65535"))?;
        let port = port
            .parse()
            .map_err(|_| invalid("port", "port must be 1..=65535"))?;
        (host.to_string(), port)
    };
    require_port(port)?;
    Ok((host, port))
}

fn query_map(query: &str) -> Result<std::collections::BTreeMap<String, String>, ProfileStoreError> {
    let mut map = std::collections::BTreeMap::new();
    if query.is_empty() {
        return Ok(map);
    }
    for part in query.split('&').filter(|part| !part.is_empty()) {
        let (key, value) = part
            .split_once('=')
            .ok_or_else(|| invalid("subscription", "invalid URI query"))?;
        if map
            .insert(percent_decode(key), percent_decode(value))
            .is_some()
        {
            return Err(invalid("subscription", "duplicate URI query key"));
        }
    }
    Ok(map)
}

fn require_only_keys(
    query: &std::collections::BTreeMap<String, String>,
    allowed: &[&str],
) -> Result<(), ProfileStoreError> {
    for key in query.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(invalid(
                "subscription",
                "URI contains an unsupported query parameter",
            ));
        }
    }
    Ok(())
}

fn percent_decode(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) =
                (hex_digit(bytes[index + 1]), hex_digit(bytes[index + 2]))
            {
                output.push((high * 16 + low) as char);
                index += 3;
                continue;
            }
        }
        output.push(bytes[index] as char);
        index += 1;
    }
    output
}

fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn decode_base64(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = 0_u32;
    let mut bits = 0_u8;
    for byte in bytes
        .iter()
        .copied()
        .filter(|byte| !byte.is_ascii_whitespace())
    {
        if byte == b'=' {
            break;
        }
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        };
        buffer = (buffer << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    (!output.is_empty()).then_some(output)
}

fn compile_ss_uri(rest: &str) -> Result<Value, ProfileStoreError> {
    let (body, name) = rest
        .split_once('#')
        .map_or((rest, "Imported node"), |(body, name)| (body, name));
    let body = body.split('?').next().unwrap_or(body);
    let (userinfo, authority) = if let Some((userinfo, authority)) = body.rsplit_once('@') {
        (percent_decode(userinfo), authority.to_string())
    } else {
        let decoded = decode_base64(body.as_bytes())
            .ok_or_else(|| invalid("subscription", "invalid ss node URI"))?;
        let decoded = String::from_utf8(decoded)
            .map_err(|_| invalid("subscription", "invalid ss node URI"))?;
        let (userinfo, authority) = decoded
            .rsplit_once('@')
            .ok_or_else(|| invalid("subscription", "invalid ss node URI"))?;
        (userinfo.to_string(), authority.to_string())
    };
    let (host, port) = split_host_port(&authority)?;
    let (cipher, password) = userinfo
        .split_once(':')
        .ok_or_else(|| invalid("subscription", "invalid ss node URI"))?;
    let name = percent_decode(name);
    compile_ss(
        if name.is_empty() {
            "Imported node"
        } else {
            &name
        },
        &host,
        port,
        cipher,
        password,
    )
}

fn compile_trojan_uri(rest: &str) -> Result<Value, ProfileStoreError> {
    let (password, hostport, query, name) = split_uri_userinfo(rest)?;
    let query = query_map(&query)?;
    require_only_keys(&query, &["sni", "allowInsecure", "peer"])?;
    if query
        .get("allowInsecure")
        .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
    {
        return Err(invalid(
            "subscription",
            "skip-cert-verify / allowInsecure is not allowed",
        ));
    }
    let (host, port) = split_host_port(&hostport)?;
    let sni = query
        .get("sni")
        .or_else(|| query.get("peer"))
        .map(String::as_str);
    compile_trojan_tls(
        if name.is_empty() {
            "Imported node"
        } else {
            &name
        },
        &host,
        port,
        &password,
        sni,
    )
}

fn compile_vless_uri(rest: &str) -> Result<Value, ProfileStoreError> {
    let (uuid, hostport, query, name) = split_uri_userinfo(rest)?;
    let query = query_map(&query)?;
    let security = query.get("security").map(String::as_str).unwrap_or("none");
    let network = query.get("type").map(String::as_str).unwrap_or("tcp");
    let name = if name.is_empty() {
        "Imported node"
    } else {
        name.as_str()
    };
    let (host, port) = split_host_port(&hostport)?;
    let sni = query.get("sni").map(String::as_str);

    match (security, network) {
        ("reality", "tcp") => {
            require_only_keys(
                &query,
                &[
                    "security",
                    "type",
                    "sni",
                    "pbk",
                    "sid",
                    "fp",
                    "flow",
                    "encryption",
                ],
            )?;
            if query.get("fp").is_some_and(|value| value != "chrome") {
                return Err(invalid(
                    "subscription",
                    "URI contains an unsupported fingerprint",
                ));
            }
            if query
                .get("flow")
                .is_some_and(|value| value != "xtls-rprx-vision")
            {
                return Err(invalid("subscription", "URI contains an unsupported flow"));
            }
            if query
                .get("encryption")
                .is_some_and(|value| !value.is_empty())
            {
                return Err(invalid(
                    "subscription",
                    "URI contains an unsupported encryption",
                ));
            }
            let public_key = query
                .get("pbk")
                .ok_or_else(|| invalid("public-key", "public-key is required"))?;
            let short_id = query
                .get("sid")
                .ok_or_else(|| invalid("short-id", "short-id is required"))?;
            compile_vless_reality(name, &host, port, &uuid, sni, public_key, short_id)
        }
        ("tls", "tcp") => {
            require_only_keys(&query, &["security", "type", "sni", "encryption", "fp"])?;
            if query
                .get("encryption")
                .is_some_and(|value| !value.is_empty())
            {
                return Err(invalid(
                    "subscription",
                    "URI contains an unsupported encryption",
                ));
            }
            compile_vless_tls(name, &host, port, &uuid, sni)
        }
        ("tls", "ws") => {
            require_only_keys(
                &query,
                &["security", "type", "sni", "host", "path", "encryption"],
            )?;
            let host_header = query
                .get("host")
                .ok_or_else(|| invalid("host", "host is required"))?;
            let path = query
                .get("path")
                .ok_or_else(|| invalid("path", "path is required"))?;
            compile_vless_ws_tls(name, &host, port, &uuid, sni, host_header, path)
        }
        ("tls", "grpc") => {
            require_only_keys(
                &query,
                &["security", "type", "sni", "serviceName", "encryption"],
            )?;
            let service_name = query
                .get("serviceName")
                .ok_or_else(|| invalid("service-name", "service-name is required"))?;
            compile_vless_grpc_tls(name, &host, port, &uuid, sni, service_name)
        }
        ("reality", _) => Err(invalid(
            "subscription",
            "REALITY with non-TCP transport is unsupported; use native Mihomo YAML",
        )),
        _ => Err(invalid(
            "subscription",
            "unsupported VLESS URI; use proxy.yaml presets or native Mihomo YAML",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UUID: &str = "00000000-0000-0000-0000-000000000000";
    const PBK: &str = "abcdefghijklmnopqrstuvwxyz0123456789ABCDE";

    fn mapping_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
        value.as_mapping()?.get(Value::String(key.into()))?.as_str()
    }

    #[test]
    fn compiles_vless_reality_with_defaults() {
        let yaml = format!(
            r#"
kind: luma-proxy
version: 1
name: Personal VPS
nodes:
  - name: US VPS
    preset: vless-reality
    server: 203.0.113.10
    port: 443
    uuid: {UUID}
    sni: www.microsoft.com
    public-key: {PBK}
    short-id: ab12cd34
"#
        );
        let compiled = compile_convention_recipe(yaml.as_bytes()).unwrap();
        let value: Value = serde_yaml_ng::from_slice(&compiled).unwrap();
        let proxy = &value.as_mapping().unwrap()["proxies"]
            .as_sequence()
            .unwrap()[0];
        assert_eq!(mapping_string(proxy, "type"), Some("vless"));
        assert_eq!(mapping_string(proxy, "flow"), Some("xtls-rprx-vision"));
        assert_eq!(mapping_string(proxy, "client-fingerprint"), Some("chrome"));
        assert_eq!(mapping_string(proxy, "network"), Some("tcp"));
        let reality = proxy.as_mapping().unwrap()["reality-opts"]
            .as_mapping()
            .unwrap();
        assert_eq!(
            reality
                .get(Value::String("public-key".into()))
                .and_then(Value::as_str),
            Some(PBK)
        );
        let groups = value.as_mapping().unwrap()["proxy-groups"]
            .as_sequence()
            .unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(mapping_string(&groups[0], "name"), Some("PROXY"));
        let proxies = groups[0].as_mapping().unwrap()["proxies"]
            .as_sequence()
            .unwrap();
        assert_eq!(proxies[0].as_str(), Some("US VPS"));
        assert!(!proxies.iter().any(|item| item.as_str() == Some("DIRECT")));
        assert_eq!(
            value.as_mapping().unwrap()["rules"].as_sequence().unwrap()[0].as_str(),
            Some("MATCH,PROXY")
        );
    }

    #[test]
    fn compiles_phase1_and_phase2_presets() {
        let yaml = format!(
            r#"
kind: luma-proxy
version: 1
name: All
nodes:
  - name: SS
    preset: ss
    server: ss.example.com
    port: 8388
    cipher: aes-256-gcm
    password: secret-password
  - name: Trojan
    preset: trojan-tls
    server: trojan.example.com
    port: 443
    password: secret-password
  - name: VLESS TLS
    preset: vless-tls
    server: vless.example.com
    port: 443
    uuid: {UUID}
  - name: VLESS WS
    preset: vless-ws-tls
    server: 198.51.100.10
    port: 443
    uuid: {UUID}
    sni: ws.example.com
    host: ws.example.com
    path: /vless
  - name: VLESS gRPC
    preset: vless-grpc-tls
    server: grpc.example.com
    port: 443
    uuid: {UUID}
    service-name: gun
  - name: HY2
    preset: hysteria2
    server: hy2.example.com
    port: 443
    password: secret-password
    obfs: salamander
    obfs-password: obfs-secret
  - name: TUIC
    preset: tuic-v5
    server: tuic.example.com
    port: 443
    uuid: {UUID}
    password: secret-password
"#
        );
        let compiled = compile_convention_recipe(yaml.as_bytes()).unwrap();
        let text = String::from_utf8(compiled).unwrap();
        assert!(text.contains("type: ss"));
        assert!(text.contains("type: trojan"));
        assert!(text.contains("type: hysteria2"));
        assert!(text.contains("type: tuic"));
        assert!(text.contains("network: ws"));
        assert!(text.contains("network: grpc"));
        assert!(text.contains("version: 5"));
        assert!(!text.contains("skip-cert-verify: true"));
    }

    #[test]
    fn ip_without_sni_fails_without_echoing_server() {
        let yaml = format!(
            r#"
kind: luma-proxy
version: 1
name: Bad
nodes:
  - name: Node
    preset: vless-tls
    server: 203.0.113.10
    port: 443
    uuid: {UUID}
"#
        );
        let error = compile_convention_recipe(yaml.as_bytes()).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("sni"));
        assert!(!message.contains("203.0.113.10"));
        assert!(!message.contains(UUID));
    }

    #[test]
    fn unknown_field_and_duplicate_names_fail() {
        let unknown = r#"
kind: luma-proxy
version: 1
name: Bad
nodes:
  - name: Node
    preset: ss
    server: ss.example.com
    port: 1
    cipher: aes-256-gcm
    password: x
    plugin: v2ray-plugin
"#;
        assert!(matches!(
            compile_convention_recipe(unknown.as_bytes()),
            Err(ProfileStoreError::InvalidInput { field, .. }) if field == "yaml"
        ));

        let dup = r#"
kind: luma-proxy
version: 1
name: Bad
nodes:
  - name: Same
    preset: ss
    server: a.example.com
    port: 1
    cipher: aes-256-gcm
    password: x
  - name: Same
    preset: ss
    server: b.example.com
    port: 1
    cipher: aes-256-gcm
    password: y
"#;
        assert!(matches!(
            compile_convention_recipe(dup.as_bytes()),
            Err(ProfileStoreError::InvalidInput { field, .. }) if field == "name"
        ));
    }

    #[test]
    fn uri_reality_reuses_preset_compiler() {
        let uri = format!(
            "vless://{UUID}@203.0.113.10:443?security=reality&type=tcp&sni=www.microsoft.com&pbk={PBK}&sid=ab12cd34&fp=chrome&flow=xtls-rprx-vision#US"
        );
        let proxy = compile_proxy_from_supported_uri(&uri).unwrap();
        assert_eq!(mapping_string(&proxy, "flow"), Some("xtls-rprx-vision"));
        assert!(proxy
            .as_mapping()
            .unwrap()
            .contains_key(Value::String("reality-opts".into())));
    }

    #[test]
    fn uri_rejects_unknown_query_without_echoing_values() {
        let uri = format!(
            "vless://{UUID}@vless.example.com:443?security=tls&type=tcp&sni=vless.example.com&evil=leak-secret"
        );
        let error = compile_proxy_from_supported_uri(&uri).unwrap_err();
        let message = error.to_string();
        assert!(!message.contains("leak-secret"));
        assert!(!message.contains(UUID));
    }
}
