#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::core::legacy::migration::{
        LegacyExternalToolDef, LegacyFilterPreset, LegacyHighlightRule, SettingsMigrationWizard,
    };
    use crate::core::legacy::session_importer::LegacySessionImporter;

    #[test]
    fn test_safe_legacy_session_import() {
        let sample_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<logExpertSession>
  <document>
    <fileName>C:\logs\production.log</fileName>
    <columnizer>Webserver CLF</columnizer>
    <currentLine>142</currentLine>
    <bookmark>42</bookmark>
    <bookmark>99</bookmark>
    <filterText>ERROR 500</filterText>
  </document>
</logExpertSession>"#;

        let (session, summary) =
            LegacySessionImporter::import_session(sample_xml).expect("Import safe legacy XML");

        assert_eq!(summary.documents_imported, 1);
        assert_eq!(summary.bookmarks_imported, 2);
        assert_eq!(summary.filter_rules_imported, 1);

        assert_eq!(session.documents.len(), 1);
        let doc = &session.documents[0];
        assert_eq!(doc.path.as_deref(), Some("C:\\logs\\production.log"));
        assert_eq!(doc.columnizer_id, "Webserver CLF");
        assert_eq!(doc.selected_line, 142);
        assert_eq!(doc.bookmarks, vec![42, 99]);
        assert_eq!(doc.filter_pattern.as_deref(), Some("ERROR 500"));
    }

    #[test]
    fn test_anti_xxe_entity_rejection() {
        // Malicious XXE injection attempt
        let evil_xml = r#"<?xml version="1.0"?>
<!DOCTYPE foo [
<!ELEMENT foo ANY >
<!ENTITY xxe SYSTEM "file:///c:/windows/win.ini" >]>
<logExpertSession>
  <fileName>&xxe;</fileName>
</logExpertSession>"#;

        let result = LegacySessionImporter::import_session(evil_xml);
        assert!(
            result.is_err(),
            "Must strictly reject XXE DOCTYPE/ENTITY payloads"
        );
        let err = result.err().unwrap();
        assert!(err.contains("XXE Hazard"));
    }

    #[test]
    fn test_settings_migration_wizard() {
        let legacy_highlights = vec![
            LegacyHighlightRule {
                pattern: "FATAL".to_string(),
                fg_color: 0x00FFFFFF,
                bg_color: 0x000000FF,
                bold: true,
                case_sensitive: true,
            },
            LegacyHighlightRule {
                pattern: "WARN*".to_string(),
                fg_color: 0x00000000,
                bg_color: 0x0000FFFF,
                bold: false,
                case_sensitive: false,
            },
        ];

        let group =
            SettingsMigrationWizard::migrate_highlight_rules("Server Alarms", &legacy_highlights);
        assert_eq!(group.rules.len(), 2);
        assert_eq!(group.rules[0].pattern, "FATAL");
        assert!(group.rules[0].bold);
        assert!(group.rules[1].is_regex); // Pattern with '*' treated as regex

        let legacy_filters = vec![LegacyFilterPreset {
            name: "Errors Only".to_string(),
            include_pattern: "ERR".to_string(),
            exclude_pattern: "DEBUG".to_string(),
            case_sensitive: false,
        }];
        let filters = SettingsMigrationWizard::migrate_filter_presets(&legacy_filters);
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].name, "Errors Only");
        assert_eq!(filters[0].pattern, "ERR");

        let legacy_tools = vec![LegacyExternalToolDef {
            name: "Curl Ping".to_string(),
            command: "curl.exe".to_string(),
            arguments: "https://api.internal/health".to_string(),
        }];
        let tools = SettingsMigrationWizard::migrate_external_tools(&legacy_tools);
        assert_eq!(tools.len(), 1);
        assert!(
            !tools[0].is_trusted,
            "Migrated external tools must be untrusted by default"
        );
    }
}
