use std::collections::HashSet;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

use mdns_sd::{
    ResolvedService, ScopedIp, ServiceDaemon, ServiceEvent, ServiceInfo, UnregisterStatus,
};

use crate::error::EngineError;
use crate::lan::LanPeer;

pub const SNIFFVAULT_SERVICE_TYPE: &str = "_sniffvault._tcp.local.";
pub const TXT_KEY_ID: &str = "id";
pub const TXT_KEY_NAME: &str = "name";
pub const TXT_KEY_ROLE: &str = "role";
pub const TXT_ROLE_RECEIVER: &str = "receiver";
pub const MAX_DEVICE_NAME_LEN: usize = 64;

pub struct ReceiverRegistration {
    daemon: ServiceDaemon,
    fullname: String,
}

pub fn register_receiver(
    device_id: &str,
    device_name: &str,
    host: &str,
    port: u16,
) -> Result<ReceiverRegistration, EngineError> {
    if device_id.is_empty() {
        return Err(EngineError::InvalidArg(
            "device_id must not be empty".into(),
        ));
    }
    if host.is_empty() {
        return Err(EngineError::InvalidArg("host must not be empty".into()));
    }

    let daemon = ServiceDaemon::new().map_err(mdns_error)?;
    let display_name = truncate_device_name(device_name);
    let hostname = format!("{host}.local.");
    let properties = [
        (TXT_KEY_ID, device_id),
        (TXT_KEY_NAME, display_name.as_str()),
        (TXT_KEY_ROLE, TXT_ROLE_RECEIVER),
    ];

    let service = ServiceInfo::new(
        SNIFFVAULT_SERVICE_TYPE,
        device_id,
        &hostname,
        host,
        port,
        &properties[..],
    )
    .map_err(mdns_error)?;

    daemon.register(service).map_err(mdns_error)?;
    let fullname = format!("{}.{}", device_id, SNIFFVAULT_SERVICE_TYPE);

    Ok(ReceiverRegistration { daemon, fullname })
}

pub fn unregister(registration: ReceiverRegistration) -> Result<(), EngineError> {
    let status_rx = registration
        .daemon
        .unregister(&registration.fullname)
        .map_err(mdns_error)?;
    match status_rx.recv_timeout(Duration::from_secs(5)) {
        Ok(UnregisterStatus::OK) | Ok(UnregisterStatus::NotFound) => {}
        Err(err) => {
            return Err(EngineError::Message(format!(
                "mDNS unregister timed out: {err}"
            )));
        }
    }
    registration.daemon.shutdown().map_err(mdns_error)?;
    Ok(())
}

pub fn discover_receivers(timeout: Duration) -> Result<Vec<LanPeer>, EngineError> {
    let daemon = ServiceDaemon::new().map_err(mdns_error)?;
    let receiver = daemon.browse(SNIFFVAULT_SERVICE_TYPE).map_err(mdns_error)?;

    let deadline = Instant::now() + timeout;
    let mut peers = Vec::new();
    let mut seen = HashSet::new();

    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let wait = remaining.min(Duration::from_millis(200));
        match receiver.recv_timeout(wait) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                if let Ok(peer) = parse_service_instance(&*info) {
                    if seen.insert(peer.device_id.clone()) {
                        peers.push(peer);
                    }
                }
            }
            Ok(_) => {}
            Err(_) => continue,
        }
    }

    daemon.shutdown().map_err(mdns_error)?;
    Ok(peers)
}

pub fn parse_service_instance(resolved: &ResolvedService) -> Result<LanPeer, EngineError> {
    if !resolved.is_valid() {
        return Err(EngineError::InvalidArg("incomplete mDNS service".into()));
    }

    let role = resolved
        .get_property_val_str(TXT_KEY_ROLE)
        .ok_or_else(|| EngineError::InvalidArg("missing TXT role".into()))?;
    let device_id = resolved
        .get_property_val_str(TXT_KEY_ID)
        .ok_or_else(|| EngineError::InvalidArg("missing TXT id".into()))?;
    let device_name = resolved
        .get_property_val_str(TXT_KEY_NAME)
        .ok_or_else(|| EngineError::InvalidArg("missing TXT name".into()))?;

    parse_receiver_record(
        role,
        device_id,
        device_name,
        resolved.get_addresses(),
        resolved.get_port(),
    )
}

