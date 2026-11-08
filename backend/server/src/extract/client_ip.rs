use std::net::IpAddr;

use axum::http::HeaderMap;

/// One `trusted_proxies` entry: a single address or a CIDR range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrustedNet {
    addr: IpAddr,
    prefix: u8,
}

impl TrustedNet {
    pub fn parse(entry: &str) -> Option<Self> {
        let (addr, prefix) = match entry.trim().split_once('/') {
            Some((addr, prefix)) => (addr.parse::<IpAddr>().ok()?, Some(prefix.parse().ok()?)),
            None => (entry.trim().parse::<IpAddr>().ok()?, None),
        };
        let addr = addr.to_canonical();
        let max = if addr.is_ipv4() { 32 } else { 128 };
        let prefix = prefix.unwrap_or(max);
        (prefix <= max).then_some(Self { addr, prefix })
    }

    pub fn contains(&self, ip: IpAddr) -> bool {
        match (self.addr, ip.to_canonical()) {
            (IpAddr::V4(net), IpAddr::V4(ip)) => {
                let mask = u32::MAX.checked_shl(32 - self.prefix as u32).unwrap_or(0);
                u32::from(net) & mask == u32::from(ip) & mask
            }
            (IpAddr::V6(net), IpAddr::V6(ip)) => {
                let mask = u128::MAX.checked_shl(128 - self.prefix as u32).unwrap_or(0);
                u128::from(net) & mask == u128::from(ip) & mask
            }
            _ => false,
        }
    }
}

/// Parse the configured entries, logging and skipping any that are invalid.
pub fn parse_trusted(entries: &[String]) -> Vec<TrustedNet> {
    entries
        .iter()
        .filter_map(|entry| {
            let parsed = TrustedNet::parse(entry);
            if parsed.is_none() {
                tracing::warn!("ignoring invalid hosting.trusted_proxies entry {entry:?}");
            }
            parsed
        })
        .collect()
}

/// The client address for a request that arrived from `peer`.
pub fn resolve(
    peer: Option<IpAddr>,
    headers: &HeaderMap,
    trusted: &[TrustedNet],
) -> Option<IpAddr> {
    let peer = peer?.to_canonical();
    let is_trusted = |ip: IpAddr| trusted.iter().any(|net| net.contains(ip));
    if !is_trusted(peer) {
        return Some(peer);
    }

    let forwarded: Vec<IpAddr> = headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .filter_map(|ip| ip.trim().parse::<IpAddr>().ok())
        .map(|ip| ip.to_canonical())
        .collect();
    if let Some(client) = forwarded.iter().rev().find(|ip| !is_trusted(**ip)) {
        return Some(*client);
    }
    if let Some(first) = forwarded.first() {
        // Every hop is trusted, so the leftmost is as close to the client as it gets.
        return Some(*first);
    }

    headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<IpAddr>().ok())
        .map(|ip| ip.to_canonical())
        .or(Some(peer))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nets(entries: &[&str]) -> Vec<TrustedNet> {
        entries
            .iter()
            .map(|e| TrustedNet::parse(e).unwrap())
            .collect()
    }

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.append(*name, value.parse().unwrap());
        }
        map
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn parses_addresses_and_ranges() {
        assert!(
            TrustedNet::parse("10.0.0.0/8")
                .unwrap()
                .contains(ip("10.1.2.3"))
        );
        assert!(
            !TrustedNet::parse("10.0.0.0/8")
                .unwrap()
                .contains(ip("11.0.0.1"))
        );
        assert!(TrustedNet::parse("::1").unwrap().contains(ip("::1")));
        assert!(
            TrustedNet::parse("0.0.0.0/0")
                .unwrap()
                .contains(ip("8.8.8.8"))
        );
        assert!(
            TrustedNet::parse("fd00::/8")
                .unwrap()
                .contains(ip("fd12::1"))
        );
        assert!(TrustedNet::parse("10.0.0.0/33").is_none());
        assert!(TrustedNet::parse("not an ip").is_none());
    }

    #[test]
    fn ignores_forwarded_headers_from_untrusted_peers() {
        let h = headers(&[("x-forwarded-for", "1.2.3.4")]);
        assert_eq!(
            resolve(Some(ip("203.0.113.9")), &h, &nets(&["127.0.0.1"])),
            Some(ip("203.0.113.9"))
        );
    }

    #[test]
    fn takes_the_rightmost_untrusted_hop() {
        // The client spoofed 6.6.6.6; the proxy appended the real address.
        let h = headers(&[("x-forwarded-for", "6.6.6.6, 198.51.100.4, 10.0.0.2")]);
        let trusted = nets(&["127.0.0.1", "10.0.0.0/8"]);
        assert_eq!(
            resolve(Some(ip("127.0.0.1")), &h, &trusted),
            Some(ip("198.51.100.4"))
        );
    }

    #[test]
    fn falls_back_to_x_real_ip_then_the_peer() {
        let trusted = nets(&["127.0.0.1"]);
        let h = headers(&[("x-real-ip", "198.51.100.4")]);
        assert_eq!(
            resolve(Some(ip("127.0.0.1")), &h, &trusted),
            Some(ip("198.51.100.4"))
        );
        assert_eq!(
            resolve(Some(ip("127.0.0.1")), &HeaderMap::new(), &trusted),
            Some(ip("127.0.0.1"))
        );
    }

    #[test]
    fn treats_ipv4_mapped_peers_as_ipv4() {
        let h = headers(&[("x-forwarded-for", "198.51.100.4")]);
        assert_eq!(
            resolve(Some(ip("::ffff:127.0.0.1")), &h, &nets(&["127.0.0.1"])),
            Some(ip("198.51.100.4"))
        );
    }
}
