use std::path::PathBuf;

use crate::core::export::privacy::PrivacyMaskingOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormatChoice {
    HtmlOffline,
    Json,
    Csv,
    Tsv,
}

pub struct ReportExportDialog {
    pub destination_path: PathBuf,
    pub format: ReportFormatChoice,
    pub masking_options: PrivacyMaskingOptions,
}

impl ReportExportDialog {
    pub fn new(destination_path: PathBuf) -> Self {
        Self {
            destination_path,
            format: ReportFormatChoice::HtmlOffline,
            masking_options: PrivacyMaskingOptions::default(),
        }
    }
}
