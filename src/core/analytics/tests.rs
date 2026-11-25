#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::core::analytics::bucket::{BucketInterval, SparseTimeBucketer};
    use crate::core::analytics::cardinality::{CountMinSketch, HyperLogLog, SpaceSavingTopK};
    use crate::core::analytics::engine::AnalyticsEngine;
    use crate::core::analytics::query::{
        AggregateField, AggregationType, AnalyticsQuery, EvaluationMode,
    };
    use crate::core::analytics::spill::DiskSpillManager;
    use crate::core::anomaly::baseline::RollingBaselineDetector;
    use crate::core::columnizer::WebserverClfColumnizer;
    use crate::core::compare::normalizer::ComparisonEngine;
    use crate::core::domain::endpoint::NormalizedEndpoint;
    use crate::core::domain::http::HttpStatusAnalytics;
    use crate::core::domain::ip::{CanonicalIp, TrustedProxyPolicy};
    use crate::core::domain::latency::LatencyAnalytics;
    use crate::core::domain::security::SecurityPresetAnalyzer;
    use crate::core::export::privacy::{PrivacyFilter, PrivacyMaskingOptions};
    use crate::core::export::report::OfflineReportGenerator;
    use crate::core::pattern::tokenizer::DynamicTokenizer;
    use crate::core::pattern::PatternGrouper;
    use crate::core::source::MockSource;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn test_space_saving_top_k() {
        let mut topk = SpaceSavingTopK::new(5);
        for _ in 0..100 {
            topk.insert("192.168.1.1");
        }
        for _ in 0..50 {
            topk.insert("192.168.1.2");
        }
        for _ in 0..25 {
            topk.insert("10.0.0.1");
        }
        for i in 0..10 {
            topk.insert(&format!("172.16.0.{}", i));
        }

        let res = topk.top_k();
        assert!(!res.is_empty());
        assert_eq!(res[0].item, "192.168.1.1");
        assert_eq!(res[0].count, 100);
        assert_eq!(res[1].item, "192.168.1.2");
        assert_eq!(res[1].count, 50);
        assert_eq!(res[2].item, "10.0.0.1");
        assert_eq!(res[2].count, 25);
    }

    #[test]
    fn test_count_min_sketch() {
        let mut cms = CountMinSketch::new(4, 256);
        for _ in 0..42 {
            cms.insert("login_failure");
        }
        for _ in 0..7 {
            cms.insert("404_not_found");
        }
        assert!(cms.estimate("login_failure") >= 42);
        assert!(cms.estimate("404_not_found") >= 7);
        assert_eq!(cms.estimate("non_existent"), 0);
    }

    #[test]
    fn test_hyperloglog_cardinality() {
        let mut hll = HyperLogLog::new(8);
        for i in 0..1000 {
            hll.insert(&format!("client_session_{}", i));
        }
        let est = hll.estimate();
        // HyperLogLog with 256 registers has ~6.5% standard error
        assert!(
            est >= 850 && est <= 1150,
            "Estimated {} was out of range",
            est
        );
    }

    #[test]
    fn test_bounded_disk_spill_and_merge() {
        let mut spill = DiskSpillManager::new(50);
        for i in 0..200 {
            spill.insert(format!("key_{:02}", i % 10));
        }
        assert!(spill.partition_count() >= 3);
        let merged = spill.merge_and_aggregate();
        assert_eq!(merged.len(), 10);
        for (k, c) in &merged {
            assert!(k.starts_with("key_"));
            assert_eq!(*c, 20);
        }
    }

    #[test]
    fn test_sparse_time_bucketing_and_downsample() {
        let mut bucketer = SparseTimeBucketer::new(BucketInterval::Seconds(10));
        bucketer.insert(1_000_000_000, false, 500);
        bucketer.insert(1_000_000_000 + 5_000_000_000, true, 200);
        bucketer.insert(1_000_000_000 + 30_000_000_000, false, 800);

        assert_eq!(bucketer.total_count(), 3);
        assert_eq!(bucketer.total_errors(), 1);
        assert_eq!(bucketer.buckets().len(), 2);

        let downsampled = bucketer.downsample(1);
        assert_eq!(downsampled.len(), 1);
        assert_eq!(downsampled[0].1.count, 3);
        assert_eq!(downsampled[0].1.error_count, 1);
    }

    #[test]
    fn test_http_status_distribution() {
        let mut http = HttpStatusAnalytics::new();
        http.record(200, Some("GET"), Some(1024));
        http.record(200, Some("GET"), Some(2048));
        http.record(404, Some("GET"), Some(256));
        http.record(500, Some("POST"), Some(512));

        let dist = http.distribution();
        assert_eq!(dist.total_requests, 4);
        assert_eq!(dist.c2xx, 2);
        assert_eq!(dist.c4xx, 1);
        assert_eq!(dist.c5xx, 1);
        assert_eq!(dist.client_error_rate(), 0.25);
        assert_eq!(dist.server_error_rate(), 0.25);
        assert_eq!(dist.avg_bytes(), 960.0);
    }

    #[test]
    fn test_canonical_ip_and_trusted_proxy() {
        let ip_v4 = CanonicalIp::parse(" 192.168.01.001 ").unwrap();
        assert_eq!(ip_v4.raw, "192.168.1.1");
        assert_eq!(ip_v4.mask_privacy(), "192.168.1.xxx");

        let ip_mapped = CanonicalIp::parse("::ffff:192.0.2.128").unwrap();
        assert_eq!(ip_mapped.raw, "192.0.2.128");
        assert!(!ip_mapped.is_ipv6);

        let policy = TrustedProxyPolicy::new(vec!["10.0.0.1".to_string()]);

        // Direct IP is trusted proxy: takes X-Forwarded-For
        let (client_ip, forwarded) =
            policy.resolve_client_ip("10.0.0.1", Some("203.0.113.195, 10.0.0.1"));
        assert!(forwarded);
        assert_eq!(client_ip.raw, "203.0.113.195");

        // Direct IP is untrusted: ignores X-Forwarded-For (spoof defense)
        let (untrusted_ip, forwarded_false) =
            policy.resolve_client_ip("198.51.100.2", Some("203.0.113.195"));
        assert!(!forwarded_false);
        assert_eq!(untrusted_ip.raw, "198.51.100.2");
    }

    #[test]
    fn test_endpoint_normalization() {
        let ep1 = NormalizedEndpoint::parse("GET", "/api/v1/users/12345/profile?lang=en&ref=home");
        assert_eq!(ep1.method, "GET");
        assert_eq!(ep1.path, "/api/v1/users/{id}/profile");

        let ep2 =
            NormalizedEndpoint::parse("POST", "/items/550e8400-e29b-41d4-a716-446655440000/edit");
        assert_eq!(ep2.path, "/items/{uuid}/edit");

        let ep3 = NormalizedEndpoint::parse("DELETE", "/commits/a1b2c3d4e5f6");
        assert_eq!(ep3.path, "/commits/{hash}");
    }

    #[test]
    fn test_latency_quantiles_and_sla() {
        let mut lat = LatencyAnalytics::new(500_000_000); // 500ms SLA
        for d in &[
            100_000_000,
            200_000_000,
            300_000_000,
            400_000_000,
            600_000_000,
            1_000_000_000,
        ] {
            lat.record_explicit_duration(*d);
        }
        let q = lat.compute_quantiles();
        assert_eq!(q.count, 6);
        assert_eq!(q.min_nanos, 100_000_000);
        assert_eq!(q.max_nanos, 1_000_000_000);
        assert_eq!(q.sla_breaches, 2);
        assert!(q.p90_nanos >= 600_000_000);
    }

    #[test]
    fn test_dynamic_tokenizer_and_pattern_grouping() {
        let line1 =
            "User 9876 logged in from 192.168.1.50 with token 550e8400-e29b-41d4-a716-446655440000";
        let line2 =
            "User 1234 logged in from 10.0.0.1 with token 6ba7b810-9dad-11d1-80b4-00c04fd430c8";

        let tmpl1 = DynamicTokenizer::tokenize(line1);
        let tmpl2 = DynamicTokenizer::tokenize(line2);
        assert_eq!(tmpl1, tmpl2);
        assert_eq!(tmpl1, "User <NUM> logged in from <IP> with token <UUID>");

        let mut grouper = PatternGrouper::new();
        grouper.record_line(line1, 0);
        grouper.record_line(line2, 1);
        let patterns = grouper.top_patterns(10);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].count, 2);
    }

    #[test]
    fn test_rolling_anomaly_detection() {
        let mut detector = RollingBaselineDetector::new(10, 3.0);
        // Feed normal baseline values (10, 11, 10, 12, 10, 11)
        for i in 0..10 {
            detector.evaluate(i * 1_000_000_000, "http_5xx", 10.0);
        }

        // Spike occurs: 50.0
        let anomaly = detector.evaluate(11 * 1_000_000_000, "http_5xx", 50.0);
        assert!(anomaly.is_some());
        let a = anomaly.unwrap();
        assert_eq!(a.metric_name, "http_5xx");
        assert_eq!(a.observed_value, 50.0);
        assert!(a.z_score >= 3.0);
        assert!(a.explanation.contains("exceeding rolling baseline"));
    }

    #[test]
    fn test_dataset_comparison_and_zero_baseline() {
        let summary = ComparisonEngine::compare(
            "Incident Day",
            1000,
            50,
            3600.0,
            "Baseline Day",
            2000,
            20,
            7200.0,
            &[("503".to_string(), 10), ("404".to_string(), 40)],
            &[
                ("503".to_string(), 0),
                ("404".to_string(), 60),
                ("500".to_string(), 5),
            ],
        );

        assert_eq!(summary.name_a, "Incident Day");
        assert_eq!(summary.name_b, "Baseline Day");

        // Verify zero baseline handling
        let key_500 = summary.keys.iter().find(|k| k.key == "500").unwrap();
        assert_eq!(key_500.count_a, 0);
        assert_eq!(key_500.count_b, 5);
        assert_eq!(key_500.diff_str, "+5 (new)");

        let key_503 = summary.keys.iter().find(|k| k.key == "503").unwrap();
        assert_eq!(key_503.count_a, 10);
        assert_eq!(key_503.count_b, 0);
        assert_eq!(key_503.diff_str, "-10 (removed)");
    }

    #[test]
    fn test_privacy_filter_masking() {
        let raw = "User test@example.com Authorization: Bearer secret_token_xyz password=MySecretPassword123 IP 192.168.1.100";
        let opts = PrivacyMaskingOptions::default();
        let masked = PrivacyFilter::mask_line(raw, opts);

        assert!(!masked.contains("secret_token_xyz"));
        assert!(masked.contains("***MASKED_TOKEN***"));
        assert!(!masked.contains("MySecretPassword123"));
        assert!(masked.contains("password=***MASKED***"));
        assert!(!masked.contains("test@example.com"));
        assert!(masked.contains("t***@example.com"));
        assert!(!masked.contains("192.168.1.100"));
        assert!(masked.contains("192.168.1.xxx"));
    }

    #[test]
    fn test_offline_html_report_xss_immunity() {
        let xss_title = "<script>alert('XSS')</script>";
        let escaped = OfflineReportGenerator::escape_html(xss_title);
        assert_eq!(escaped, "&lt;script&gt;alert(&#39;XSS&#39;)&lt;/script&gt;");
        assert!(!escaped.contains('<'));
        assert!(!escaped.contains('>'));
    }

    #[test]
    fn test_security_preset_analyzer() {
        let mut sec = SecurityPresetAnalyzer::new();
        let wazuh_line = r#"{"timestamp":"2026-11-25T10:00:00Z","rule":{"level":12,"description":"SSH Brute Force"}}"#;
        let suricata_line = r#"{"timestamp":"2026-11-25T10:00:01Z","event_type":"alert","alert":{"signature":"ET MALWARE Suspicious Domain"}}"#;
        let auth_line =
            r#"192.168.1.50 - - [25/Nov/2026:10:00:02 +0700] "POST /admin HTTP/1.1" 401 256"#;

        let obs1 = sec.inspect_line(wazuh_line, 0);
        assert!(obs1.is_some());
        assert_eq!(obs1.unwrap().category, "Wazuh High Severity Alert");

        let obs2 = sec.inspect_line(suricata_line, 1);
        assert!(obs2.is_some());
        assert_eq!(obs2.unwrap().category, "Suricata IDS Alert");

        let obs3 = sec.inspect_line(auth_line, 2);
        assert!(obs3.is_some());
        assert_eq!(obs3.unwrap().category, "Authentication Failure Observation");
    }

    #[test]
    fn test_streaming_analytics_engine_execution() {
        let sample = [
            r#"192.168.1.1 - admin [25/Nov/2026:10:00:00 +0700] "GET /index.html HTTP/1.1" 200 1024"#,
            r#"192.168.1.2 - guest [25/Nov/2026:10:00:01 +0700] "POST /login HTTP/1.1" 403 512"#,
            r#"192.168.1.1 - admin [25/Nov/2026:10:00:02 +0700] "GET /api/v1/users/10 HTTP/1.1" 200 2048"#,
            r#"192.168.1.3 - robot [25/Nov/2026:10:00:03 +0700] "GET /admin HTTP/1.1" 500 128"#,
        ];
        let mut source = MockSource::new("clf.log", &sample);
        let col = WebserverClfColumnizer;

        let mut query = AnalyticsQuery::new("Test Group By IP");
        query.add_group_by("Host");
        query.add_aggregate(AggregateField::new("Host", AggregationType::Count));
        query.set_mode(EvaluationMode::Exact);

        let cancel = AtomicBool::new(false);
        let result = AnalyticsEngine::execute_streaming(&mut source, &col, &query, &cancel);

        assert_eq!(result.total_lines_scanned, 4);
        assert_eq!(result.matched_records, 4);
        assert!(result.is_exact);
        assert_eq!(result.rows.len(), 3); // 192.168.1.1 (2), 192.168.1.2 (1), 192.168.1.3 (1)
        assert_eq!(result.rows[0].key, "192.168.1.1");
        assert_eq!(result.rows[0].count, 2);
    }
}
