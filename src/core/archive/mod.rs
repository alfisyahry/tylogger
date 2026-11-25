pub mod gzip;
pub mod zip;

#[cfg(test)]
mod tests;

pub use gzip::{
    decompress_gzip, inflate_deflate, GzipLogReader, TempFileGuard, MAX_COMPRESSION_RATIO,
    MAX_UNCOMPRESSED_SIZE,
};
pub use zip::{validate_safe_zip_entry_path, ZipArchiveReader, ZipMember};
