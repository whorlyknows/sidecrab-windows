use std::collections::HashMap;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Discovers the host terminal or editor's process name, PID, and top-level HWND.
pub unsafe fn detect_host_terminal() -> (Option<String>, u32, u64) {
    let current_pid = GetCurrentProcessId();
    let h_snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if h_snap == INVALID_HANDLE_VALUE {
        return (None, 0, 0);
    }

    let mut pe = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        cntUsage: 0,
        th32ProcessID: 0,
        th32DefaultHeapID: 0,
        th32ModuleID: 0,
        cntThreads: 0,
        th32ParentProcessID: 0,
        pcPriClassBase: 0,
        dwFlags: 0,
        szExeFile: [0u16; 260],
    };

    // Fast integer map: pid -> (ppid, [u16; 32]) with zero heap string allocations
    let mut tree: HashMap<u32, (u32, [u16; 32])> = HashMap::with_capacity(256);

    if Process32FirstW(h_snap, &mut pe) != 0 {
        loop {
            let mut name_buf = [0u16; 32];
            for (dest, &src) in name_buf.iter_mut().zip(pe.szExeFile.iter()) {
                if src == 0 {
                    break;
                }
                *dest = (src as u8 as char).to_ascii_lowercase() as u16;
            }
            tree.insert(pe.th32ProcessID, (pe.th32ParentProcessID, name_buf));

            if Process32NextW(h_snap, &mut pe) == 0 {
                break;
            }
        }
    }
    CloseHandle(h_snap);

    let mut check_pid = current_pid;
    let mut host_pid = 0;
    let mut host_name: Option<String> = None;
    let mut candidate_pids = Vec::with_capacity(8);
    let mut depth = 0;

    while depth < 32 {
        depth += 1;
        if let Some(&(ppid, ref name_raw)) = tree.get(&check_pid) {
            let end = name_raw.iter().position(|&c| c == 0).unwrap_or(name_raw.len());
            let name_str = String::from_utf16_lossy(&name_raw[..end]);

            if host_pid == 0
                && (name_str.contains("windowsterminal")
                    || name_str.contains("code")
                    || name_str.contains("cursor")
                    || name_str.contains("alacritty")
                    || name_str.contains("wezterm")
                    || name_str.contains("powershell")
                    || name_str.contains("cmd")
                    || name_str.contains("conhost"))
            {
                host_pid = check_pid;
                host_name = Some(name_str);
            }

            if host_pid != 0 && !candidate_pids.contains(&check_pid) {
                candidate_pids.push(check_pid);
            }

            if ppid == 0 || ppid == check_pid {
                break;
            }
            check_pid = ppid;
        } else {
            break;
        }
    }

    if host_pid == 0 {
        if let Some(&(ppid, _)) = tree.get(&current_pid) {
            host_pid = ppid;
            if let Some((_, parent_name)) = tree.get(&ppid) {
                let end = parent_name.iter().position(|&c| c == 0).unwrap_or(parent_name.len());
                host_name = Some(String::from_utf16_lossy(&parent_name[..end]));
            }
            if !candidate_pids.contains(&ppid) {
                candidate_pids.push(ppid);
            }
        }
    }

    // Single-pass EnumWindows for all candidate PIDs in the parent chain
    let mut hwnd_found: HWND = null_mut();
    if !candidate_pids.is_empty() {
        struct EnumData<'a> {
            candidate_pids: &'a [u32],
            hwnd: HWND,
            count: u32,
        }

        unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let data = &mut *(lparam as *mut EnumData);
            data.count += 1;
            if data.count > 256 {
                return 0; // Guard against slow/infinite window enumerations
            }

            let mut win_pid = 0;
            GetWindowThreadProcessId(hwnd, &mut win_pid);
            if data.candidate_pids.contains(&win_pid) && IsWindowVisible(hwnd) != 0 {
                let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
                if (style & WS_CHILD) == 0 {
                    data.hwnd = hwnd;
                    return 0; // Found, stop enumeration immediately!
                }
            }
            1
        }

        let mut data = EnumData {
            candidate_pids: &candidate_pids,
            hwnd: null_mut(),
            count: 0,
        };
        EnumWindows(Some(enum_proc), &mut data as *mut _ as LPARAM);
        hwnd_found = data.hwnd;
    }

    let hwnd_u64 = if !hwnd_found.is_null() {
        hwnd_found as usize as u64
    } else {
        0
    };

    (host_name, host_pid, hwnd_u64)
}
