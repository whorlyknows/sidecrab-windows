use common::{HatType, WindowSize};
use std::ptr::null;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub const IDM_SIZE_S: usize = 101;
pub const IDM_SIZE_M: usize = 102;
pub const IDM_SIZE_L: usize = 103;

pub const IDM_CORNER_TL: usize = 201;
pub const IDM_CORNER_TR: usize = 202;
pub const IDM_CORNER_BL: usize = 203;
pub const IDM_CORNER_BR: usize = 204;
pub const IDM_CORNER_RESET: usize = 205;

pub const IDM_HAT_NONE: usize = 300;
pub const IDM_HAT_TOP: usize = 301;
pub const IDM_HAT_CHEF: usize = 302;
pub const IDM_HAT_FEDORA: usize = 303;
pub const IDM_HAT_HELI: usize = 304;

pub const IDM_WANDER: usize = 401;
pub const IDM_AUTOSTART: usize = 402;
pub const IDM_HOOKS_INSTALL: usize = 403;
pub const IDM_HOOKS_REMOVE: usize = 404;
pub const IDM_WANDER_NOW: usize = 405;
pub const IDM_RESET_STATE: usize = 406;
pub const IDM_FLIGHT_EASTER_EGG: usize = 407;
pub const IDM_DROP_SNACK: usize = 408;

pub const IDM_TEST_REST: usize = 501;
pub const IDM_TEST_THINK: usize = 502;
pub const IDM_TEST_WORK: usize = 503;
pub const IDM_TEST_ALERT: usize = 504;
pub const IDM_TEST_CELEBRATE: usize = 505;
pub const IDM_TEST_WALK: usize = 506;
pub const IDM_TEST_STRETCH: usize = 507;
pub const IDM_TEST_BLINK: usize = 508;
pub const IDM_TEST_PEEK: usize = 509;
pub const IDM_TEST_LOOK: usize = 510;
pub const IDM_TEST_WAVE: usize = 511;
pub const IDM_TEST_SHUFFLE: usize = 512;
pub const IDM_TEST_SLEEP: usize = 513;
pub const IDM_TEST_PANIC: usize = 514;
pub const IDM_TEST_GLARE: usize = 515;
pub const IDM_TEST_CHASE: usize = 516;
pub const IDM_TEST_RELAX: usize = 517;
pub const IDM_TEST_RESET: usize = 599;

pub const IDM_EXIT: usize = 499;

fn wide_str(s: &str) -> Vec<u16> {
    let mut v: Vec<u16> = s.encode_utf16().collect();
    v.push(0);
    v
}

