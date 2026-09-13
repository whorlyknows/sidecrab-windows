use std::ptr::{null, null_mut};
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows_sys::Win32::System::Registry::*;

const SUBKEY: &[u16] = &[
    b'S' as u16, b'o' as u16, b'f' as u16, b't' as u16, b'w' as u16, b'a' as u16, b'r' as u16,
    b'e' as u16, b'\\' as u16, b'M' as u16, b'i' as u16, b'c' as u16, b'r' as u16, b'o' as u16,
    b's' as u16, b'o' as u16, b'f' as u16, b't' as u16, b'\\' as u16, b'W' as u16, b'i' as u16,
    b'n' as u16, b'd' as u16, b'o' as u16, b'w' as u16, b's' as u16, b'\\' as u16, b'C' as u16,
    b'u' as u16, b'r' as u16, b'r' as u16, b'e' as u16, b'n' as u16, b't' as u16, b'V' as u16,
    b'e' as u16, b'r' as u16, b's' as u16, b'i' as u16, b'o' as u16, b'n' as u16, b'\\' as u16,
    b'R' as u16, b'u' as u16, b'n' as u16, 0,
];

const VALUE_NAME: &[u16] = &[
    b'S' as u16, b'i' as u16, b'd' as u16, b'e' as u16, b'c' as u16, b'r' as u16, b'a' as u16,
    b'b' as u16, 0,
];

/// Checks whether autostart is currently enabled in HKCU\...\Run.
pub unsafe fn is_autostart_enabled() -> bool {
    let mut hkey: HKEY = null_mut();
    if RegOpenKeyExW(
        HKEY_CURRENT_USER,
        SUBKEY.as_ptr(),
        0,
        KEY_QUERY_VALUE,
        &mut hkey,
    ) != 0
    {
        return false;
    }

    let mut data_type: u32 = 0;
    let mut data_len: u32 = 0;
    let res = RegQueryValueExW(
        hkey,
        VALUE_NAME.as_ptr(),
        null(),
        &mut data_type,
        null_mut(),
        &mut data_len,
    );
    RegCloseKey(hkey);
    res == 0
}

/// Enables or disables autostart in HKCU\...\Run.
pub unsafe fn set_autostart(enable: bool) -> bool {
    let mut hkey: HKEY = null_mut();
    if RegOpenKeyExW(
        HKEY_CURRENT_USER,
        SUBKEY.as_ptr(),
        0,
        KEY_SET_VALUE | KEY_QUERY_VALUE,
        &mut hkey,
    ) != 0
    {
        return false;
    }

    let success = if enable {
        let mut exe_path = [0u16; 1024];
        let len = GetModuleFileNameW(null_mut(), exe_path.as_mut_ptr(), 1024);
        if len > 0 {
            // Quote the executable path
            let mut quoted = Vec::with_capacity((len + 3) as usize);
            quoted.push(b'"' as u16);
            quoted.extend_from_slice(&exe_path[..len as usize]);
            quoted.push(b'"' as u16);
            quoted.push(0);

            let byte_len = (quoted.len() * 2) as u32;
            RegSetValueExW(
                hkey,
                VALUE_NAME.as_ptr(),
                0,
                REG_SZ,
                quoted.as_ptr() as *const u8,
                byte_len,
            ) == 0
        } else {
            false
        }
    } else {
        RegDeleteValueW(hkey, VALUE_NAME.as_ptr()) == 0
    };

    RegCloseKey(hkey);
    success
}
