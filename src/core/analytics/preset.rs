use crate::core::analytics::query::{
    AggregateField, AggregationType, AnalyticsQuery, EvaluationMode, FilterCondition, QueryOperator,
};

#[derive(Debug, Clone, PartialEq)]
pub struct AnalyticsPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub query: AnalyticsQuery,
}

pub struct PresetManager;

impl PresetManager {
    pub fn builtin_presets() -> Vec<AnalyticsPreset> {
        vec![
            AnalyticsPreset {
                id: "http_status".to_string(),
                name: "HTTP Status Code Breakdown".to_string(),
                description:
                    "Distribution of HTTP response status codes (1xx - 5xx) with error rates."
                        .to_string(),
                category: "Web & Proxy".to_string(),
                query: {
                    let mut q = AnalyticsQuery::new("HTTP Status Breakdown");
                    q.add_group_by("status");
                    q.add_aggregate(AggregateField::new("status", AggregationType::Count));
                    q.set_limit(50);
                    q
                },
            },
            AnalyticsPreset {
                id: "top_client_ips".to_string(),
                name: "Top Client IP Addresses".to_string(),
                description:
                    "Top requesting client IP addresses with traffic volume and percentage share."
                        .to_string(),
                category: "Traffic & Security".to_string(),
                query: {
                    let mut q = AnalyticsQuery::new("Top Client IPs");
                    q.add_group_by("ip");
                    q.add_aggregate(AggregateField::new("ip", AggregationType::Count));
                    q.set_limit(50);
                    q
                },
            },
            AnalyticsPreset {
                id: "top_endpoints".to_string(),
                name: "Top Requested Endpoints".to_string(),
                description: "Most requested URLs and endpoints with error rates and volume."
                    .to_string(),
                category: "Web & Proxy".to_string(),
                query: {
                    let mut q = AnalyticsQuery::new("Top Endpoints");
                    q.add_group_by("endpoint");
                    q.add_aggregate(AggregateField::new("endpoint", AggregationType::Count));
                    q.set_limit(50);
                    q
                },
            },
            AnalyticsPreset {
                id: "auth_failures".to_string(),
                name: "Authentication Failures & Access Denials".to_string(),
                description:
                    "Investigate repeated 401 Unauthorized, 403 Forbidden, and auth failure logs."
                        .to_string(),
                category: "Security".to_string(),
                query: {
                    let mut q = AnalyticsQuery::new("Auth Failures");
                    q.add_filter(FilterCondition::new("status", QueryOperator::Equals, "401"));
                    q.add_group_by("ip");
                    q.add_aggregate(AggregateField::new("status", AggregationType::Count));
                    q.set_limit(100);
                    q
                },
            },
            AnalyticsPreset {
                id: "server_errors".to_string(),
                name: "HTTP 5xx Server Error Bursts".to_string(),
                description:
                    "Identify backend outages, bad gateways, and unhandled server exceptions."
                        .to_string(),
                category: "Availability".to_string(),
                query: {
                    let mut q = AnalyticsQuery::new("HTTP 5xx Errors");
                    q.add_filter(FilterCondition::new(
                        "status",
                        QueryOperator::GreaterThan,
                        "499",
                    ));
                    q.add_group_by("status");
                    q.add_aggregate(AggregateField::new("status", AggregationType::Count));
                    q.set_limit(50);
                    q
                },
            },
            AnalyticsPreset {
                id: "wazuh_alerts".to_string(),
                name: "Wazuh Security Alerts Summary".to_string(),
                description:
                    "Aggregated Wazuh security events grouped by rule ID and severity level."
                        .to_string(),
                category: "SIEM".to_string(),
                query: {
                    let mut q = AnalyticsQuery::new("Wazuh Security Alerts");
                    q.add_group_by("rule_level");
                    q.add_aggregate(AggregateField::new("rule_level", AggregationType::Count));
                    q.set_mode(EvaluationMode::Exact);
                    q.set_limit(100);
                    q
                },
            },
            AnalyticsPreset {
                id: "suricata_signatures".to_string(),
                name: "Suricata EVE IDS Signatures".to_string(),
                description:
                    "Breakdown of detected network intrusion signatures from Suricata eve.json."
                        .to_string(),
                category: "IDS".to_string(),
                query: {
                    let mut q = AnalyticsQuery::new("Suricata Signatures");
                    q.add_group_by("signature");
                    q.add_aggregate(AggregateField::new("signature", AggregationType::Count));
                    q.set_limit(100);
                    q
                },
            },
        ]
    }
}
