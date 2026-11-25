#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamTemplate {
    TailFollow {
        path: String,
        lines: Option<usize>,
    },
    Journald {
        unit: Option<String>,
        since: Option<String>,
        priority: Option<String>,
        cursor: Option<String>,
    },
    DockerLogs {
        container: String,
        timestamps: bool,
        tail: Option<usize>,
    },
}

impl StreamTemplate {
    /// Validates all parameters and builds the exact read-only shell command.
    /// Strictly rejects any shell chaining operators (| ; & ` $ () > < \n \r).
    pub fn build_command(&self) -> Result<String, String> {
        match self {
            StreamTemplate::TailFollow { path, lines } => {
                validate_safe_argument(path)?;
                let mut cmd = String::from("tail -F");
                if let Some(n) = lines {
                    cmd.push_str(&format!(" -n {}", n));
                }
                cmd.push(' ');
                cmd.push_str(path);
                Ok(cmd)
            }
            StreamTemplate::Journald {
                unit,
                since,
                priority,
                cursor,
            } => {
                let mut cmd = String::from("journalctl -f -o json");
                if let Some(u) = unit {
                    validate_safe_argument(u)?;
                    cmd.push_str(&format!(" --unit {}", u));
                }
                if let Some(s) = since {
                    validate_safe_argument(s)?;
                    cmd.push_str(&format!(" --since \"{}\"", s));
                }
                if let Some(p) = priority {
                    validate_safe_argument(p)?;
                    cmd.push_str(&format!(" -p {}", p));
                }
                if let Some(c) = cursor {
                    validate_safe_argument(c)?;
                    cmd.push_str(&format!(" --after-cursor=\"{}\"", c));
                }
                Ok(cmd)
            }
            StreamTemplate::DockerLogs {
                container,
                timestamps,
                tail,
            } => {
                validate_safe_argument(container)?;
                let mut cmd = String::from("docker logs --follow");
                if *timestamps {
                    cmd.push_str(" --timestamps");
                }
                if let Some(t) = tail {
                    cmd.push_str(&format!(" --tail {}", t));
                }
                cmd.push(' ');
                cmd.push_str(container);
                Ok(cmd)
            }
        }
    }
}

/// Strictly checks argument for shell injection vulnerabilities (RSK-03).
pub fn validate_safe_argument(arg: &str) -> Result<(), String> {
    if arg.trim().is_empty() {
        return Err("Argument cannot be empty".to_string());
    }

    const FORBIDDEN_CHARS: &[char] = &[
        ';', '&', '|', '`', '$', '(', ')', '{', '}', '<', '>', '\n', '\r', '\t', '\0', '\\',
    ];

    for &ch in FORBIDDEN_CHARS {
        if arg.contains(ch) {
            return Err(format!(
                "Command injection hazard: illegal character '{}' in argument",
                ch
            ));
        }
    }

    Ok(())
}
