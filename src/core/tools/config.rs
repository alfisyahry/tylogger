// src/core/tools/config.rs
//
// External Tools Configuration & Trust Model for Tylogger
// Untrusted inputs, structured parameters, explicit trust confirmation for imported tools.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalToolConfig {
    pub id: u32,
    pub display_name: String,
    pub executable_path: String,
    pub working_dir: Option<String>,
    pub arg_template: Vec<String>,
    pub capture_stdout: bool,
    pub capture_stderr: bool,
    pub timeout_ms: u64,
    pub output_size_limit_bytes: usize,
    pub is_trusted: bool,
}

impl ExternalToolConfig {
    pub fn new(
        id: u32,
        display_name: &str,
        executable_path: &str,
        arg_template: Vec<String>,
    ) -> Self {
        Self {
            id,
            display_name: display_name.to_string(),
            executable_path: executable_path.to_string(),
            working_dir: None,
            arg_template,
            capture_stdout: true,
            capture_stderr: true,
            timeout_ms: 10_000,                       // 10s default timeout
            output_size_limit_bytes: 5 * 1024 * 1024, // 5 MB default limit
            is_trusted: false, // Default unapproved until explicit user trust
        }
    }

    pub fn set_trusted(&mut self, trusted: bool) {
        self.is_trusted = trusted;
    }
}
