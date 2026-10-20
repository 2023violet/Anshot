//! Per-user start-at-login entry (HKCU\...\Run). Nothing machine-wide, no elevation.
use std::io;
use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ};

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const NAME: &str = "ShareReady";

/// Tests point this at a throw-away key so they never touch the real Run list.
fn run_key() -> String { std::env::var("SHAREREADY_RUN_KEY").unwrap_or_else(|_| RUN.into()) }
fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() }

/// The registered command line, if start-at-login is on.
pub fn autostart() -> io::Result<Option<String>> { read(&run_key()) }
/// Register `command` to run at login, or remove the entry with `None`.
pub fn set_autostart(command: Option<&str>) -> io::Result<()> { write(&run_key(), command) }

fn read(subkey: &str) -> io::Result<Option<String>> {
    let (key, name) = (wide(subkey), wide(NAME));
    let mut buf = vec![0u16; 1024];
    let mut bytes = (buf.len() * 2) as u32;
    // SAFETY: NUL-terminated key/value names; `buf` is writable for `bytes` bytes.
    let rc = unsafe { RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buf.as_mut_ptr().cast(), &mut bytes) };
    match rc {
        ERROR_SUCCESS => {
            let len = (bytes as usize / 2).saturating_sub(1).min(buf.len());
            Ok(Some(String::from_utf16_lossy(&buf[..len])))
        }
        ERROR_FILE_NOT_FOUND => Ok(None),
        e => Err(io::Error::from_raw_os_error(e as i32)),
    }
}

fn write(subkey: &str, command: Option<&str>) -> io::Result<()> {
    let (key, name) = (wide(subkey), wide(NAME));
    let rc = match command {
        Some(cmd) => {
            let data = wide(cmd);
            // SAFETY: NUL-terminated names; `data` is a NUL-terminated UTF-16 REG_SZ of the given byte length.
            unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr(), REG_SZ, data.as_ptr().cast(), (data.len() * 2) as u32) }
        }
        // SAFETY: NUL-terminated names; deletes only our own value.
        None => match unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr()) } { ERROR_FILE_NOT_FOUND => ERROR_SUCCESS, rc => rc },
    };
    if rc == ERROR_SUCCESS { Ok(()) } else { Err(io::Error::from_raw_os_error(rc as i32)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_in_a_scratch_key() {
        let key = format!(r"Software\ShareReady-test-{}", std::process::id());
        assert_eq!(read(&key).unwrap(), None);
        write(&key, Some(r#""C:\Apps\ShareReady 截图\ShareReady.exe" --background"#)).unwrap();
        assert_eq!(read(&key).unwrap().as_deref(), Some(r#""C:\Apps\ShareReady 截图\ShareReady.exe" --background"#));
        write(&key, None).unwrap();
        write(&key, None).unwrap(); // removing twice is fine
        assert_eq!(read(&key).unwrap(), None);
        let k = wide(&key);
        // SAFETY: deletes the throw-away key created above.
        unsafe { windows_sys::Win32::System::Registry::RegDeleteTreeW(HKEY_CURRENT_USER, k.as_ptr()) };
    }
}
