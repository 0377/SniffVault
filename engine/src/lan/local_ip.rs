use std::net::{IpAddr, Ipv4Addr, UdpSocket};

pub fn enumerate_local_ipv4() -> Vec<String> {
    let mut ips = Vec::new();
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                if let IpAddr::V4(v4) = addr.ip() {
                    ips.push(v4.to_string());
                }
            }
        }
    }
    ips.push(Ipv4Addr::LOCALHOST.to_string());
    ips
}

pub fn select_for_peer(peer_host: &str, local_ips: &[&str]) -> String {
    let peer = parse_ipv4(peer_host);

    if let Some(peer_ip) = peer {
        for ip in local_ips {
            if let Some(local_ip) = parse_ipv4(ip) {
                if same_subnet_24(peer_ip, local_ip) {
                    return (*ip).to_string();
                }
            }
        }
    }

    for ip in local_ips {
        if let Some(local_ip) = parse_ipv4(ip) {
            if !local_ip.is_loopback() && !is_link_local(local_ip) {
                return (*ip).to_string();
            }
        }
    }

    local_ips
        .first()
        .map(|ip| (*ip).to_string())
        .unwrap_or_default()
}

fn parse_ipv4(value: &str) -> Option<Ipv4Addr> {
    value.parse().ok()
}

fn same_subnet_24(a: Ipv4Addr, b: Ipv4Addr) -> bool {
    a.octets()[0..3] == b.octets()[0..3]
}

fn is_link_local(addr: Ipv4Addr) -> bool {
    addr.octets()[0] == 169 && addr.octets()[1] == 254
}

#[cfg(test)]
mod tests {
    use super::select_for_peer;

    #[test]
    fn selects_ip_in_same_subnet_as_peer() {
        let ip = select_for_peer("192.168.1.20", &["192.168.1.5", "10.0.0.2"]);
        assert_eq!(ip, "192.168.1.5");
    }
}
