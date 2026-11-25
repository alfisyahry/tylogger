use crate::core::filter::{FilterMode, FilterPreset};
use crate::core::highlight::{HighlightGroup, HighlightRule};
use crate::core::tools::ExternalToolConfig;

#[derive(Debug, Clone)]
pub struct LegacyHighlightRule {
    pub pattern: String,
    pub fg_color: u32,
    pub bg_color: u32,
    pub bold: bool,
    pub case_sensitive: bool,
}

#[derive(Debug, Clone)]
pub struct LegacyFilterPreset {
    pub name: String,
    pub include_pattern: String,
    pub exclude_pattern: String,
    pub case_sensitive: bool,
}

#[derive(Debug, Clone)]
pub struct LegacyExternalToolDef {
    pub name: String,
    pub command: String,
    pub arguments: String,
}

pub struct SettingsMigrationWizard;

impl SettingsMigrationWizard {
    /// Migrates legacy LogExpert highlight rules into a Tylogger HighlightGroup.
    pub fn migrate_highlight_rules(
        group_name: &str,
        legacy_rules: &[LegacyHighlightRule],
    ) -> HighlightGroup {
        let mut group = HighlightGroup::new(group_name);
        let total = legacy_rules.len();
        for (i, r) in legacy_rules.iter().enumerate() {
            let rule = HighlightRule {
                id: (i + 1) as u32,
                name: format!("Rule {}", i + 1),
                pattern: r.pattern.clone(),
                is_regex: r.pattern.contains('*')
                    || r.pattern.contains('^')
                    || r.pattern.contains('$'),
                case_sensitive: r.case_sensitive,
                whole_word: false,
                column_index: None,
                fg_color: r.fg_color,
                bg_color: r.bg_color,
                bold: r.bold,
                priority: (total - i) as i32,
                marker_bar: true,
                enabled: true,
            };
            group.add_rule(rule);
        }
        group
    }

    /// Migrates legacy LogExpert filter presets into Tylogger FilterPreset objects.
    pub fn migrate_filter_presets(legacy_presets: &[LegacyFilterPreset]) -> Vec<FilterPreset> {
        legacy_presets
            .iter()
            .map(|p| FilterPreset {
                name: p.name.clone(),
                pattern: p.include_pattern.clone(),
                mode: FilterMode::Include,
                case_sensitive: p.case_sensitive,
                whole_word: false,
                is_regex: false,
                search_mode: "plain".to_string(),
                range_start: None,
                range_end: None,
                back_spread: 0,
                fore_spread: 0,
                column_index: None,
                columnizer_id: None,
            })
            .collect()
    }

    /// Migrates legacy external tools into Tylogger ExternalToolConfig.
    /// Strictly marks them untrusted / disabled by default for user review (RSK-11).
    pub fn migrate_external_tools(
        legacy_tools: &[LegacyExternalToolDef],
    ) -> Vec<ExternalToolConfig> {
        legacy_tools
            .iter()
            .enumerate()
            .map(|(i, t)| ExternalToolConfig {
                id: (i + 1) as u32,
                display_name: t.name.clone(),
                executable_path: t.command.clone(),
                working_dir: None,
                arg_template: t
                    .arguments
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect(),
                capture_stdout: true,
                capture_stderr: true,
                timeout_ms: 30000,
                output_size_limit_bytes: 10 * 1024 * 1024,
                is_trusted: false, // Untrusted/disabled by default for defense in depth!
            })
            .collect()
    }
}
