// src/core/tools/execution.rs
//
// Safe Structured Argument Construction & Placeholder Substitution for External Tools
// Strictly avoids shell invocation (zero cmd.exe / powershell.exe) and command injection.

#[derive(Debug, Clone, Default)]
pub struct ToolPlaceholderContext {
    pub file_path: Option<String>,
    pub file_name: Option<String>,
    pub source_dir: Option<String>,
    pub line_number: Option<u64>,
    pub selected_text: Option<String>,
    pub current_record: Option<String>,
    pub timestamp: Option<String>,
    pub parsed_ip: Option<String>,
}

pub struct SafeProcessLauncher;

impl SafeProcessLauncher {
    /// Substitute placeholders directly into structured argument slots (zero shell concatenation).
    pub fn build_arguments(template: &[String], ctx: &ToolPlaceholderContext) -> Vec<String> {
        template
            .iter()
            .map(|arg| {
                let mut res = arg.clone();

                if let Some(fpath) = &ctx.file_path {
                    res = res.replace("%FILE%", fpath);
                }
                if let Some(fname) = &ctx.file_name {
                    res = res.replace("%FILENAME%", fname);
                }
                if let Some(dir) = &ctx.source_dir {
                    res = res.replace("%DIR%", dir);
                }
                if let Some(line) = ctx.line_number {
                    res = res.replace("%LINE%", &line.to_string());
                }
                if let Some(sel) = &ctx.selected_text {
                    res = res.replace("%SELECTION%", sel);
                }
                if let Some(rec) = &ctx.current_record {
                    res = res.replace("%RECORD%", rec);
                }
                if let Some(ts) = &ctx.timestamp {
                    res = res.replace("%TIMESTAMP%", ts);
                }
                if let Some(ip) = &ctx.parsed_ip {
                    res = res.replace("%IP%", ip);
                }

                res
            })
            .collect()
    }

    /// Format structured arguments into Windows CommandLineToArgvW-compatible string.
    pub fn format_windows_command_line(executable: &str, args: &[String]) -> String {
        let mut cmd = String::new();

        // Push quoted executable
        if executable.contains(' ') || executable.is_empty() {
            cmd.push('"');
            cmd.push_str(executable);
            cmd.push('"');
        } else {
            cmd.push_str(executable);
        }

        for arg in args {
            cmd.push(' ');
            if arg.contains(' ') || arg.contains('\t') || arg.contains('"') || arg.is_empty() {
                cmd.push('"');
                for c in arg.chars() {
                    if c == '"' {
                        cmd.push('\\');
                    }
                    cmd.push(c);
                }
                cmd.push('"');
            } else {
                cmd.push_str(arg);
            }
        }

        cmd
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_injection_immunity() {
        let template = vec!["-c".to_string(), "4".to_string(), "%SELECTION%".to_string()];

        // Malicious string containing shell chaining operator
        let ctx = ToolPlaceholderContext {
            selected_text: Some("127.0.0.1 & del C:\\test.txt".to_string()),
            ..Default::default()
        };

        let args = SafeProcessLauncher::build_arguments(&template, &ctx);

        // Third argument remains an intact single vector element, NOT split into a shell command!
        assert_eq!(args.len(), 3);
        assert_eq!(args[2], "127.0.0.1 & del C:\\test.txt");

        let cmd_line = SafeProcessLauncher::format_windows_command_line("ping.exe", &args);
        // Quoted properly for CommandLineToArgvW
        assert!(cmd_line.contains("\"127.0.0.1 & del C:\\test.txt\""));
    }
}
