use common::sidecrab_dir;
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};
use std::thread;
use std::time::Duration;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Storage::FileSystem::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub const WM_SIDECRAB_STATE_CHANGED: u32 = WM_USER + 101;

#[repr(C)]
struct FileNotifyInformation {
    next_entry_offset: u32,
    action: u32,
    file_name_length: u32,
    file_name: [u16; 1],
}

/// Starts a zero-CPU background directory watcher that posts `WM_SIDECRAB_STATE_CHANGED`
/// to the pet window strictly when `state.json` is modified.
pub fn start_state_watcher(hwnd: isize) {
    thread::spawn(move || {
        let hwnd = hwnd as HWND;
        let dir = sidecrab_dir();
        let _ = std::fs::create_dir_all(&dir);

        // Initial trigger on pet startup to load current state immediately
        thread::sleep(Duration::from_millis(50));
        unsafe {
            PostMessageW(hwnd, WM_SIDECRAB_STATE_CHANGED, 0, 0);
        }

        let mut wide_dir: Vec<u16> = dir.as_os_str().encode_wide().collect();
        wide_dir.push(0);

        loop {
            unsafe {
                let h_dir = CreateFileW(
                    wide_dir.as_ptr(),
                    FILE_LIST_DIRECTORY,
                    FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                    null(),
                    OPEN_EXISTING,
                    FILE_FLAG_BACKUP_SEMANTICS,
                    null_mut(),
                );

                if h_dir == INVALID_HANDLE_VALUE {
                    thread::sleep(Duration::from_millis(500));
                    continue;
                }

                let mut buffer = [0u8; 4096];
                let mut bytes_returned: u32 = 0;

                while ReadDirectoryChangesW(
                    h_dir,
                    buffer.as_mut_ptr() as *mut _,
                    buffer.len() as u32,
                    0, // Watch subtree: false
                    FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_LAST_WRITE,
                    &mut bytes_returned,
                    null_mut(),
                    None,
                ) != 0
                {
                    if bytes_returned == 0 {
                        continue;
                    }

                    // Inspect notification records to ensure state.json was touched
                    let mut offset = 0usize;
                    let mut touches_state = false;

                    while offset < bytes_returned as usize {
                        let info = &*(buffer.as_ptr().add(offset) as *const FileNotifyInformation);
                        let name_len = (info.file_name_length / 2) as usize;
                        let name_slice = std::slice::from_raw_parts(
                            info.file_name.as_ptr(),
                            name_len,
                        );
                        let file_name = String::from_utf16_lossy(name_slice);

                        if file_name.eq_ignore_ascii_case("state.json") {
                            touches_state = true;
                            break;
                        }

                        if info.next_entry_offset == 0 {
                            break;
                        }
                        offset += info.next_entry_offset as usize;
                    }

                    if touches_state {
                        // Debounce rapid bursts from atomic tmp + rename
                        thread::sleep(Duration::from_millis(60));
                        PostMessageW(hwnd, WM_SIDECRAB_STATE_CHANGED, 0, 0);
                    }
                }

                CloseHandle(h_dir);
            }
            thread::sleep(Duration::from_millis(200));
        }
    });
}
