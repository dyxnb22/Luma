use luma_application::ProfileStoreError;
use serde_yaml_ng::{Mapping, Value};
use std::time::{SystemTime, UNIX_EPOCH};

use super::MAX_PROFILE_BYTES;

const SUPPORTED_PROFILE_ROOT_KEYS: &[&str] = &[
    "name",
    "proxies",
    "proxy-groups",
    "proxy-providers",
    "rule-providers",
    "rules",
    "sub-rules",
];

pub(super) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub(super) fn normalize_subscription_bytes(bytes: Vec<u8>) -> Result<Vec<u8>, ProfileStoreError> {
    normalize_subscription_bytes_inner(bytes, 0)
}

fn normalize_subscription_bytes_inner(
    bytes: Vec<u8>,
    depth: u8,
) -> Result<Vec<u8>, ProfileStoreError> {
    if depth > 1 || bytes.len() as u64 > MAX_PROFILE_BYTES {
        return Err(ProfileStoreError::SecurityDenied(
            "subscription response exceeds the size limit".into(),
        ));
    }
    if let Ok(text) = String::from_utf8(bytes.clone()) {
        if let Ok(value) = serde_yaml_ng::from_str::<Value>(&text) {
            if value.is_mapping() {
                return Ok(text.into_bytes());
            }
        }
        if let Ok(converted) = convert_node_uris(&text) {
            return Ok(converted);
        }
    }
    let decoded = decode_base64(&bytes).ok_or_else(|| ProfileStoreError::InvalidInput {
        field: "subscription".into(),
        message: "subscription is not Clash YAML or a supported node list".into(),
    })?;
    normalize_subscription_bytes_inner(decoded, depth + 1)
}

fn convert_node_uris(text: &str) -> Result<Vec<u8>, ProfileStoreError> {
    let mut proxies = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if line.starts_with('#') {
            continue;
        }
        let proxy = if line.starts_with("vmess://") {
            parse_vmess(line)
        } else if line.starts_with("vless://")
            || line.starts_with("ss://")
            || line.starts_with("trojan://")
        {
            // Reuse the convention preset compiler so REALITY/advanced params are never
            // silently dropped (fail-closed for unsupported shapes).
            super::recipe::compile_proxy_from_supported_uri(line)
        } else {
            return Err(ProfileStoreError::InvalidInput {
                field: "subscription".into(),
                message: "subscription contains an unsupported node format".into(),
            });
        };
        proxies.push(proxy?);
        if proxies.len() > 2000 {
            return Err(ProfileStoreError::SecurityDenied(
                "subscription contains too many nodes".into(),
            ));
        }
    }
    if proxies.is_empty() {
        return Err(ProfileStoreError::InvalidInput {
            field: "subscription".into(),
            message: "subscription contains no supported nodes".into(),
        });
    }
    let mut root = Mapping::new();
    root.insert(Value::String("proxies".into()), Value::Sequence(proxies));
    serde_yaml_ng::to_string(&Value::Mapping(root))
        .map(|value| value.into_bytes())
        .map_err(|_| ProfileStoreError::Unavailable("subscription could not be converted".into()))
}

#[cfg(test)]
fn parse_vless(uri: &str) -> Result<Value, ProfileStoreError> {
    super::recipe::compile_proxy_from_supported_uri(uri)
}

fn parse_vmess(uri: &str) -> Result<Value, ProfileStoreError> {
    let encoded = uri
        .strip_prefix("vmess://")
        .ok_or_else(|| invalid_uri("vmess"))?;
    let decoded = decode_base64(encoded.as_bytes()).ok_or_else(|| invalid_uri("vmess"))?;
    let value: serde_json::Value =
        serde_json::from_slice(&decoded).map_err(|_| invalid_uri("vmess"))?;
    let mut map = proxy_base(
        "vmess",
        value
            .get("ps")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Imported node"),
    );
    insert_string(
        &mut map,
        "server",
        value
            .get("add")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(""),
    );
    insert_u16(
        &mut map,
        "port",
        value
            .get("port")
            .and_then(serde_json::Value::as_u64)
            .or_else(|| {
                value
                    .get("port")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|v| v.parse().ok())
            }),
    )?;
    insert_string(
        &mut map,
        "uuid",
        value
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(""),
    );
    insert_bool(
        &mut map,
        "tls",
        value.get("tls").and_then(serde_json::Value::as_str) == Some("tls"),
    );
    if let Some(network) = value.get("net").and_then(serde_json::Value::as_str) {
        insert_string(&mut map, "network", network);
    }
    Ok(Value::Mapping(map))
}

