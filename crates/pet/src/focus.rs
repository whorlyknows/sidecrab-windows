use common::{state_path, StateJson};
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Activates and brings the target window to the foreground, bypassing OS foreground lock restrictions.
pub unsafe fn bring_to_foreground(target_hwnd: HWND, target_pid: u32) {
    if target_hwnd.is_null() || IsWindow(target_hwnd) == 0 {
        return;
    }

    if target_pid != 0 {
        AllowSetForegroundWindow(target_pid);
    }

    let foreground_hwnd = GetForegroundWindow();
    let foreground_thread = GetWindowThreadProcessId(foreground_hwnd, null_mut());
    let current_thread = GetCurrentThreadId();

    if foreground_thread != current_thread && foreground_thread != 0 {
        AttachThreadInput(current_thread, foreground_thread, 1);
    }

    if IsIconic(target_hwnd) != 0 {
        ShowWindow(target_hwnd, SW_RESTORE);
    } else {
        ShowWindow(target_hwnd, SW_SHOW);
    }

    BringWindowToTop(target_hwnd);
    SetForegroundWindow(target_hwnd);

    if foreground_thread != current_thread && foreground_thread != 0 {
        AttachThreadInput(current_thread, foreground_thread, 0);
    }
}

/// Discovers the Claude Code host terminal/editor window and brings it to the foreground.
pub unsafe fn focus_host_window() {
    // 1. Try reading host_hwnd and host_pid from state.json
    let path = state_path();
    if let Ok(data) = std::fs::read_to_string(&path) {
        let clean = data.trim_start_matches('\u{feff}').trim();
        if let Ok(state) = serde_json::from_str::<StateJson>(clean) {
            if let Some(hwnd_val) = state.host_hwnd {
                let hwnd = hwnd_val as usize as HWND;
                if !hwnd.is_null() && IsWindow(hwnd) != 0 && IsWindowVisible(hwnd) != 0 {
                    bring_to_foreground(hwnd, state.host_pid.unwrap_or(0));
                    return;
                }
            }
        }
    }

    // 2. Fallback: Enumerate top-level windows for terminal / editor candidates
    let mut candidate: HWND = null_mut();
    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }

        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        if (style & WS_CHILD) != 0 {
            return 1;
        }

        let mut title_buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), 512);
        if len > 0 {
            let title = String::from_utf16_lossy(&title_buf[..len as usize]).to_lowercase();
            if title.contains("claude")
                || title.contains("windows terminal")
                || title.contains("visual studio code")
                || title.contains("powershell")
                || title.contains("cmd.exe")
            {
                *(lparam as *mut HWND) = hwnd;
                return 0; // stop enumeration
            }
        }
        1
    }

    EnumWindows(Some(enum_proc), &mut candidate as *mut _ as LPARAM);
    if !candidate.is_null() {
        bring_to_foreground(candidate, 0);
    }
}
