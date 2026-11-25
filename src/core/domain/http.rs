use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct HttpStatusDistribution {
    pub total_requests: u64,
    pub c1xx: u64,
    pub c2xx: u64,
    pub c3xx: u64,
    pub c4xx: u64,
    pub c5xx: u64,
    pub status_counts: HashMap<u16, u64>,
    pub method_counts: HashMap<String, u64>,
    pub total_bytes: u64,
    pub max_bytes: u64,
}

impl HttpStatusDistribution {
    pub fn client_error_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.c4xx as f64) / (self.total_requests as f64)
        }
    }

    pub fn server_error_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.c5xx as f64) / (self.total_requests as f64)
        }
    }

    pub fn avg_bytes(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.total_bytes as f64) / (self.total_requests as f64)
        }
    }
}

pub struct HttpStatusAnalytics {
    distribution: HttpStatusDistribution,
}

impl Default for HttpStatusAnalytics {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpStatusAnalytics {
    pub fn new() -> Self {
        Self {
            distribution: HttpStatusDistribution::default(),
        }
    }

    pub fn record(&mut self, status: u16, method: Option<&str>, bytes: Option<u64>) {
        self.distribution.total_requests += 1;

        match status {
            100..=199 => self.distribution.c1xx += 1,
            200..=299 => self.distribution.c2xx += 1,
            300..=399 => self.distribution.c3xx += 1,
            400..=499 => self.distribution.c4xx += 1,
            500..=599 => self.distribution.c5xx += 1,
            _ => {}
        }

        *self.distribution.status_counts.entry(status).or_insert(0) += 1;

        if let Some(m) = method {
            let m_upper = m.to_ascii_uppercase();
            *self.distribution.method_counts.entry(m_upper).or_insert(0) += 1;
        }

        if let Some(b) = bytes {
            self.distribution.total_bytes = self.distribution.total_bytes.saturating_add(b);
            self.distribution.max_bytes = self.distribution.max_bytes.max(b);
        }
    }

    pub fn distribution(&self) -> &HttpStatusDistribution {
        &self.distribution
    }
}