fn proxy_base(kind: &str, name: impl Into<String>) -> Mapping {
    let mut map = Mapping::new();
    map.insert(Value::String("name".into()), Value::String(name.into()));
    map.insert(Value::String("type".into()), Value::String(kind.into()));
    map
}

fn insert_string(map: &mut Mapping, key: &str, value: impl Into<String>) {
    map.insert(Value::String(key.into()), Value::String(value.into()));
}

fn insert_bool(map: &mut Mapping, key: &str, value: bool) {
    map.insert(Value::String(key.into()), Value::Bool(value));
}

fn insert_u16(map: &mut Mapping, key: &str, value: Option<u64>) -> Result<(), ProfileStoreError> {
    let Some(value) = value
        .and_then(|value| u16::try_from(value).ok())
        .filter(|value| *value != 0)
    else {
        return Err(invalid_uri("port"));
    };
    map.insert(Value::String(key.into()), Value::Number(value.into()));
    Ok(())
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

fn invalid_uri(kind: &str) -> ProfileStoreError {
    ProfileStoreError::InvalidInput {
        field: "subscription".into(),
        message: format!("invalid {kind} node URI"),
    }
}

pub(super) fn validate_profile(value: &Value) -> Result<(), ProfileStoreError> {
    let Some(map) = value.as_mapping() else {
        return Err(ProfileStoreError::InvalidInput {
            field: "yaml".into(),
            message: "profile root must be a mapping".into(),
        });
    };
    let mut has_runtime_content = false;
    for key in map.keys() {
        let Some(key) = key.as_str() else {
            return Err(ProfileStoreError::SecurityDenied(
                "imported Profile contains unsupported root settings".into(),
            ));
        };
        if !SUPPORTED_PROFILE_ROOT_KEYS.contains(&key) {
            // Do not echo an untrusted key. It can itself contain a credential or URL.
            return Err(ProfileStoreError::SecurityDenied(
                "imported Profile contains unsupported root settings".into(),
            ));
        }
        has_runtime_content |= key != "name";
    }
    if !has_runtime_content {
        return Err(ProfileStoreError::InvalidInput {
            field: "yaml".into(),
            message: "profile has no supported proxy or rule content".into(),
        });
    }
    Ok(())
}
pub(super) fn sequence_len(value: Option<&Value>) -> usize {
    value.and_then(Value::as_sequence).map_or(0, Vec::len)
}
pub(super) fn safe_name(value: &str) -> Result<String, ProfileStoreError> {
    let value = value.trim();
    if value.is_empty() || value.len() > 120 || value.chars().any(|c| c.is_control()) {
        return Err(ProfileStoreError::InvalidInput {
            field: "name".into(),
            message: "Profile name is invalid".into(),
        });
    }
    Ok(value.to_string())
}
pub(super) fn valid_id(id: &str) -> bool {
    id.len() == 24 && id.starts_with("p-") && id[2..].chars().all(|c| c.is_ascii_hexdigit())
}
pub(super) fn new_id(raw: &str, name: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    raw.hash(&mut h);
    name.hash(&mut h);
    now().hash(&mut h);
    format!("p-{:022x}", h.finish())
}
#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::ProfileStoreError;
    use serde_yaml_ng::Value;

    #[test]
    fn rejects_controller_and_listener_fields_before_persistence() {
        for key in [
            "external-controller-unix",
            "secret",
            "authentication",
            "external-ui",
            "listeners",
            "bind-address",
        ] {
            let yaml = format!("{key}: forbidden\nproxies: []\n");
            let value: Value = serde_yaml_ng::from_str(&yaml).unwrap();
            assert!(matches!(
                validate_profile(&value),
                Err(ProfileStoreError::SecurityDenied(_))
            ));
        }
    }

    #[test]
    fn vless_reality_uri_is_compiled_not_silently_truncated() {
        let uri = "vless://00000000-0000-0000-0000-000000000000@203.0.113.10:443?security=reality&type=tcp&sni=www.microsoft.com&pbk=abcdefghijklmnopqrstuvwxyz0123456789ABCDE&sid=ab12cd34&fp=chrome&flow=xtls-rprx-vision#US";
        let value = parse_vless(uri).unwrap();
        assert!(value
            .as_mapping()
            .unwrap()
            .contains_key(Value::String("reality-opts".into())));
    }

    #[test]
    fn vless_reality_without_keys_fails_closed() {
        let uri = "vless://00000000-0000-0000-0000-000000000000@203.0.113.10:443?security=reality&type=tcp&sni=www.microsoft.com#US";
        assert!(parse_vless(uri).is_err());
    }
}
