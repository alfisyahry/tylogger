#![allow(clippy::manual_c_str_literals)]

use std::ptr::null_mut;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress, LoadLibraryA};
use windows_sys::Win32::System::Threading::{CreateEventW, SetEvent, WaitForSingleObject};

/// Fallback synchronization primitive using native Win32 Events (Win7 SP1 compatible).
pub struct Win32SyncEvent {
    event_handle: HANDLE,
}

impl Win32SyncEvent {
    pub fn new(manual_reset: bool, initial_state: bool) -> Result<Self, String> {
        let handle = unsafe {
            CreateEventW(
                null_mut(),
                if manual_reset { 1 } else { 0 },
                if initial_state { 1 } else { 0 },
                null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            return Err("Failed to create Win32 synchronization event".to_string());
        }
        Ok(Self {
            event_handle: handle,
        })
    }

    pub fn set(&self) -> bool {
        unsafe { SetEvent(self.event_handle) != 0 }
    }

    pub fn wait(&self, timeout_ms: u32) -> bool {
        let res = unsafe { WaitForSingleObject(self.event_handle, timeout_ms) };
        res == WAIT_OBJECT_0
    }
}

impl Drop for Win32SyncEvent {
    fn drop(&mut self) {
        if !self.event_handle.is_null() && self.event_handle != INVALID_HANDLE_VALUE {
            unsafe { CloseHandle(self.event_handle) };
        }
    }
}

/// Cryptographically secure randomness provider with fallback to CryptoAPI on Windows 7.
pub fn fill_secure_random(buf: &mut [u8]) -> Result<(), String> {
    if buf.is_empty() {
        return Ok(());
    }

    // 1. Try modern BCryptGenRandom if available in bcrypt.dll
    unsafe {
        let bcrypt = GetModuleHandleA(b"bcrypt.dll\0".as_ptr());
        let bcrypt_handle = if bcrypt.is_null() {
            LoadLibraryA(b"bcrypt.dll\0".as_ptr())
        } else {
            bcrypt
        };

        if !bcrypt_handle.is_null() {
            let func = GetProcAddress(bcrypt_handle, b"BCryptGenRandom\0".as_ptr());
            if let Some(proc) = func {
                type BCryptGenRandomFn =
                    unsafe extern "system" fn(*mut std::ffi::c_void, *mut u8, u32, u32) -> i32;
                let bcrypt_gen: BCryptGenRandomFn = std::mem::transmute(proc);
                // BCRYPT_USE_SYSTEM_PREFERRED_RNG = 0x00000002
                let status = bcrypt_gen(null_mut(), buf.as_mut_ptr(), buf.len() as u32, 0x00000002);
                if status == 0 {
                    return Ok(());
                }
            }
        }
    }

    // 2. Deterministic fallback to CryptoAPI (advapi32.dll) for Windows 7 SP1
    unsafe {
        let advapi = GetModuleHandleA(b"advapi32.dll\0".as_ptr());
        let advapi_handle = if advapi.is_null() {
            LoadLibraryA(b"advapi32.dll\0".as_ptr())
        } else {
            advapi
        };

        if !advapi_handle.is_null() {
            let acq_fn = GetProcAddress(advapi_handle, b"CryptAcquireContextW\0".as_ptr());
            let gen_fn = GetProcAddress(advapi_handle, b"CryptGenRandom\0".as_ptr());
            let rel_fn = GetProcAddress(advapi_handle, b"CryptReleaseContext\0".as_ptr());

            if let (Some(acq), Some(gen), Some(rel)) = (acq_fn, gen_fn, rel_fn) {
                type CryptAcqFn =
                    unsafe extern "system" fn(*mut usize, *const u16, *const u16, u32, u32) -> i32;
                type CryptGenFn = unsafe extern "system" fn(usize, u32, *mut u8) -> i32;
                type CryptRelFn = unsafe extern "system" fn(usize, u32) -> i32;

                let crypt_acq: CryptAcqFn = std::mem::transmute(acq);
                let crypt_gen: CryptGenFn = std::mem::transmute(gen);
                let crypt_rel: CryptRelFn = std::mem::transmute(rel);

                let mut prov: usize = 0;
                // PROV_RSA_FULL = 1, CRYPT_VERIFYCONTEXT = 0xF0000000, CRYPT_SILENT = 0x00000040
                let flags = 0xF0000000u32 | 0x00000040u32;
                if crypt_acq(&mut prov, null_mut(), null_mut(), 1, flags) != 0 {
                    let ok = crypt_gen(prov, buf.len() as u32, buf.as_mut_ptr());
                    crypt_rel(prov, 0);
                    if ok != 0 {
                        return Ok(());
                    }
                }
            }
        }
    }

    Err("Failed to generate secure random bytes using bcrypt or advapi32 fallback".to_string())
}

/// Enforces safe DLL search order via `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)`
/// to mitigate DLL Search-Order Hijacking (RSK-26, AG-08, AG-28).
/// Dynamically resolves the API to remain 100% compatible with unpatched Windows 7 SP1.
pub fn enforce_safe_dll_search_order() -> bool {
    const LOAD_LIBRARY_SEARCH_SYSTEM32: u32 = 0x00000800;
    unsafe {
        let kernel32 = GetModuleHandleA(b"kernel32.dll\0".as_ptr());
        if !kernel32.is_null() {
            let func = GetProcAddress(kernel32, b"SetDefaultDllDirectories\0".as_ptr());
            if let Some(proc) = func {
                type SetDefaultDllDirectoriesFn = unsafe extern "system" fn(u32) -> i32;
                let set_dirs: SetDefaultDllDirectoriesFn = std::mem::transmute(proc);
                return set_dirs(LOAD_LIBRARY_SEARCH_SYSTEM32) != 0;
            }
        }
    }
    false
}

/// Fallback check verifying that delay-load shims resolve cleanly without hard crashing.
pub fn verify_delay_load_runtime_safety() -> bool {
    let mut entropy = [0u8; 16];
    if fill_secure_random(&mut entropy).is_err() {
        return false;
    }

    // Test Win32 event fallback
    if let Ok(ev) = Win32SyncEvent::new(false, false) {
        ev.set();
        ev.wait(100)
    } else {
        false
    }
}

/// Creates or opens the application single-instance mutex for installer detection (RSK-31, AG-40).
pub fn create_app_mutex(name: &str) -> Option<HANDLE> {
    use windows_sys::Win32::System::Threading::CreateMutexW;
    let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let handle = unsafe { CreateMutexW(null_mut(), 0, wide_name.as_ptr()) };
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        None
    } else {
        Some(handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_win32_sync_event_lifecycle() {
        let event = Win32SyncEvent::new(false, false).expect("Create event");
        assert!(!event.wait(10)); // Should timeout
        assert!(event.set());
        assert!(event.wait(100)); // Should signal
    }

    #[test]
    fn test_secure_random_entropy() {
        let mut buf1 = [0u8; 32];
        let mut buf2 = [0u8; 32];
        fill_secure_random(&mut buf1).expect("Entropy generation 1");
        fill_secure_random(&mut buf2).expect("Entropy generation 2");
        assert_ne!(buf1, [0u8; 32]);
        assert_ne!(buf1, buf2);
    }

    #[test]
    fn test_delay_load_runtime_safety() {
        assert!(verify_delay_load_runtime_safety());
    }

    #[test]
    fn test_safe_dll_search_order() {
        // Must execute cleanly without crashing on any Windows version
        let _ = enforce_safe_dll_search_order();
    }

    #[test]
    fn test_app_mutex_creation() {
        let mutex_handle = create_app_mutex("TyloggerSingleInstanceMutex_Test");
        assert!(mutex_handle.is_some());
        if let Some(h) = mutex_handle {
            unsafe { CloseHandle(h) };
        }
    }
}
