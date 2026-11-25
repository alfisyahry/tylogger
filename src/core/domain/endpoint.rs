use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalizedEndpoint {
    pub method: String,
    pub path: String,
}

impl NormalizedEndpoint {
    pub fn parse(method: &str, raw_url: &str) -> Self {
        let method = method.trim().to_ascii_uppercase();
        // 1. Strip query string
        let path_without_query = if let Some(pos) = raw_url.find('?') {
            &raw_url[..pos]
        } else {
            raw_url
        };

        // 2. Normalize path parameters
        let segments: Vec<&str> = path_without_query.split('/').collect();
        let mut normalized_segments = Vec::with_capacity(segments.len());

        for seg in segments {
            if seg.is_empty() {
                normalized_segments.push("");
                continue;
            }

            // Check numeric ID
            if seg.chars().all(|c| c.is_ascii_digit()) {
                normalized_segments.push("{id}");
                continue;
            }

            // Check UUID format (8-4-4-4-12)
            if seg.len() == 36 && seg.chars().filter(|c| *c == '-').count() == 4 {
                normalized_segments.push("{uuid}");
                continue;
            }

            // Check hex hash (e.g. git commit sha or md5 >= 8 chars)
            if seg.len() >= 8 && seg.chars().all(|c| c.is_ascii_hexdigit()) {
                normalized_segments.push("{hash}");
                continue;
            }

            normalized_segments.push(seg);
        }

        let normalized_path = normalized_segments.join("/");
        Self {
            method,
            path: normalized_path,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct EndpointStat {
    pub request_count: u64,
    pub error_count: u64,
    pub total_duration_nanos: u64,
}

impl EndpointStat {
    pub fn avg_duration_ms(&self) -> f64 {
        if self.request_count == 0 {
            0.0
        } else {
            (self.total_duration_nanos as f64) / (self.request_count as f64) / 1_000_000.0
        }
    }

    pub fn error_rate(&self) -> f64 {
        if self.request_count == 0 {
            0.0
        } else {
            (self.error_count as f64) / (self.request_count as f64)
        }
    }
}

pub struct EndpointAnalytics {
    endpoints: HashMap<NormalizedEndpoint, EndpointStat>,
}

impl Default for EndpointAnalytics {
    fn default() -> Self {
        Self::new()
    }
}

impl EndpointAnalytics {
    pub fn new() -> Self {
        Self {
            endpoints: HashMap::new(),
        }
    }

    pub fn record(
        &mut self,
        method: &str,
        raw_url: &str,
        is_error: bool,
        duration_nanos: Option<u64>,
    ) {
        let ep = NormalizedEndpoint::parse(method, raw_url);
        let stat = self.endpoints.entry(ep).or_default();
        stat.request_count += 1;
        if is_error {
            stat.error_count += 1;
        }
        if let Some(dur) = duration_nanos {
            stat.total_duration_nanos = stat.total_duration_nanos.saturating_add(dur);
        }
    }

    pub fn top_endpoints(&self, limit: usize) -> Vec<(NormalizedEndpoint, EndpointStat)> {
        let mut list: Vec<(NormalizedEndpoint, EndpointStat)> = self
            .endpoints
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        list.sort_by_key(|b| std::cmp::Reverse(b.1.request_count));
        list.truncate(limit);
        list
    }
}
