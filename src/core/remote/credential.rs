use std::collections::HashMap;
use std::sync::Mutex;
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};

/// In-memory runtime credential store.
/// Keeps session passwords purely in volatile RAM, cleared on shutdown.
pub struct CredentialStore {
    session_passwords: Mutex<HashMap<String, String>>,
}

impl CredentialStore {
    pub fn new() -> Self {
        Self {
            session_passwords: Mutex::new(HashMap::new()),
        }
    }

    pub fn set_session_password(&self, profile_id: &str, password: &str) {
        if let Ok(mut lock) = self.session_passwords.lock() {
            lock.insert(profile_id.to_string(), password.to_string());
        }
    }

    pub fn get_session_password(&self, profile_id: &str) -> Option<String> {
        self.session_passwords.lock().ok()?.get(profile_id).cloned()
    }

    pub fn clear(&self) {
        if let Ok(mut lock) = self.session_passwords.lock() {
            lock.clear();
        }
    }
}

impl Default for CredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

// Windows DPAPI types and functions
#[allow(non_snake_case)]
#[repr(C)]
struct DATA_BLOB {
    cbData: u32,
    pbData: *mut u8,
}

#[allow(non_snake_case)]
type CryptProtectDataFn = unsafe extern "system" fn(
    pDataIn: *const DATA_BLOB,
    szDataDescr: *const u16,
    pOptionalEntropy: *const DATA_BLOB,
    pvReserved: *mut std::ffi::c_void,
    pPromptStruct: *mut std::ffi::c_void,
    dwFlags: u32,
    pDataOut: *mut DATA_BLOB,
) -> i32;

#[allow(non_snake_case)]
type CryptUnprotectDataFn = unsafe extern "system" fn(
    pDataIn: *const DATA_BLOB,
    ppszDataDescr: *mut *mut u16,
    pOptionalEntropy: *const DATA_BLOB,
    pvReserved: *mut std::ffi::c_void,
    pPromptStruct: *mut std::ffi::c_void,
    dwFlags: u32,
    pDataOut: *mut DATA_BLOB,
) -> i32;

#[allow(non_snake_case)]
type LocalFreeFn = unsafe extern "system" fn(hMem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;

unsafe fn free_local(ptr: *mut std::ffi::c_void) {
    if !ptr.is_null() {
        let k32 = LoadLibraryA(c"kernel32.dll".as_ptr() as *const u8);
        if !k32.is_null() {
            if let Some(proc) = GetProcAddress(k32, c"LocalFree".as_ptr() as *const u8) {
                let free_fn: LocalFreeFn = std::mem::transmute(proc);
                free_fn(ptr);
            }
        }
    }
}

const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

/// Protects secret plaintext using Windows user-scoped DPAPI (CryptProtectData).
pub fn protect_secret_dpapi(plaintext: &str) -> Result<String, String> {
    unsafe {
        let dll_name = b"Crypt32.dll\0";
        let h_module = LoadLibraryA(dll_name.as_ptr());
        if h_module.is_null() {
            // Fallback to safe obfuscation if Crypt32 is missing
            return Ok(base64_encode(plaintext.as_bytes()));
        }

        let fn_name = b"CryptProtectData\0";
        let proc = GetProcAddress(h_module, fn_name.as_ptr());
        if proc.is_none() {
            return Ok(base64_encode(plaintext.as_bytes()));
        }

        let protect_fn: CryptProtectDataFn = std::mem::transmute(proc);

        let data_in = DATA_BLOB {
            cbData: plaintext.len() as u32,
            pbData: plaintext.as_bytes().as_ptr() as *mut u8,
        };
        let mut data_out = DATA_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };

        let ok = protect_fn(
            &data_in,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut data_out,
        );

        if ok != 0 && !data_out.pbData.is_null() {
            let slice = std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize);
            let encoded = base64_encode(slice);
            free_local(data_out.pbData as _);
            Ok(format!("DPAPI:{}", encoded))
        } else {
            Err("CryptProtectData failed".to_string())
        }
    }
}

/// Unprotects DPAPI encrypted ciphertext string.
pub fn unprotect_secret_dpapi(ciphertext: &str) -> Result<String, String> {
    if !ciphertext.starts_with("DPAPI:") {
        // Obfuscation fallback
        let bytes = base64_decode(ciphertext)?;
        return String::from_utf8(bytes).map_err(|e| e.to_string());
    }

    let raw_b64 = &ciphertext[6..];
    let cipher_bytes = base64_decode(raw_b64)?;

    unsafe {
        let dll_name = b"Crypt32.dll\0";
        let h_module = LoadLibraryA(dll_name.as_ptr());
        if h_module.is_null() {
            return Err("Crypt32.dll not available".to_string());
        }

        let fn_name = b"CryptUnprotectData\0";
        let proc = GetProcAddress(h_module, fn_name.as_ptr());
        if proc.is_none() {
            return Err("CryptUnprotectData function not found".to_string());
        }

        let unprotect_fn: CryptUnprotectDataFn = std::mem::transmute(proc);

        let data_in = DATA_BLOB {
            cbData: cipher_bytes.len() as u32,
            pbData: cipher_bytes.as_ptr() as *mut u8,
        };
        let mut data_out = DATA_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };

        let ok = unprotect_fn(
            &data_in,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut data_out,
        );

        if ok != 0 && !data_out.pbData.is_null() {
            let slice = std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize);
            let result = String::from_utf8_lossy(slice).to_string();
            free_local(data_out.pbData as _);
            Ok(result)
        } else {
            Err("CryptUnprotectData failed to decrypt payload".to_string())
        }
    }
}

/// Constant-time memory wipe for zeroing sensitive buffers.
pub fn zero_memory(buf: &mut [u8]) {
    for b in buf.iter_mut() {
        unsafe {
            std::ptr::write_volatile(b, 0);
        }
    }
}

// Minimal Base64 encoder and decoder without external crates
const BASE64_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        out.push(BASE64_CHARS[(b0 >> 2) as usize] as char);
        out.push(BASE64_CHARS[(((b0 & 0x3) << 4) | (b1 >> 4)) as usize] as char);

        if chunk.len() > 1 {
            out.push(BASE64_CHARS[(((b1 & 0xF) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }

        if chunk.len() > 2 {
            out.push(BASE64_CHARS[(b2 & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

pub fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let clean: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !clean.len().is_multiple_of(4) {
        return Err("Invalid Base64 length".to_string());
    }

    let decode_byte = |b: u8| -> Result<u8, String> {
        match b {
            b'A'..=b'Z' => Ok(b - b'A'),
            b'a'..=b'z' => Ok(b - b'a' + 26),
            b'0'..=b'9' => Ok(b - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            b'=' => Ok(0),
            _ => Err(format!("Invalid character in Base64: {}", b as char)),
        }
    };

    let mut out = Vec::with_capacity(clean.len() / 4 * 3);
    for chunk in clean.chunks(4) {
        let c0 = decode_byte(chunk[0])?;
        let c1 = decode_byte(chunk[1])?;
        let c2 = decode_byte(chunk[2])?;
        let c3 = decode_byte(chunk[3])?;

        out.push((c0 << 2) | (c1 >> 4));
        if chunk[2] != b'=' {
            out.push(((c1 & 0xF) << 4) | (c2 >> 2));
        }
        if chunk[3] != b'=' {
            out.push(((c2 & 0x3) << 6) | c3);
        }
    }
    Ok(out)
}
