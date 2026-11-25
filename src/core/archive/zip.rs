use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::core::archive::gzip::{
    inflate_deflate, TempFileGuard, MAX_COMPRESSION_RATIO, MAX_UNCOMPRESSED_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipMember {
    pub name: String,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub mtime: u64,
    pub is_dir: bool,
    pub compression_method: u16,
    pub local_header_offset: u64,
}

pub struct ZipArchiveReader {
    archive_path: PathBuf,
    members: Vec<ZipMember>,
}

impl ZipArchiveReader {
    pub fn open(path: &Path) -> Result<Self, String> {
        let mut file =
            File::open(path).map_err(|e| format!("Failed to open ZIP archive: {}", e))?;
        let file_len = file.metadata().map_err(|e| e.to_string())?.len();

        let members = parse_central_directory(&mut file, file_len)?;
        Ok(Self {
            archive_path: path.to_path_buf(),
            members,
        })
    }

    pub fn list_members(&self) -> &[ZipMember] {
        &self.members
    }

    /// Extracts a specific member to a secure temporary file guarded by TempFileGuard.
    /// Strictly protects against Path Traversal (RSK-05) and Zip-Bombs (RSK-04).
    pub fn extract_member_to_temp(&self, member_name: &str) -> Result<TempFileGuard, String> {
        validate_safe_zip_entry_path(member_name)?;

        let member = self
            .members
            .iter()
            .find(|m| m.name == member_name)
            .ok_or_else(|| format!("Member '{}' not found in archive", member_name))?;

        if member.uncompressed_size > MAX_UNCOMPRESSED_SIZE {
            return Err(format!(
                "Zip-Bomb hazard: member '{}' uncompressed size {} exceeds 2 GB",
                member_name, member.uncompressed_size
            ));
        }

        if member.compressed_size > 0
            && member.uncompressed_size / member.compressed_size.max(1) > MAX_COMPRESSION_RATIO
            && member.uncompressed_size > 10 * 1024 * 1024
        {
            return Err(format!(
                "Zip-Bomb hazard: member '{}' compression ratio exceeds 100:1 policy",
                member_name
            ));
        }

        let mut file =
            File::open(&self.archive_path).map_err(|e| format!("Open ZIP error: {}", e))?;

        // Seek to local file header
        file.seek(SeekFrom::Start(member.local_header_offset))
            .map_err(|e| e.to_string())?;

        let mut local_header = [0u8; 30];
        file.read_exact(&mut local_header)
            .map_err(|e| format!("Read local header error: {}", e))?;

        if &local_header[0..4] != b"PK\x03\x04" {
            return Err("Invalid local file header signature".to_string());
        }

        let name_len = u16::from_le_bytes([local_header[26], local_header[27]]) as usize;
        let extra_len = u16::from_le_bytes([local_header[28], local_header[29]]) as usize;

        // Skip filename and extra fields in local header
        file.seek(SeekFrom::Current((name_len + extra_len) as i64))
            .map_err(|e| e.to_string())?;

        let mut compressed_data = vec![0u8; member.compressed_size as usize];
        file.read_exact(&mut compressed_data)
            .map_err(|e| format!("Read compressed data error: {}", e))?;

        let uncompressed_data = match member.compression_method {
            0 => {
                // Stored (no compression)
                compressed_data
            }
            8 => {
                // Deflate
                inflate_deflate(&compressed_data, member.compressed_size)?
            }
            other => {
                return Err(format!("Unsupported ZIP compression method: {}", other));
            }
        };

        let guard = TempFileGuard::new("zip_extract")?;
        let mut out_file = File::create(guard.path())
            .map_err(|e| format!("Create extracted file error: {}", e))?;
        out_file
            .write_all(&uncompressed_data)
            .map_err(|e| format!("Write extracted file error: {}", e))?;
        out_file.flush().map_err(|e| e.to_string())?;

        Ok(guard)
    }
}

/// Strictly checks ZIP entry name to prevent Path Traversal attacks (RSK-05).
pub fn validate_safe_zip_entry_path(entry_path: &str) -> Result<(), String> {
    if entry_path.is_empty() {
        return Err("Empty ZIP entry name".to_string());
    }

    // Reject drive letters
    if entry_path.len() >= 2 && entry_path.as_bytes()[1] == b':' {
        return Err("Absolute drive letter path forbidden in ZIP entry".to_string());
    }

    // Reject leading slashes
    if entry_path.starts_with('/') || entry_path.starts_with('\\') {
        return Err("Leading slash path forbidden in ZIP entry".to_string());
    }

    // Reject directory traversal components
    let normalized = entry_path.replace('\\', "/");
    for part in normalized.split('/') {
        if part == ".." {
            return Err("Path traversal '..' detected in ZIP entry name".to_string());
        }
    }

    Ok(())
}

fn parse_central_directory(file: &mut File, file_len: u64) -> Result<Vec<ZipMember>, String> {
    if file_len < 22 {
        return Err("File too small to be a valid ZIP archive".to_string());
    }

    // Scan backwards from end of file for EOCD signature PK\x05\x06
    let scan_size = file_len.min(65557) as usize;
    let mut scan_buf = vec![0u8; scan_size];
    file.seek(SeekFrom::Start(file_len - scan_size as u64))
        .map_err(|e| e.to_string())?;
    file.read_exact(&mut scan_buf).map_err(|e| e.to_string())?;

    let mut eocd_pos = None;
    for i in (0..scan_size.saturating_sub(3)).rev() {
        if &scan_buf[i..i + 4] == b"PK\x05\x06" {
            eocd_pos = Some(file_len - scan_size as u64 + i as u64);
            break;
        }
    }

    let eocd_offset =
        eocd_pos.ok_or_else(|| "End of Central Directory signature not found".to_string())?;
    file.seek(SeekFrom::Start(eocd_offset + 10))
        .map_err(|e| e.to_string())?;

    let mut count_buf = [0u8; 2];
    file.read_exact(&mut count_buf).map_err(|e| e.to_string())?;
    let total_entries = u16::from_le_bytes(count_buf) as usize;

    file.seek(SeekFrom::Start(eocd_offset + 16))
        .map_err(|e| e.to_string())?;
    let mut cd_offset_buf = [0u8; 4];
    file.read_exact(&mut cd_offset_buf)
        .map_err(|e| e.to_string())?;
    let cd_offset = u32::from_le_bytes(cd_offset_buf) as u64;

    file.seek(SeekFrom::Start(cd_offset))
        .map_err(|e| e.to_string())?;

    let mut members = Vec::with_capacity(total_entries.min(1024));
    for _ in 0..total_entries {
        let mut header = [0u8; 46];
        if file.read_exact(&mut header).is_err() {
            break;
        }

        if &header[0..4] != b"PK\x01\x02" {
            break;
        }

        let comp_method = u16::from_le_bytes([header[10], header[11]]);
        let comp_size = u32::from_le_bytes([header[20], header[21], header[22], header[23]]) as u64;
        let uncomp_size =
            u32::from_le_bytes([header[24], header[25], header[26], header[27]]) as u64;
        let name_len = u16::from_le_bytes([header[28], header[29]]) as usize;
        let extra_len = u16::from_le_bytes([header[30], header[31]]) as usize;
        let comment_len = u16::from_le_bytes([header[32], header[33]]) as usize;
        let local_offset =
            u32::from_le_bytes([header[42], header[43], header[44], header[45]]) as u64;

        let mut name_buf = vec![0u8; name_len];
        file.read_exact(&mut name_buf).map_err(|e| e.to_string())?;
        let name = String::from_utf8_lossy(&name_buf).to_string();

        file.seek(SeekFrom::Current((extra_len + comment_len) as i64))
            .map_err(|e| e.to_string())?;

        let is_dir = name.ends_with('/') || name.ends_with('\\');

        members.push(ZipMember {
            name,
            compressed_size: comp_size,
            uncompressed_size: uncomp_size,
            mtime: 0,
            is_dir,
            compression_method: comp_method,
            local_header_offset: local_offset,
        });
    }

    Ok(members)
}
