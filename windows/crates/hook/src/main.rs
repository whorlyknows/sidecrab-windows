mod host;
mod installer;
mod session;

use common::{HookEvent, RawHookPayload};
use installer::{install_hooks, is_installed, uninstall_hooks};
use session::handle_hook_event;
use std::io::{self, Read};
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::Storage::FileSystem::{GetFileType, FILE_TYPE_DISK, FILE_TYPE_PIPE};
use windows_sys::Win32::System::Console::{GetStdHandle, STD_INPUT_HANDLE};
use windows_sys::Win32::System::Pipes::PeekNamedPipe;

fn read_stdin_payload() -> RawHookPayload {
    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE);
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            return RawHookPayload::default();
        }
        let file_type = GetFileType(handle);
        match file_type {
            FILE_TYPE_DISK => {
                let mut input = String::with_capacity(4096);
                let _ = io::stdin().read_to_string(&mut input);
                let clean = input.trim_start_matches('\u{feff}').trim();
                if !clean.is_empty() {
                    serde_json::from_str(clean).unwrap_or_default()
                } else {
                    RawHookPayload::default()
                }
            }
            FILE_TYPE_PIPE => {
                let mut buffer = Vec::with_capacity(4096);
                let mut read_buf = [0u8; 8192];
                let start = std::time::Instant::now();

                // Non-blocking drain of pipe data
                loop {
                    let mut bytes_avail: u32 = 0;
                    if PeekNamedPipe(handle, null_mut(), 0, null_mut(), &mut bytes_avail, null_mut()) == 0 {
                        // Pipe broken / closed -> EOF
                        break;
                    }

                    if bytes_avail > 0 {
                        let to_read = (bytes_avail as usize).min(read_buf.len());
                        if let Ok(bytes_read) = io::stdin().read(&mut read_buf[..to_read]) {
                            if bytes_read > 0 {
                                buffer.extend_from_slice(&read_buf[..bytes_read]);
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    } else if !buffer.is_empty() {
                        // Already read some data, check once more with tiny pause in case another chunk is in transit
                        std::thread::sleep(std::time::Duration::from_millis(2));
                        let mut remaining: u32 = 0;
                        if PeekNamedPipe(handle, null_mut(), 0, null_mut(), &mut remaining, null_mut()) == 0 || remaining == 0 {
                            break;
                        }
                    } else {
                        // Pipe is empty so far, wait up to 20ms for sender to write
                        if start.elapsed() >= std::time::Duration::from_millis(20) {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                }

                if !buffer.is_empty() {
                    let text = String::from_utf8_lossy(&buffer);
                    let clean = text.trim_start_matches('\u{feff}').trim();
                    if !clean.is_empty() {
                        serde_json::from_str(clean).unwrap_or_default()
                    } else {
                        RawHookPayload::default()
                    }
                } else {
                    RawHookPayload::default()
                }
            }
            _ => RawHookPayload::default(),
        }
    }
}

fn print_help() {
    println!("sidecrab-hook: Claude Code hook CLI for Sidecrab native Windows desktop pet");
    println!();
    println!("Usage:");
    println!("  sidecrab-hook [EVENT]              Process a Claude Code lifecycle event from stdin");
    println!("  sidecrab-hook --install [PATH]     Install hooks into Claude settings.json");
    println!("  sidecrab-hook --uninstall [PATH]   Remove Sidecrab hooks from settings.json");
    println!("  sidecrab-hook --status [PATH]      Check if hooks are installed");
    println!("  sidecrab-hook --help, -h           Show this help message");
    println!("  sidecrab-hook --version, -V        Show version information");
    println!();
    println!("Supported Events:");
    println!("  prompt, pre, post, notify, permreq, stop, start, end");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        let first_arg = args[1].as_str();
        match first_arg {
            "--help" | "-h" => {
                print_help();
                return;
            }
            "--version" | "-V" => {
                println!("sidecrab-hook {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "--install" => {
                let custom_path = args.get(2).map(|s| s.as_str());
                match install_hooks(custom_path) {
                    Ok(_) => {
                        println!("Successfully installed Sidecrab hooks.");
                        return;
                    }
                    Err(e) => {
                        eprintln!("Error installing hooks: {}", e);
                        std::process::exit(2);
                    }
                }
            }
            "--uninstall" => {
                let custom_path = args.get(2).map(|s| s.as_str());
                match uninstall_hooks(custom_path) {
                    Ok(_) => {
                        println!("Successfully uninstalled Sidecrab hooks.");
                        return;
                    }
                    Err(e) => {
                        eprintln!("Error uninstalling hooks: {}", e);
                        std::process::exit(2);
                    }
                }
            }
            "--status" => {
                let custom_path = args.get(2).map(|s| s.as_str());
                if is_installed(custom_path) {
                    println!("Sidecrab hooks are installed.");
                    std::process::exit(0);
                } else {
                    println!("Sidecrab hooks are not installed.");
                    std::process::exit(1);
                }
            }
            _ => {}
        }
    }

    // Determine event from CLI argument
    let event_from_arg = args.get(1).and_then(|a| HookEvent::from_str(a));

    // Read payload from stdin if data is available (pipe or file)
    let payload: RawHookPayload = read_stdin_payload();

    // Determine target event
    let target_event = event_from_arg
        .or_else(|| {
            payload
                .hook_event_name
                .as_deref()
                .and_then(HookEvent::from_str)
        })
        .unwrap_or_else(|| {
            if payload.tool_name.is_some() {
                HookEvent::PreToolUse
            } else if payload.prompt.is_some() {
                HookEvent::UserPromptSubmit
            } else {
                HookEvent::UserPromptSubmit
            }
        });

    handle_hook_event(target_event, &payload);
}
