//! SSRF-resistant address policy for the RPC layer.
//!
//! Before any live request is made to a user-supplied endpoint, the target must
//! survive this policy: literal addresses are classified directly, and hostnames
//! are resolved so every resulting address can be checked. This closes the
//! classic SSRF bypasses (loopback, private ranges, link-local/metadata,
//! IPv4-mapped IPv6, and shared/reserved ranges).
//!
//! Nothing here performs a network request beyond DNS resolution via the
//! standard library.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};

use observatory_core::{ObservatoryError, Result};
use serde::Serialize;

/// Classification of an IP address for SSRF purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddressClass {
    /// Routable public address.
    Public,
    /// Loopback (`127.0.0.0/8`, `::1`).
    Loopback,
    /// RFC 1918 private (`10/8`, `172.16/12`, `192.168/16`).
    Private,
    /// Link-local (`169.254.0.0/16`, `fe80::/10`).
    LinkLocal,
    /// Cloud metadata service (`169.254.169.254`).
    Metadata,
    /// IPv6 unique-local (`fc00::/7`).
    UniqueLocal,
    /// Carrier-grade NAT / shared (`100.64.0.0/10`).
    Shared,
    /// Benchmarking (`198.18.0.0/15`).
    Benchmarking,
    /// Documentation ranges.
    Documentation,
    /// Unspecified (`0.0.0.0`, `::`).
    Unspecified,
    /// Multicast.
    Multicast,
    /// Broadcast (`255.255.255.255`).
    Broadcast,
    /// Reserved (`240.0.0.0/4`).
    Reserved,
}

/// Policy controlling which address classes may be contacted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SsrfPolicy {
    /// Whether loopback addresses are permitted (local development only).
    pub allow_loopback: bool,
}

impl Default for SsrfPolicy {
    fn default() -> Self {
        SsrfPolicy::strict()
    }
}

impl SsrfPolicy {
    /// Block loopback and every non-public class. Suitable for servers.
    #[must_use]
    pub fn strict() -> Self {
        SsrfPolicy {
            allow_loopback: false,
        }
    }

    /// Permit loopback for local development while still blocking everything
    /// else non-public.
    #[must_use]
    pub fn local() -> Self {
        SsrfPolicy {
            allow_loopback: true,
        }
    }

    /// Validate a host and port, returning the resolved addresses.
    ///
    /// A literal IP is classified directly. A hostname is resolved and every
    /// resulting address must be permitted.
    pub fn validate(&self, host: &str, port: u16) -> Result<Vec<IpAddr>> {
        let host = normalize_host(host);
        if host.is_empty() {
            return Err(ObservatoryError::invalid("endpoint host is empty"));
        }
        if let Ok(ip) = host.parse::<IpAddr>() {
            self.check(ip)?;
            return Ok(vec![ip]);
        }
        let addresses: Vec<IpAddr> = (host.as_str(), port)
            .to_socket_addrs()
            .map_err(|error| {
                ObservatoryError::invalid(format!("cannot resolve `{host}`: {error}"))
            })?
            .map(|addr| addr.ip())
            .collect();
        if addresses.is_empty() {
            return Err(ObservatoryError::invalid(format!(
                "no addresses resolved for `{host}`"
            )));
        }
        for address in &addresses {
            self.check(*address)?;
        }
        Ok(addresses)
    }

    /// Check a single address against the policy.
    pub fn check(&self, ip: IpAddr) -> Result<()> {
        let class = classify(ip);
        if class == AddressClass::Loopback && self.allow_loopback {
            return Ok(());
        }
        if class == AddressClass::Public {
            return Ok(());
        }
        Err(ObservatoryError::invalid(format!(
            "address {ip} is blocked by the SSRF policy ({class:?})"
        )))
    }
}

/// Classify an address. IPv4-mapped IPv6 addresses are classified as IPv4.
#[must_use]
pub fn classify(ip: IpAddr) -> AddressClass {
    match ip {
        IpAddr::V4(v4) => classify_v4(v4),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return classify_v4(v4);
            }
            classify_v6(v6)
        }
    }
}

fn classify_v4(ip: Ipv4Addr) -> AddressClass {
    let octets = ip.octets();
    if ip.is_loopback() {
        return AddressClass::Loopback;
    }
    if ip.is_unspecified() {
        return AddressClass::Unspecified;
    }
    if ip.is_broadcast() {
        return AddressClass::Broadcast;
    }
    if ip.is_link_local() {
        if octets == [169, 254, 169, 254] {
            return AddressClass::Metadata;
        }
        return AddressClass::LinkLocal;
    }
    if ip.is_private() {
        return AddressClass::Private;
    }
    if ip.is_multicast() {
        return AddressClass::Multicast;
    }
    if ip.is_documentation() {
        return AddressClass::Documentation;
    }
    if octets[0] == 100 && (64..=127).contains(&octets[1]) {
        return AddressClass::Shared;
    }
    if octets[0] == 198 && (octets[1] == 18 || octets[1] == 19) {
        return AddressClass::Benchmarking;
    }
    if octets[0] >= 240 {
        return AddressClass::Reserved;
    }
    AddressClass::Public
}