/// Displays the native Win32 context popup menu and returns the selected command ID (or 0 if dismissed).
pub unsafe fn show_context_menu(
    hwnd: HWND,
    x: i32,
    y: i32,
    current_size: WindowSize,
    current_hat: HatType,
    wander_enabled: bool,
    autostart_enabled: bool,
    hooks_installed: bool,
) -> usize {
    let menu = CreatePopupMenu();
    let size_menu = CreatePopupMenu();
    let corner_menu = CreatePopupMenu();
    let hat_menu = CreatePopupMenu();
    let test_menu = CreatePopupMenu();

    // Size Submenu
    let s_flag = if current_size == WindowSize::Small { MF_CHECKED } else { MF_UNCHECKED };
    let m_flag = if current_size == WindowSize::Medium { MF_CHECKED } else { MF_UNCHECKED };
    let l_flag = if current_size == WindowSize::Large { MF_CHECKED } else { MF_UNCHECKED };
    AppendMenuW(size_menu, MF_STRING | s_flag, IDM_SIZE_S, wide_str("Small (102x96)").as_ptr());
    AppendMenuW(size_menu, MF_STRING | m_flag, IDM_SIZE_M, wide_str("Medium (153x144)").as_ptr());
    AppendMenuW(size_menu, MF_STRING | l_flag, IDM_SIZE_L, wide_str("Large (204x192)").as_ptr());
    AppendMenuW(menu, MF_POPUP, size_menu as usize, wide_str("Size").as_ptr());

    // Corner Presets Submenu
    AppendMenuW(corner_menu, MF_STRING, IDM_CORNER_TL, wide_str("Top-Left").as_ptr());
    AppendMenuW(corner_menu, MF_STRING, IDM_CORNER_TR, wide_str("Top-Right").as_ptr());
    AppendMenuW(corner_menu, MF_STRING, IDM_CORNER_BL, wide_str("Bottom-Left").as_ptr());
    AppendMenuW(corner_menu, MF_STRING, IDM_CORNER_BR, wide_str("Bottom-Right").as_ptr());
    AppendMenuW(corner_menu, MF_SEPARATOR, 0, null());
    AppendMenuW(corner_menu, MF_STRING, IDM_CORNER_RESET, wide_str("Reset to Default").as_ptr());
    AppendMenuW(menu, MF_POPUP, corner_menu as usize, wide_str("Corner Presets").as_ptr());

    // Hats Submenu
    let h_none = if current_hat == HatType::None { MF_CHECKED } else { MF_UNCHECKED };
    let h_top = if current_hat == HatType::Top { MF_CHECKED } else { MF_UNCHECKED };
    let h_chef = if current_hat == HatType::Chef { MF_CHECKED } else { MF_UNCHECKED };
    let h_fedora = if current_hat == HatType::Fedora { MF_CHECKED } else { MF_UNCHECKED };
    let h_heli = if current_hat == HatType::Helicopter { MF_CHECKED } else { MF_UNCHECKED };
    AppendMenuW(hat_menu, MF_STRING | h_none, IDM_HAT_NONE, wide_str("None").as_ptr());
    AppendMenuW(hat_menu, MF_STRING | h_top, IDM_HAT_TOP, wide_str("Top Hat").as_ptr());
    AppendMenuW(hat_menu, MF_STRING | h_chef, IDM_HAT_CHEF, wide_str("Chef's Hat").as_ptr());
    AppendMenuW(hat_menu, MF_STRING | h_fedora, IDM_HAT_FEDORA, wide_str("Fedora").as_ptr());
    AppendMenuW(hat_menu, MF_STRING | h_heli, IDM_HAT_HELI, wide_str("Helicopter Hat").as_ptr());
    AppendMenuW(menu, MF_POPUP, hat_menu as usize, wide_str("Hats").as_ptr());

    // Test Animations Submenu
    AppendMenuW(test_menu, MF_STRING, IDM_TEST_REST, wide_str("▶ Idle / Rest Pose").as_ptr());
    AppendMenuW(test_menu, MF_STRING, IDM_TEST_RELAX, wide_str("▶ Sit & Relax (Legs Out, 8s Inverse)").as_ptr());
    AppendMenuW(test_menu, MF_STRING, IDM_TEST_THINK, wide_str("▶ Thinking (Thought Bubble)").as_ptr());
    AppendMenuW(test_menu, MF_STRING, IDM_TEST_WORK, wide_str("▶ Working (Laptop Typing)").as_ptr());
    AppendMenuW(test_menu, MF_STRING, IDM_TEST_ALERT, wide_str("▶ Alert (Double Claw Wave)").as_ptr());
    AppendMenuW(test_menu, MF_STRING, IDM_TEST_CELEBRATE, wide_str("▶ Celebrate (Happy Hop)").as_ptr());
    AppendMenuW(test_menu, MF_STRING, IDM_TEST_WALK, wide_str("▶ Walk Cycle").as_ptr());
    AppendMenuW(test_menu, MF_SEPARATOR, 0, null());
    AppendMenuW(test_menu, MF_STRING, IDM_TEST_STRETCH, wide_str("Morning Stretch").as_ptr());
    AppendMenuW(test_menu, MF_STRING, IDM_TEST_BLINK, wide_str("Blink").as_ptr());
    AppendMenuW(test_menu, MF_STRING, IDM_TEST_PEEK, wide_str("▶ Peek Left & Right (True Turn)").as_ptr());
    AppendMenuW(test_menu, MF_STRING, IDM_TEST_LOOK, wide_str("Look Around").as_ptr());
    AppendMenuW(test_menu, MF_STRING, IDM_TEST_WAVE, wide_str("Wave").as_ptr());
    AppendMenuW(test_menu, MF_STRING, IDM_TEST_SHUFFLE, wide_str("Shuffle").as_ptr());
    AppendMenuW(test_menu, MF_STRING, IDM_TEST_SLEEP, wide_str("Sleep (Deep zZz)").as_ptr());
    AppendMenuW(test_menu, MF_STRING, IDM_TEST_PANIC, wide_str("Panic (Held/Carried)").as_ptr());
    AppendMenuW(test_menu, MF_STRING, IDM_TEST_GLARE, wide_str("Glare").as_ptr());
    AppendMenuW(test_menu, MF_STRING, IDM_TEST_CHASE, wide_str("Chase Cursor").as_ptr());
    AppendMenuW(test_menu, MF_SEPARATOR, 0, null());
    AppendMenuW(test_menu, MF_STRING, IDM_TEST_RESET, wide_str("🔄 Resume Live Claude State").as_ptr());
    AppendMenuW(menu, MF_POPUP, test_menu as usize, wide_str("🎭 Test Animations").as_ptr());

    // Interactive & Physics Actions
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    AppendMenuW(menu, MF_STRING, IDM_DROP_SNACK, wide_str("🍰 Drop Snack Treat").as_ptr());
    AppendMenuW(menu, MF_STRING, IDM_FLIGHT_EASTER_EGG, wide_str("🚀 Fly & Bounce").as_ptr());
    AppendMenuW(menu, MF_STRING, IDM_WANDER_NOW, wide_str("🐾 Take a Walk Now").as_ptr());
    AppendMenuW(menu, MF_STRING, IDM_RESET_STATE, wide_str("🔄 Reset State to Idle").as_ptr());

    // Toggles
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    let wander_flag = if wander_enabled { MF_CHECKED } else { MF_UNCHECKED };
    AppendMenuW(menu, MF_STRING | wander_flag, IDM_WANDER, wide_str("Wander Mode").as_ptr());

    let autostart_flag = if autostart_enabled { MF_CHECKED } else { MF_UNCHECKED };
    AppendMenuW(menu, MF_STRING | autostart_flag, IDM_AUTOSTART, wide_str("Launch at Login").as_ptr());

    // Hook Management
    if hooks_installed {
        AppendMenuW(menu, MF_STRING, IDM_HOOKS_REMOVE, wide_str("Remove Claude Code Hooks").as_ptr());
    } else {
        AppendMenuW(menu, MF_STRING, IDM_HOOKS_INSTALL, wide_str("Install Claude Code Hooks").as_ptr());
    }

    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    AppendMenuW(menu, MF_STRING, IDM_EXIT, wide_str("Exit").as_ptr());

    // Windows menu dismissal idiom
    SetForegroundWindow(hwnd);
    let cmd = TrackPopupMenuEx(
        menu,
        TPM_RIGHTBUTTON | TPM_RETURNCMD | TPM_NONOTIFY,
        x,
        y,
        hwnd,
        null(),
    );
    PostMessageW(hwnd, WM_NULL, 0, 0);

    DestroyMenu(menu);

    cmd as usize
}
