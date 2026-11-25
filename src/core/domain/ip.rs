use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalIp {
    pub raw: String,
    pub is_ipv6: bool,
}

impl CanonicalIp {
    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Handle IPv4-mapped IPv6: ::ffff:192.0.2.1
        if trimmed.to_ascii_lowercase().starts_with("::ffff:") {
            let ipv4_part = &trimmed[7..];
            return Self::parse_ipv4(ipv4_part);
        }

        if trimmed.contains(':') {
            // IPv6
            let lower = trimmed.to_ascii_lowercase();
            Some(Self {
                raw: lower,
                is_ipv6: true,
            })
        } else if trimmed.contains('.') {
            Self::parse_ipv4(trimmed)
        } else {
            None
        }
    }

    fn parse_ipv4(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 4 {
            return None;
        }
        let mut canonical_parts = Vec::with_capacity(4);
        for p in parts {
            let num: u8 = p.trim().parse().ok()?;
            canonical_parts.push(num.to_string());
        }
        Some(Self {
            raw: canonical_parts.join("."),
            is_ipv6: false,
        })
    }

    pub fn mask_privacy(&self) -> String {
        if self.is_ipv6 {
            // Mask last 64 bits of IPv6
            let parts: Vec<&str> = self.raw.split(':').collect();
            if parts.len() > 4 {
                let keep = parts[..4].join(":");
                format!("{}:xxxx:xxxx:xxxx:xxxx", keep)
            } else {
                format!("{}:xxxx", self.raw)
            }
        } else {
            // Mask last octet of IPv4
            let parts: Vec<&str> = self.raw.split('.').collect();
            if parts.len() == 4 {
                format!("{}.{}.{}.xxx", parts[0], parts[1], parts[2])
            } else {
                format!("{}.xxx", self.raw)
            }
        }
    }
}

pub struct TrustedProxyPolicy {
    pub trusted_proxies: Vec<String>,
}

impl TrustedProxyPolicy {
    pub fn new(trusted: Vec<String>) -> Self {
        Self {
            trusted_proxies: trusted
                .into_iter()
                .map(|s| s.trim().to_ascii_lowercase())
                .collect(),
        }
    }

    /// Evaluates direct remote IP vs forwarding header with spoofing defense (RSK-09).
    pub fn resolve_client_ip(
        &self,
        direct_remote_ip: &str,
        forwarded_for_header: Option<&str>,
    ) -> (CanonicalIp, bool) {
        let direct_canonical =
            CanonicalIp::parse(direct_remote_ip).unwrap_or_else(|| CanonicalIp {
                raw: direct_remote_ip.to_string(),
                is_ipv6: direct_remote_ip.contains(':'),
            });

        // Only evaluate X-Forwarded-For if direct_remote_ip is explicitly in trusted_proxies
        let is_trusted = self
            .trusted_proxies
            .iter()
            .any(|t| t == &direct_canonical.raw.to_ascii_lowercase());

        if is_trusted {
            if let Some(header) = forwarded_for_header {
                let first_hop = header.split(',').next().unwrap_or("").trim();
                if let Some(client_ip) = CanonicalIp::parse(first_hop) {
                    return (client_ip, true);
                }
            }
        }

        (direct_canonical, false)
    }
}

#[derive(Debug, Clone, Default)]
pub struct IpStat {
    pub request_count: u64,
    pub bytes_sum: u64,
    pub first_seen_nanos: i64,
    pub last_seen_nanos: i64,
}

pub struct IpAnalytics {
    stats: HashMap<CanonicalIp, IpStat>,
}

impl Default for IpAnalytics {
    fn default() -> Self {
        Self::new()
    }
}

impl IpAnalytics {
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
        }
    }

    pub fn record(&mut self, ip: CanonicalIp, bytes: u64, ts_nanos: i64) {
        let entry = self.stats.entry(ip).or_insert_with(|| IpStat {
            request_count: 0,
            bytes_sum: 0,
            first_seen_nanos: ts_nanos,
            last_seen_nanos: ts_nanos,
        });

        entry.request_count += 1;
        entry.bytes_sum = entry.bytes_sum.saturating_add(bytes);
        entry.first_seen_nanos = entry.first_seen_nanos.min(ts_nanos);
        entry.last_seen_nanos = entry.last_seen_nanos.max(ts_nanos);
    }

    pub fn top_ips(&self, limit: usize) -> Vec<(CanonicalIp, IpStat)> {
        let mut list: Vec<(CanonicalIp, IpStat)> = self
            .stats
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        list.sort_by_key(|b| std::cmp::Reverse(b.1.request_count));
        list.truncate(limit);
        list
    }
}
