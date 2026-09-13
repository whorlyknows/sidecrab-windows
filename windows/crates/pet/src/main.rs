#![windows_subsystem = "windows"]

mod anim;
mod assets;
mod autostart;
mod config;
mod dib;
mod focus;
mod ipc;
mod menu;
mod physics;
mod wander;
mod window;

use std::ptr::{null, null_mut};
use std::time::Instant;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use assets::{CANVAS_H, CANVAS_W};
use config::load_config;
use dib::LayeredFrameBuffer;
use ipc::start_state_watcher;
use physics::PhysicsController;
use wander::WanderController;
use window::*;

const MUTEX_NAME: &[u16] = &[
    b'S' as u16, b'i' as u16, b'd' as u16, b'e' as u16, b'c' as u16, b'r' as u16,
    b'a' as u16, b'b' as u16, b'P' as u16, b'e' as u16, b't' as u16, b'M' as u16,
    b'u' as u16, b't' as u16, b'e' as u16, b'x' as u16, 0,
];

fn main() {
    unsafe {
        // 1. Single instance check
        let h_mutex = CreateMutexW(null(), 0, MUTEX_NAME.as_ptr());
        if !h_mutex.is_null() && GetLastError() == ERROR_ALREADY_EXISTS {
            let existing_hwnd = FindWindowW(PET_CLASS_NAME.as_ptr(), null());
            if !existing_hwnd.is_null() {
                SetForegroundWindow(existing_hwnd);
            }
            CloseHandle(h_mutex);
            return;
        }

        // 2. Load persisted configuration
        let cfg = load_config();
        let width = cfg.size.width();
        let height = cfg.size.height();
        let scale = cfg.size.scale();

        let hinstance = GetModuleHandleW(null());
        register_pet_class(hinstance);

        // 3. Create layered window
        let hwnd = create_pet_window(hinstance, cfg.x, cfg.y, width, height);
        if hwnd.is_null() {
            return;
        }

        // 4. Initialize framebuffer, animation state, wander, and physics
        let framebuffer = LayeredFrameBuffer::new(width, height, scale);
        let mut animator = anim::AnimationController::new();
        animator.hat = cfg.hat;

        let mut wanderer = WanderController::new(cfg.x, cfg.y);
        wanderer.enabled = cfg.wander_enabled;

        let physics = PhysicsController::new(cfg.x, cfg.y);
        let now = Instant::now();

        let state = Box::new(AppState {
            hwnd,
            x: cfg.x,
            y: cfg.y,
            size: cfg.size,
            framebuffer,
            logical_buffer: [[0; 4]; CANVAS_W * CANVAS_H],
            animator,
            wanderer,
            physics,
            last_tick: now,
            last_trim: now - std::time::Duration::from_secs(10),
            is_active_fps: true,
            is_dragging: false,
            drag_start_cursor: POINT { x: 0, y: 0 },
            drag_start_win: POINT { x: cfg.x, y: cfg.y },
            last_mouse_pos: POINT { x: 0, y: 0 },
            last_mouse_time: now,
            mouse_velocity: (0.0, 0.0),
            last_hook_time: now,
        });

        let state_raw = Box::into_raw(state);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_raw as isize);

        // 5. Initial render and presentation
        (*state_raw).render_and_present();
        SetTimer(hwnd, TIMER_ANIM_ID, 50, None); // 20 FPS smooth active timer

        // 6. Start background IPC watcher
        start_state_watcher(hwnd as isize);

        // 7. Aggressively trim initial working set
        (*state_raw).trim_memory();

        // 8. Low-power blocking message loop (0.00% CPU when waiting)
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // Cleanup
        let state = Box::from_raw(state_raw);
        drop(state);

        if !h_mutex.is_null() {
            CloseHandle(h_mutex);
        }
    }
}