fn parse_receiver_record(
    role: &str,
    device_id: &str,
    device_name: &str,
    addresses: &HashSet<ScopedIp>,
    port: u16,
) -> Result<LanPeer, EngineError> {
    if role != TXT_ROLE_RECEIVER {
        return Err(EngineError::InvalidArg(format!(
            "unsupported mDNS role: {role}"
        )));
    }
    if device_id.is_empty() {
        return Err(EngineError::InvalidArg("empty TXT id".into()));
    }
    if device_name.is_empty() {
        return Err(EngineError::InvalidArg("empty TXT name".into()));
    }

    let host = select_host(addresses)
        .ok_or_else(|| EngineError::InvalidArg("no usable service address".into()))?;

    Ok(LanPeer {
        device_id: device_id.to_string(),
        device_name: device_name.to_string(),
        host,
        port,
        is_trusted: false,
    })
}

fn truncate_device_name(name: &str) -> String {
    if name.len() <= MAX_DEVICE_NAME_LEN {
        return name.to_string();
    }
    name.chars().take(MAX_DEVICE_NAME_LEN).collect()
}

fn select_host(addresses: &HashSet<ScopedIp>) -> Option<String> {
    let mut v4_addrs = Vec::new();
    for scoped in addresses {
        if let ScopedIp::V4(v4) = scoped {
            v4_addrs.push(*v4.addr());
        }
    }

    for addr in &v4_addrs {
        if !addr.is_loopback() && !is_link_local(*addr) {
            return Some(addr.to_string());
        }
    }

    v4_addrs.first().map(|addr| addr.to_string())
}

fn is_link_local(addr: Ipv4Addr) -> bool {
    addr.octets()[0] == 169 && addr.octets()[1] == 254
}

fn mdns_error(err: mdns_sd::Error) -> EngineError {
    EngineError::Message(format!("mDNS error: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdns_sd::{InterfaceId, ScopedIpV4};
    use std::collections::HashSet;
    use std::net::Ipv4Addr;

    fn test_interface() -> InterfaceId {
        InterfaceId {
            name: "lo0".to_string(),
            index: 1,
        }
    }

    fn ipv4(addr: Ipv4Addr) -> ScopedIp {
        ScopedIp::V4(ScopedIpV4::new(addr, test_interface()))
    }

    fn txt_value(s: &str) -> Vec<u8> {
        s.as_bytes().to_vec()
    }

    fn resolved_from_parts(
        id: &str,
        name: &str,
        role: &str,
        addr: Ipv4Addr,
        port: u16,
    ) -> ResolvedService {
        let json = serde_json::json!({
            "ty_domain": SNIFFVAULT_SERVICE_TYPE,
            "fullname": format!("{id}.{SNIFFVAULT_SERVICE_TYPE}"),
            "host": "tv.local.",
            "port": port,
            "addresses": [{"V4": {"addr": addr.to_string(), "interface_ids": []}}],
            "txt_properties": [
                {"key": "id", "value": txt_value(id)},
                {"key": "name", "value": txt_value(name)},
                {"key": "role", "value": txt_value(role)}
            ]
        });
        serde_json::from_value(json).expect("resolved service json")
    }

    #[test]
    fn parse_service_instance_maps_receiver_txt_and_srv() {
        let resolved = resolved_from_parts(
            "tv-uuid",
            "Living Room TV",
            TXT_ROLE_RECEIVER,
            Ipv4Addr::new(192, 168, 1, 10),
            8080,
        );

        let peer = parse_service_instance(&resolved).unwrap();
        assert_eq!(peer.device_id, "tv-uuid");
        assert_eq!(peer.device_name, "Living Room TV");
        assert_eq!(peer.host, "192.168.1.10");
        assert_eq!(peer.port, 8080);
        assert!(!peer.is_trusted);
    }

    #[test]
    fn parse_service_instance_rejects_non_receiver_role() {
        let resolved = resolved_from_parts(
            "tv-uuid",
            "Living Room TV",
            "sender",
            Ipv4Addr::new(192, 168, 1, 10),
            8080,
        );

        let err = parse_service_instance(&resolved).unwrap_err();
        assert!(err.to_string().contains("role"));
    }

    #[test]
    fn parse_service_instance_prefers_non_loopback_ipv4() {
        let peer = parse_receiver_record(
            TXT_ROLE_RECEIVER,
            "tv-uuid",
            "Living Room TV",
            &HashSet::from([
                ipv4(Ipv4Addr::new(127, 0, 0, 1)),
                ipv4(Ipv4Addr::new(192, 168, 1, 20)),
            ]),
            8080,
        )
        .unwrap();
        assert_eq!(peer.host, "192.168.1.20");
    }

    #[test]
    fn truncate_device_name_limits_to_64_chars() {
        let long_name = "n".repeat(80);
        let truncated = truncate_device_name(&long_name);
        assert_eq!(truncated.len(), MAX_DEVICE_NAME_LEN);
        assert_eq!(truncated, "n".repeat(MAX_DEVICE_NAME_LEN));
    }
}
