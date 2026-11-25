use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::core::reader::UpdateEvent;
use crate::core::source::{
    allocate_source_id, LogSource, SingleFileSource, SourceCapabilities, SourceIdentity,
    SourceLineRef, SourceType,
};

/// Maximum decompression expansion ratio allowed before flagging an archive bomb (RSK-04).
pub const MAX_COMPRESSION_RATIO: u64 = 100;
/// Maximum decompressed size in bytes (2 GB).
pub const MAX_UNCOMPRESSED_SIZE: u64 = 2 * 1024 * 1024 * 1024;

/// RAII Temporary File Guard that automatically deletes the file on drop (RSK-05 / RSK-17).
pub struct TempFileGuard {
    path: PathBuf,
}

impl TempFileGuard {
    pub fn new(prefix: &str) -> Result<Self, String> {
        let temp_dir = std::env::temp_dir().join("Tylogger").join("Temp");
        let _ = fs::create_dir_all(&temp_dir);
        let filename = format!(
            "{}_{}_{}.tmp",
            prefix,
            std::process::id(),
            allocate_source_id()
        );
        let path = temp_dir.join(filename);
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// GZIP log reader that decompresses stream to a managed temporary file with anti-bomb guards.
pub struct GzipLogReader {
    id: u32,
    inner_source: SingleFileSource,
    _guard: TempFileGuard,
    archive_path: PathBuf,
}

impl GzipLogReader {
    pub fn open(gzip_path: &Path) -> Result<Self, String> {
        let mut gz_file =
            File::open(gzip_path).map_err(|e| format!("Open GZIP file error: {}", e))?;
        let compressed_size = gz_file.metadata().map(|m| m.len()).unwrap_or(0);

        let guard = TempFileGuard::new("remote_gzip")?;
        let mut out_file = File::create(guard.path())
            .map_err(|e| format!("Create temp extraction file error: {}", e))?;

        // Decompress GZIP payload into temp file
        let mut buffer = Vec::new();
        gz_file
            .read_to_end(&mut buffer)
            .map_err(|e| format!("Read GZIP error: {}", e))?;

        let decompressed = decompress_gzip(&buffer, compressed_size)?;
        out_file
            .write_all(&decompressed)
            .map_err(|e| format!("Write decompressed GZIP error: {}", e))?;
        out_file.flush().map_err(|e| e.to_string())?;

        let inner_source = SingleFileSource::open(guard.path())?;
        Ok(Self {
            id: allocate_source_id(),
            inner_source,
            _guard: guard,
            archive_path: gzip_path.to_path_buf(),
        })
    }
}

impl LogSource for GzipLogReader {
    fn source_id(&self) -> u32 {
        self.id
    }

    fn generation(&self) -> u64 {
        self.inner_source.generation()
    }

    fn display_name(&self) -> &str {
        self.inner_source.display_name()
    }

    fn source_path(&self) -> Option<&Path> {
        Some(&self.archive_path)
    }

    fn line_count(&self) -> usize {
        self.inner_source.line_count()
    }

    fn file_size(&self) -> u64 {
        self.inner_source.file_size()
    }

    fn get_line(&mut self, idx: usize) -> Result<String, String> {
        self.inner_source.get_line(idx)
    }

    fn get_line_ref(&self, idx: usize) -> SourceLineRef {
        self.inner_source.get_line_ref(idx)
    }

    fn check_for_updates(&mut self) -> Result<UpdateEvent, String> {
        Ok(UpdateEvent::NoChange)
    }

    fn toggle_bookmark(&mut self, idx: usize) -> bool {
        self.inner_source.toggle_bookmark(idx)
    }

    fn is_bookmarked(&self, idx: usize) -> bool {
        self.inner_source.is_bookmarked(idx)
    }

    fn bookmarks(&self) -> &[usize] {
        self.inner_source.bookmarks()
    }

    fn source_type(&self) -> SourceType {
        SourceType::CompressedArchive
    }

    fn capabilities(&self) -> SourceCapabilities {
        SourceCapabilities {
            can_read: true,
            can_random_access: true,
            can_monitor: false,
            can_discover_files: false,
            can_detect_rotation: false,
            can_reconnect: false,
            can_resume: false,
            has_stable_identity: true,
            is_live_append: false,
            is_end_of_stream: true,
        }
    }

    fn identity(&self) -> SourceIdentity {
        SourceIdentity::new_archive(&self.archive_path, "content.log", self.file_size())
    }
}

/// Decompresses RFC 1952 GZIP data with anti-archive bomb validation.
pub fn decompress_gzip(data: &[u8], compressed_size: u64) -> Result<Vec<u8>, String> {
    if data.len() < 10 {
        return Err("Invalid GZIP header (too short)".to_string());
    }

    // Header check: ID1=0x1F, ID2=0x8B, CM=8 (Deflate)
    if data[0] != 0x1F || data[1] != 0x8B {
        return Err("Invalid GZIP magic bytes".to_string());
    }
    if data[2] != 8 {
        return Err(format!("Unsupported GZIP compression method: {}", data[2]));
    }

    let flg = data[3];
    let mut offset = 10;

    // FEXTRA flag
    if (flg & 0x04) != 0 {
        if offset + 2 > data.len() {
            return Err("Malformed GZIP extra field".to_string());
        }
        let xlen = data[offset] as usize | ((data[offset + 1] as usize) << 8);
        offset += 2 + xlen;
    }

    // FNAME flag (zero-terminated)
    if (flg & 0x08) != 0 {
        while offset < data.len() && data[offset] != 0 {
            offset += 1;
        }
        offset += 1; // skip null
    }

    // FCOMMENT flag (zero-terminated)
    if (flg & 0x10) != 0 {
        while offset < data.len() && data[offset] != 0 {
            offset += 1;
        }
        offset += 1; // skip null
    }

    // FHCRC flag
    if (flg & 0x02) != 0 {
        offset += 2;
    }

    if offset > data.len() || data.len() < 8 {
        return Err("Malformed GZIP data boundary".to_string());
    }

    let payload = &data[offset..data.len() - 8];
    // Decompress RFC 1951 DEFLATE
    inflate_deflate(payload, compressed_size)
}

/// RFC 1951 DEFLATE decompressor with strict Zip-Bomb guards.
pub fn inflate_deflate(data: &[u8], compressed_size: u64) -> Result<Vec<u8>, String> {
    let mut bit_reader = BitReader::new(data);
    let mut out = Vec::new();

    loop {
        let bfinal = bit_reader.read_bits(1)?;
        let btype = bit_reader.read_bits(2)?;

        match btype {
            0 => {
                // Stored block (uncompressed)
                bit_reader.align_byte();
                let len = bit_reader.read_bits(16)? as usize;
                let _nlen = bit_reader.read_bits(16)?;
                for _ in 0..len {
                    let b = bit_reader.read_bits(8)? as u8;
                    out.push(b);
                    check_zip_bomb(out.len() as u64, compressed_size)?;
                }
            }
            1 => {
                // Fixed Huffman codes
                inflate_huffman_block(&mut bit_reader, &mut out, compressed_size, true)?;
            }
            2 => {
                // Dynamic Huffman codes
                inflate_huffman_block(&mut bit_reader, &mut out, compressed_size, false)?;
            }
            _ => return Err("Invalid DEFLATE block type (3)".to_string()),
        }

        if bfinal == 1 {
            break;
        }
    }

    Ok(out)
}

pub fn check_zip_bomb(uncompressed_len: u64, compressed_size: u64) -> Result<(), String> {
    if uncompressed_len > MAX_UNCOMPRESSED_SIZE {
        return Err(format!(
            "Zip-Bomb hazard: decompressed size {} exceeds 2 GB limit",
            uncompressed_len
        ));
    }
    let base_size = compressed_size.max(1);
    if uncompressed_len / base_size > MAX_COMPRESSION_RATIO && uncompressed_len > 10 * 1024 * 1024 {
        return Err(format!(
            "Zip-Bomb hazard: compression ratio {}:1 exceeds maximum 100:1 policy",
            uncompressed_len / base_size
        ));
    }
    Ok(())
}

fn inflate_huffman_block(
    r: &mut BitReader,
    out: &mut Vec<u8>,
    compressed_size: u64,
    fixed: bool,
) -> Result<(), String> {
    let (lit_tree, dist_tree) = if fixed {
        build_fixed_trees()
    } else {
        build_dynamic_trees(r)?
    };

    loop {
        let sym = lit_tree.decode(r)?;
        if sym < 256 {
            out.push(sym as u8);
            check_zip_bomb(out.len() as u64, compressed_size)?;
        } else if sym == 256 {
            // End of block
            break;
        } else if sym <= 285 {
            let (length, extra_bits) = get_length_extra(sym);
            let extra = if extra_bits > 0 {
                r.read_bits(extra_bits)? as usize
            } else {
                0
            };
            let total_len = length + extra;

            let dist_sym = dist_tree.decode(r)?;
            let (dist_base, dist_extra_bits) = get_dist_extra(dist_sym);
            let dist_extra = if dist_extra_bits > 0 {
                r.read_bits(dist_extra_bits)? as usize
            } else {
                0
            };
            let distance = dist_base + dist_extra;

            if distance > out.len() {
                return Err(format!(
                    "Invalid distance {} exceeds output buffer",
                    distance
                ));
            }

            for _ in 0..total_len {
                let b = out[out.len() - distance];
                out.push(b);
                check_zip_bomb(out.len() as u64, compressed_size)?;
            }
        } else {
            return Err(format!("Invalid literal/length symbol: {}", sym));
        }
    }
    Ok(())
}

struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: usize,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    fn read_bits(&mut self, n: usize) -> Result<u32, String> {
        let mut val = 0;
        for i in 0..n {
            if self.byte_pos >= self.data.len() {
                return Err("Unexpected end of compressed bitstream".to_string());
            }
            let bit = (self.data[self.byte_pos] >> self.bit_pos) & 1;
            val |= (bit as u32) << i;
            self.bit_pos += 1;
            if self.bit_pos == 8 {
                self.bit_pos = 0;
                self.byte_pos += 1;
            }
        }
        Ok(val)
    }

    fn align_byte(&mut self) {
        if self.bit_pos != 0 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
    }
}

struct HuffmanTree {
    table: Vec<(u32, u32)>, // (code, length)
    symbols: Vec<u16>,
}

impl HuffmanTree {
    fn from_lengths(lengths: &[u8]) -> Self {
        let mut bl_count = [0u32; 16];
        for &l in lengths {
            if l > 0 && (l as usize) < 16 {
                bl_count[l as usize] += 1;
            }
        }

        let mut next_code = [0u32; 16];
        let mut code = 0;
        for bits in 1..16 {
            code = (code + bl_count[bits - 1]) << 1;
            next_code[bits] = code;
        }

        let mut table = Vec::new();
        let mut symbols = Vec::new();

        for (sym, &len) in lengths.iter().enumerate() {
            if len > 0 {
                let bits = len as usize;
                table.push((next_code[bits], len as u32));
                symbols.push(sym as u16);
                next_code[bits] += 1;
            }
        }

        Self { table, symbols }
    }