fn classify_v6(ip: Ipv6Addr) -> AddressClass {
    if ip.is_loopback() {
        return AddressClass::Loopback;
    }
    if ip.is_unspecified() {
        return AddressClass::Unspecified;
    }
    if ip.is_multicast() {
        return AddressClass::Multicast;
    }
    let segments = ip.segments();
    if (segments[0] & 0xfe00) == 0xfc00 {
        return AddressClass::UniqueLocal;
    }
    if (segments[0] & 0xffc0) == 0xfe80 {
        return AddressClass::LinkLocal;
    }
    AddressClass::Public
}

fn normalize_host(host: &str) -> String {
    host.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(value: &str) -> IpAddr {
        value.parse().unwrap()
    }

    #[test]
    fn classifies_ipv4_ranges() {
        assert_eq!(classify(ip("8.8.8.8")), AddressClass::Public);
        assert_eq!(classify(ip("127.0.0.1")), AddressClass::Loopback);
        assert_eq!(classify(ip("10.0.0.1")), AddressClass::Private);
        assert_eq!(classify(ip("172.16.5.4")), AddressClass::Private);
        assert_eq!(classify(ip("192.168.1.1")), AddressClass::Private);
        assert_eq!(classify(ip("169.254.1.1")), AddressClass::LinkLocal);
        assert_eq!(classify(ip("169.254.169.254")), AddressClass::Metadata);
        assert_eq!(classify(ip("100.64.0.1")), AddressClass::Shared);
        assert_eq!(classify(ip("198.18.0.1")), AddressClass::Benchmarking);
        assert_eq!(classify(ip("192.0.2.1")), AddressClass::Documentation);
        assert_eq!(classify(ip("0.0.0.0")), AddressClass::Unspecified);
        assert_eq!(classify(ip("255.255.255.255")), AddressClass::Broadcast);
        assert_eq!(classify(ip("240.0.0.1")), AddressClass::Reserved);
        assert_eq!(classify(ip("224.0.0.1")), AddressClass::Multicast);
    }

    #[test]
    fn classifies_ipv6_ranges() {
        assert_eq!(classify(ip("::1")), AddressClass::Loopback);
        assert_eq!(classify(ip("::")), AddressClass::Unspecified);
        assert_eq!(classify(ip("fc00::1")), AddressClass::UniqueLocal);
        assert_eq!(classify(ip("fd12::1")), AddressClass::UniqueLocal);
        assert_eq!(classify(ip("fe80::1")), AddressClass::LinkLocal);
        assert_eq!(classify(ip("2001:4860:4860::8888")), AddressClass::Public);
    }

    #[test]
    fn ipv4_mapped_ipv6_is_classified_as_ipv4() {
        assert_eq!(classify(ip("::ffff:127.0.0.1")), AddressClass::Loopback);
        assert_eq!(classify(ip("::ffff:10.0.0.1")), AddressClass::Private);
        assert_eq!(classify(ip("::ffff:8.8.8.8")), AddressClass::Public);
    }

    #[test]
    fn strict_blocks_non_public_and_allows_public() {
        let policy = SsrfPolicy::strict();
        assert!(policy.validate("8.8.8.8", 443).is_ok());
        assert!(policy.validate("127.0.0.1", 443).is_err());
        assert!(policy.validate("10.0.0.1", 443).is_err());
        assert!(policy.validate("169.254.169.254", 80).is_err());
        assert!(policy.validate("[::1]", 443).is_err());
        assert!(policy.validate("::ffff:127.0.0.1", 443).is_err());
    }

    #[test]
    fn local_policy_permits_loopback_only() {
        let policy = SsrfPolicy::local();
        assert!(policy.validate("127.0.0.1", 8080).is_ok());
        assert!(policy.validate("::1", 8080).is_ok());
        assert!(policy.validate("10.0.0.1", 8080).is_err());
    }

    #[test]
    fn resolves_localhost_and_blocks_under_strict() {
        // localhost resolves via the hosts file on all supported platforms.
        assert!(SsrfPolicy::local().validate("localhost", 80).is_ok());
        assert!(SsrfPolicy::strict().validate("localhost", 80).is_err());
    }
}
