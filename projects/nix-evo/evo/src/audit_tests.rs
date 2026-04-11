#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // ─── AuditEntry serialization ───────────────────────────────────

    #[test]
    fn test_audit_entry_roundtrip() {
        let entry = AuditEntry {
            timestamp: "1712899200".to_string(),
            action: "config_apply".to_string(),
            method: "POST".to_string(),
            path: "/api/config/apply".to_string(),
            params_hash: "abcd1234efgh5678".to_string(),
            client_ip: "127.0.0.1".to_string(),
            result: "success".to_string(),
            duration_ms: 150,
        };

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: AuditEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.action, "config_apply");
        assert_eq!(parsed.method, "POST");
        assert_eq!(parsed.params_hash, "abcd1234efgh5678");
        assert_eq!(parsed.duration_ms, 150);
    }

    #[test]
    fn test_audit_entry_json_format() {
        let entry = AuditEntry {
            timestamp: "1712899200".to_string(),
            action: "rollback".to_string(),
            method: "POST".to_string(),
            path: "/api/rollback".to_string(),
            params_hash: "deadbeef".to_string(),
            client_ip: "10.0.0.1".to_string(),
            result: "rolled_back_to_42".to_string(),
            duration_ms: 2300,
        };

        let json = serde_json::to_string(&entry).unwrap();
        // Must be single-line JSONL
        assert!(!json.contains('\n'));
        assert!(json.contains(""action":"rollback""));
        assert!(json.contains(""duration_ms":2300"));
    }

    // ─── hash_params ────────────────────────────────────────────────

    #[test]
    fn test_hash_params_deterministic() {
        let h1 = hash_params("nginx config change");
        let h2 = hash_params("nginx config change");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_params_different_inputs() {
        let h1 = hash_params("config A");
        let h2 = hash_params("config B");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_params_empty() {
        let h = hash_params("");
        assert_eq!(h.len(), 16); // 64-bit hex = 16 chars
    }

    #[test]
    fn test_hash_params_length() {
        let h = hash_params("any input");
        assert_eq!(h.len(), 16);
        // Valid hex
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // ─── extract_action ─────────────────────────────────────────────

    #[test]
    fn test_extract_action_snapshot() {
        assert_eq!(extract_action("/api/snapshot"), "system_snapshot");
    }

    #[test]
    fn test_extract_action_config_validate() {
        assert_eq!(extract_action("/api/config/validate"), "config_validate");
    }

    #[test]
    fn test_extract_action_config_apply() {
        assert_eq!(extract_action("/api/config/apply"), "config_apply");
    }

    #[test]
    fn test_extract_action_rollback() {
        assert_eq!(extract_action("/api/rollback"), "rollback");
    }

    #[test]
    fn test_extract_action_unknown() {
        assert_eq!(extract_action("/api/mystery"), "unknown");
    }

    #[test]
    fn test_extract_action_dashboard() {
        assert_eq!(extract_action("/api/dashboard/ws"), "dashboard");
    }

    // ─── AuditLog query filtering ───────────────────────────────────

    #[test]
    fn test_audit_query_filter_by_action() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("audit.log");

        // Write test entries
        let entries = vec![
            AuditEntry {
                timestamp: "100".into(), action: "config_apply".into(),
                method: "POST".into(), path: "/api/config/apply".into(),
                params_hash: "aaa".into(), client_ip: "127.0.0.1".into(),
                result: "ok".into(), duration_ms: 100,
            },
            AuditEntry {
                timestamp: "101".into(), action: "system_snapshot".into(),
                method: "GET".into(), path: "/api/snapshot".into(),
                params_hash: "bbb".into(), client_ip: "127.0.0.1".into(),
                result: "ok".into(), duration_ms: 50,
            },
            AuditEntry {
                timestamp: "102".into(), action: "config_apply".into(),
                method: "POST".into(), path: "/api/config/apply".into(),
                params_hash: "ccc".into(), client_ip: "127.0.0.1".into(),
                result: "ok".into(), duration_ms: 200,
            },
        ];

        let mut file = std::fs::File::create(&log_path).unwrap();
        for entry in &entries {
            use std::io::Write;
            writeln!(file, "{}", serde_json::to_string(entry).unwrap()).unwrap();
        }

        let log = AuditLog { path: log_path.clone() };

        // Filter for config_apply
        let result = log.query(&AuditQuery {
            action: Some("config_apply".into()),
            path: None,
            limit: Some(100),
        }).unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|e| e.action == "config_apply"));
    }

    #[test]
    fn test_audit_query_limit() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("audit.log");

        let mut file = std::fs::File::create(&log_path).unwrap();
        for i in 0..20 {
            let entry = AuditEntry {
                timestamp: format!("{}", 100 + i),
                action: "test".into(),
                method: "GET".into(),
                path: "/api/test".into(),
                params_hash: format!("h{i}"),
                client_ip: "127.0.0.1".into(),
                result: "ok".into(),
                duration_ms: 10,
            };
            use std::io::Write;
            writeln!(file, "{}", serde_json::to_string(&entry).unwrap()).unwrap();
        }

        let log = AuditLog { path: log_path };

        let result = log.query(&AuditQuery {
            action: None, path: None, limit: Some(5),
        }).unwrap();

        // Should return last 5 entries
        assert_eq!(result.len(), 5);
        assert_eq!(result[0].timestamp, "115");
        assert_eq!(result[4].timestamp, "119");
    }

    #[test]
    fn test_audit_query_empty_log() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("audit.log");
        std::fs::File::create(&log_path).unwrap(); // empty file

        let log = AuditLog { path: log_path };
        let result = log.query(&AuditQuery {
            action: None, path: None, limit: None,
        }).unwrap();

        assert!(result.is_empty());
    }
}