    fn decode(&self, r: &mut BitReader) -> Result<u16, String> {
        let mut cur_code = 0;
        let mut cur_len = 0;
        while cur_len < 16 {
            cur_code = (cur_code << 1) | r.read_bits(1)?;
            cur_len += 1;
            for (i, &(code, len)) in self.table.iter().enumerate() {
                if len == cur_len && code == cur_code {
                    return Ok(self.symbols[i]);
                }
            }
        }
        Err("Invalid Huffman code in bitstream".to_string())
    }
}

fn build_fixed_trees() -> (HuffmanTree, HuffmanTree) {
    let mut lit_lens = [0u8; 288];
    for l in lit_lens[0..=143].iter_mut() {
        *l = 8;
    }
    for l in lit_lens[144..=255].iter_mut() {
        *l = 9;
    }
    for l in lit_lens[256..=279].iter_mut() {
        *l = 7;
    }
    for l in lit_lens[280..=287].iter_mut() {
        *l = 8;
    }
    let dist_lens = [5u8; 32];
    (
        HuffmanTree::from_lengths(&lit_lens),
        HuffmanTree::from_lengths(&dist_lens),
    )
}

fn build_dynamic_trees(r: &mut BitReader) -> Result<(HuffmanTree, HuffmanTree), String> {
    let hlit = (r.read_bits(5)? + 257) as usize;
    let hdist = (r.read_bits(5)? + 1) as usize;
    let hclen = (r.read_bits(4)? + 4) as usize;

    const CL_ORDER: [usize; 19] = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut cl_lens = [0u8; 19];
    for i in 0..hclen {
        cl_lens[CL_ORDER[i]] = r.read_bits(3)? as u8;
    }
    let cl_tree = HuffmanTree::from_lengths(&cl_lens);

    let mut all_lens = Vec::with_capacity(hlit + hdist);
    while all_lens.len() < (hlit + hdist) {
        let sym = cl_tree.decode(r)?;
        if sym < 16 {
            all_lens.push(sym as u8);
        } else if sym == 16 {
            let repeat = (r.read_bits(2)? + 3) as usize;
            let last = *all_lens.last().ok_or("No previous length to repeat")?;
            all_lens.extend(std::iter::repeat_n(last, repeat));
        } else if sym == 17 {
            let repeat = (r.read_bits(3)? + 3) as usize;
            all_lens.extend(std::iter::repeat_n(0, repeat));
        } else if sym == 18 {
            let repeat = (r.read_bits(7)? + 11) as usize;
            all_lens.extend(std::iter::repeat_n(0, repeat));
        }
    }

    let lit_tree = HuffmanTree::from_lengths(&all_lens[..hlit]);
    let dist_tree = HuffmanTree::from_lengths(&all_lens[hlit..]);
    Ok((lit_tree, dist_tree))
}

fn get_length_extra(sym: u16) -> (usize, usize) {
    match sym {
        257..=264 => ((sym - 257 + 3) as usize, 0),
        265..=268 => ((sym - 265) as usize * 2 + 11, 1),
        269..=272 => ((sym - 269) as usize * 4 + 19, 2),
        273..=276 => ((sym - 273) as usize * 8 + 35, 3),
        277..=280 => ((sym - 277) as usize * 16 + 67, 4),
        281..=284 => ((sym - 281) as usize * 32 + 131, 5),
        285 => (258, 0),
        _ => (0, 0),
    }
}

fn get_dist_extra(sym: u16) -> (usize, usize) {
    if sym < 4 {
        (sym as usize + 1, 0)
    } else if sym < 30 {
        let half = (sym - 4) / 2;
        let base = 1 + (1 << (half + 1)) + ((sym % 2) as usize * (1 << half));
        (base, half as usize)
    } else {
        (1, 0)
    }
}
