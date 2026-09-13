use common::{
    state_path, HatType, MascotState, PetConfig, StateJson, WindowSize,
};
use std::ptr::{null, null_mut};
use std::time::Instant;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::ProcessStatus::EmptyWorkingSet;
use windows_sys::Win32::System::Threading::GetCurrentProcess;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetCapture, ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::anim::{AnimId, AnimationController};
use crate::assets::{composite_logical_frame, CANVAS_H, CANVAS_W};
use crate::autostart::{is_autostart_enabled, set_autostart};
use crate::config::save_config;
use crate::dib::LayeredFrameBuffer;
use crate::focus::focus_host_window;
use crate::ipc::WM_SIDECRAB_STATE_CHANGED;
use crate::menu::*;
use crate::physics::PhysicsController;
use crate::wander::WanderController;

pub const TIMER_ANIM_ID: usize = 1;
pub const WM_MOUSELEAVE: u32 = 0x02A3;
pub const PET_CLASS_NAME: &[u16] = &[
    b'S' as u16, b'i' as u16, b'd' as u16, b'e' as u16, b'c' as u16, b'r' as u16,
    b'a' as u16, b'b' as u16, b'P' as u16, b'e' as u16, b't' as u16, b'W' as u16,
    b'i' as u16, b'n' as u16, b'd' as u16, b'o' as u16, b'w' as u16, 0,
];

pub struct AppState {
    pub hwnd: HWND,
    pub x: i32,
    pub y: i32,
    pub size: WindowSize,
    pub framebuffer: LayeredFrameBuffer,
    pub logical_buffer: [[u8; 4]; CANVAS_W * CANVAS_H],
    pub animator: AnimationController,
    pub wanderer: WanderController,
    pub physics: PhysicsController,
    pub last_tick: Instant,
    pub last_trim: Instant,
    pub is_active_fps: bool,

    // Smooth drag and fling physics tracking
    pub is_dragging: bool,
    pub drag_start_cursor: POINT,
    pub drag_start_win: POINT,
    pub last_mouse_pos: POINT,
    pub last_mouse_time: Instant,
    pub mouse_velocity: (f32, f32),

    // Stale state watchdog
    pub last_hook_time: Instant,
}

impl AppState {
    pub unsafe fn trim_memory(&self) {
        EmptyWorkingSet(GetCurrentProcess());
    }

    pub fn save_current_config(&self) {
        let cfg = PetConfig {
            x: self.x,
            y: self.y,
            size: self.size,
            hat: self.animator.hat,
            wander_enabled: self.wanderer.enabled,
            autostart: unsafe { is_autostart_enabled() },
        };
        save_config(&cfg);
    }

    pub unsafe fn set_fps_profile(&mut self, high_fps: bool) {
        if self.is_active_fps == high_fps {
            return;
        }
        self.is_active_fps = high_fps;
        let interval = if high_fps { 50 } else { 1000 };
        SetTimer(self.hwnd, TIMER_ANIM_ID, interval, None);
    }

    pub unsafe fn render_and_present(&mut self) {
        let (step, ambient_blink, thought_phase) = self.animator.current_render_step();

        composite_logical_frame(
            &mut self.logical_buffer,
            step.frame_idx,
            step.dy,
            self.animator.facing,
            step.blink || ambient_blink,
            step.squint,
            step.half_eyes,
            step.eyes_dx,
            self.animator.hat,
            self.animator.total_ms(),
            thought_phase,
            step.mark,
            step.zzz,
        );

        self.framebuffer.blit_from_logical(&self.logical_buffer);
        self.framebuffer.present(self.hwnd, self.x, self.y);
    }
}

pub unsafe fn get_desktop_work_area() -> RECT {
    let mut rc = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut rc as *mut _ as *mut _, 0);
    rc
}

pub unsafe fn register_pet_class(hinstance: HMODULE) -> u16 {
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_DBLCLKS | CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(pet_wndproc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: null_mut(),
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        hbrBackground: null_mut(),
        lpszMenuName: null(),
        lpszClassName: PET_CLASS_NAME.as_ptr(),
        hIconSm: null_mut(),
    };
    RegisterClassExW(&wc)
}

pub unsafe fn create_pet_window(hinstance: HMODULE, x: i32, y: i32, width: i32, height: i32) -> HWND {
    CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        PET_CLASS_NAME.as_ptr(),
        null(),
        WS_POPUP | WS_VISIBLE,
        x,
        y,
        width,
        height,
        null_mut(),
        null_mut(),
        hinstance,
        null(),
    )
}

pub unsafe extern "system" fn pet_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;

    match msg {
        WM_NCHITTEST => {
            if state_ptr.is_null() {
                return HTCLIENT as isize;
            }
            let state = &*state_ptr;

            if state.is_dragging {
                return HTCLIENT as isize;
            }

            let screen_x = (lparam as i32 & 0xFFFF) as i16 as i32;
            let screen_y = ((lparam as i32 >> 16) & 0xFFFF) as i16 as i32;

            let local_x = screen_x - state.x;
            let local_y = screen_y - state.y;

            if state.framebuffer.is_pixel_opaque(local_x, local_y) {
                HTCLIENT as isize
            } else {
                HTTRANSPARENT as isize
            }
        }

        WM_MOUSEMOVE => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;

                if state.is_dragging {
                    let mut pt = POINT { x: 0, y: 0 };
                    GetCursorPos(&mut pt);

                    let dx = pt.x - state.drag_start_cursor.x;
                    let dy = pt.y - state.drag_start_cursor.y;
                    state.x = state.drag_start_win.x + dx;
                    state.y = state.drag_start_win.y + dy;

                    // Calculate fling velocity
                    let now = Instant::now();
                    let dt = now.duration_since(state.last_mouse_time).as_secs_f32().max(0.001);
                    let vx = (pt.x - state.last_mouse_pos.x) as f32 / dt;
                    let vy = (pt.y - state.last_mouse_pos.y) as f32 / dt;

                    // Exponential moving average for velocity
                    state.mouse_velocity.0 = state.mouse_velocity.0 * 0.5 + (vx / 60.0) * 0.5;
                    state.mouse_velocity.1 = state.mouse_velocity.1 * 0.5 + (vy / 60.0) * 0.5;
                    state.last_mouse_pos = pt;
                    state.last_mouse_time = now;

                    SetWindowPos(
                        hwnd,
                        0,
                        state.x,
                        state.y,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                    state.render_and_present();
                    return 0;
                }

                let mut tme = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                TrackMouseEvent(&mut tme);

                if !state.animator.hovering && !state.physics.is_active() {
                    state.animator.set_hover(true);
                    state.set_fps_profile(true);
                    state.render_and_present();
                }
            }
            0
        }

        WM_MOUSELEAVE => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if state.animator.hovering && !state.is_dragging {
                    state.animator.set_hover(false);
                    state.render_and_present();
                }
            }
            0
        }

        WM_LBUTTONDOWN => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                state.wanderer.mark_user_activity();

                // If currently flying or wandering, interrupt
                if state.physics.is_active() {
                    state.physics.mode = crate::physics::PhysicsMode::Inactive;
                }

                let mut pt = POINT { x: 0, y: 0 };
                GetCursorPos(&mut pt);

                state.is_dragging = true;
                state.drag_start_cursor = pt;
                state.drag_start_win = POINT { x: state.x, y: state.y };
                state.last_mouse_pos = pt;
                state.last_mouse_time = Instant::now();
                state.mouse_velocity = (0.0, 0.0);

                SetCapture(hwnd);
                state.animator.play(AnimId::Panic); // Cute held/carried pose while dragging!
                state.set_fps_profile(true);
                state.render_and_present();
            }
            0
        }

        WM_LBUTTONUP => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if state.is_dragging {
                    ReleaseCapture();
                    state.is_dragging = false;

                    let speed = (state.mouse_velocity.0 * state.mouse_velocity.0
                        + state.mouse_velocity.1 * state.mouse_velocity.1)
                        .sqrt();

                    if speed > 5.0 {
                        // Flung / tossed by user! Launch flight physics
                        state.physics.start_fling(
                            state.x,
                            state.y,
                            state.mouse_velocity.0,
                            state.mouse_velocity.1,
                            state.animator.hat,
                        );
                        state.set_fps_profile(true);
                    } else {
                        // Placed down gently -> save as new home
                        state.wanderer.set_home(state.x, state.y);
                        state.physics.set_home(state.x, state.y);
                        state.save_current_config();

                        state.animator.pet();
                        state.render_and_present();
                    }
                }
            }
            0
        }

        WM_LBUTTONDBLCLK => {
            focus_host_window();
            0
        }

        WM_RBUTTONUP | WM_CONTEXTMENU => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if state.is_dragging {
                    ReleaseCapture();
                    state.is_dragging = false;
                }
                state.wanderer.mark_user_activity();

                let mut pt = POINT { x: 0, y: 0 };
                GetCursorPos(&mut pt);

                let autostart = is_autostart_enabled();
                let hooks_installed = common::claude_settings_path().exists();

                let cmd = show_context_menu(
                    hwnd,
                    pt.x,
                    pt.y,
                    state.size,
                    state.animator.hat,
                    state.wanderer.enabled,
                    autostart,
                    hooks_installed,
                );

                handle_context_command(state, cmd);
            }
            0
        }

        WM_TIMER => {
            if !state_ptr.is_null() && wparam == TIMER_ANIM_ID {
                let state = &mut *state_ptr;
                let now = Instant::now();
                let dt = now.duration_since(state.last_tick);
                state.last_tick = now;

                let work_area = get_desktop_work_area();

                // 1. Stale state watchdog: if tool or thinking state is older than 15s without hook activity, auto-reset to idle
                if (state.animator.feed_state == MascotState::Tool
                    || state.animator.feed_state == MascotState::Working
                    || state.animator.feed_state == MascotState::Thinking)
                    && state.animator.test_override.is_none()
                    && !state.is_dragging
                    && !state.physics.is_active()
                {
                    if state.last_hook_time.elapsed().as_secs() >= 15 {
                        state.animator.apply_feed_state(MascotState::Idle);
                    }
                }

                // 2. Physics Controller (VS Code Pet Flight & Fling Mode)
                if state.physics.is_active() {
                    if let Some((nx, ny, facing, anim, hat_override)) = state.physics.update(
                        &work_area,
                        state.framebuffer.width,
                        state.framebuffer.height,
                    ) {
                        state.x = nx;
                        state.y = ny;
                        state.animator.facing = facing;
                        state.animator.play(anim);
                        if let Some(hat) = hat_override {
                            state.animator.hat = hat;
                        }
                    } else {
                        // Finished flight -> restore resting state
                        state.animator.hat = state.physics.original_hat;
                        state.animator.reapply_state();
                    }
                } else if !state.is_dragging {
                    // 3. Wander Controller (Walking excursions)
                    let is_claude_busy = state.animator.current_anim == AnimId::Work
                        || state.animator.current_anim == AnimId::Think
                        || state.animator.current_anim == AnimId::Alert;

                    if let Some((nx, ny, facing)) = state.wanderer.update(
                        &work_area,
                        state.framebuffer.width,
                        state.framebuffer.height,
                        is_claude_busy,
                        state.animator.hovering,
                    ) {
                        state.x = nx;
                        state.y = ny;
                        state.animator.facing = facing;
                        if state.wanderer.mode == crate::wander::WanderMode::WalkingToTarget
                            || state.wanderer.mode == crate::wander::WanderMode::Homing
                        {
                            state.animator.play(AnimId::Walk);
                        }
                    }
                }

                // 4. Update Animator ticks
                state.animator.update(dt);

                // 5. Render and present
                state.render_and_present();

                // 6. Dynamic FPS profile
                let needs_high_fps = state.is_dragging
                    || state.physics.is_active()
                    || state.wanderer.mode != crate::wander::WanderMode::RestingAtHome
                    || state.animator.current_anim != AnimId::Rest;
                state.set_fps_profile(needs_high_fps);

                // 7. Memory trimming every 8s
                if state.last_trim.elapsed().as_secs() >= 8 {
                    state.trim_memory();
                    state.last_trim = now;
                }
            }
            0
        }

        WM_SIDECRAB_STATE_CHANGED => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                state.last_hook_time = Instant::now();

                let path = state_path();
                if let Ok(data) = std::fs::read_to_string(&path) {
                    let clean = data.trim_start_matches('\u{feff}').trim();
                    if let Ok(state_json) = serde_json::from_str::<StateJson>(clean) {
                        let mascot_state = state_json.mascot_state();
                        state.animator.apply_feed_state(mascot_state);
                        state.set_fps_profile(true);
                        state.render_and_present();
                        state.trim_memory();
                    }
                }
            }
            0
        }

        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn handle_context_command(state: &mut AppState, cmd: usize) {
    match cmd {
        IDM_SIZE_S => change_window_size(state, WindowSize::Small),
        IDM_SIZE_M => change_window_size(state, WindowSize::Medium),
        IDM_SIZE_L => change_window_size(state, WindowSize::Large),

        IDM_CORNER_TL => dock_to_corner(state, Corner::TopLeft),
        IDM_CORNER_TR => dock_to_corner(state, Corner::TopRight),
        IDM_CORNER_BL => dock_to_corner(state, Corner::BottomLeft),
        IDM_CORNER_BR => dock_to_corner(state, Corner::BottomRight),
        IDM_CORNER_RESET => dock_to_corner(state, Corner::BottomRight),

        IDM_HAT_NONE => {
            state.animator.hat = HatType::None;
            state.save_current_config();
            state.render_and_present();
        }
        IDM_HAT_TOP => {
            state.animator.hat = HatType::Top;
            state.save_current_config();
            state.render_and_present();
        }
        IDM_HAT_CHEF => {
            state.animator.hat = HatType::Chef;
            state.save_current_config();
            state.render_and_present();
        }
        IDM_HAT_FEDORA => {
            state.animator.hat = HatType::Fedora;
            state.save_current_config();
            state.render_and_present();
        }
        IDM_HAT_HELI => {
            state.animator.hat = HatType::Helicopter;
            state.save_current_config();
            state.render_and_present();
        }

        // Test Animations Submenu Handlers
        IDM_TEST_REST => {
            state.animator.set_test_override(AnimId::Rest);
            state.render_and_present();
        }
        IDM_TEST_THINK => {
            state.animator.set_test_override(AnimId::Think);
            state.render_and_present();
        }
        IDM_TEST_WORK => {
            state.animator.set_test_override(AnimId::Work);
            state.render_and_present();
        }
        IDM_TEST_ALERT => {
            state.animator.set_test_override(AnimId::Alert);
            state.render_and_present();
        }
        IDM_TEST_CELEBRATE => {
            state.animator.set_test_override(AnimId::Celebrate);
            state.render_and_present();
        }
        IDM_TEST_WALK => {
            state.animator.set_test_override(AnimId::Walk);
            state.render_and_present();
        }
        IDM_TEST_STRETCH => {
            state.animator.set_test_override(AnimId::Stretch);
            state.render_and_present();
        }
        IDM_TEST_BLINK => {
            state.animator.set_test_override(AnimId::Blink);
            state.render_and_present();
        }
        IDM_TEST_PEEK => {
            state.animator.set_test_override(AnimId::Peek);
            state.render_and_present();
        }
        IDM_TEST_LOOK => {
            state.animator.set_test_override(AnimId::Look);
            state.render_and_present();
        }
        IDM_TEST_WAVE => {
            state.animator.set_test_override(AnimId::Wave);
            state.render_and_present();
        }
        IDM_TEST_SHUFFLE => {
            state.animator.set_test_override(AnimId::Shuffle);
            state.render_and_present();
        }
        IDM_TEST_SLEEP => {
            state.animator.set_test_override(AnimId::Sleep);
            state.render_and_present();
        }
        IDM_TEST_PANIC => {
            state.animator.set_test_override(AnimId::Panic);
            state.render_and_present();
        }
        IDM_TEST_GLARE => {
            state.animator.set_test_override(AnimId::Glare);
            state.render_and_present();
        }
        IDM_TEST_CHASE => {
            state.animator.set_test_override(AnimId::Chase);
            state.render_and_present();
        }
        IDM_TEST_RESET => {
            state.animator.clear_test_override();
            state.render_and_present();
        }

        // Easter egg flight mode & quick wander
        IDM_FLIGHT_EASTER_EGG => {
            state.physics.start_easter_egg_flight(state.x, state.y, state.animator.hat);
            state.set_fps_profile(true);
        }
        IDM_WANDER_NOW => {
            let work_area = get_desktop_work_area();
            state.wanderer.trigger_wander_now(&work_area, state.framebuffer.width, state.framebuffer.height);
            state.set_fps_profile(true);
        }
        IDM_RESET_STATE => {
            state.animator.clear_test_override();
            state.animator.apply_feed_state(MascotState::Idle);
            let s_path = state_path();
            let _ = std::fs::write(&s_path, r#"{"state":"idle","mood":"neutral","label":"Idle"}"#);
            state.render_and_present();
        }

        IDM_WANDER => {
            state.wanderer.enabled = !state.wanderer.enabled;
            state.save_current_config();
        }

        IDM_AUTOSTART => {
            let currently = is_autostart_enabled();
            set_autostart(!currently);
            state.save_current_config();
        }

        IDM_HOOKS_INSTALL => {
            let _ = std::process::Command::new("sidecrab-hook.exe")
                .arg("--install")
                .spawn();
        }

        IDM_HOOKS_REMOVE => {
            let _ = std::process::Command::new("sidecrab-hook.exe")
                .arg("--uninstall")
                .spawn();
        }

        IDM_EXIT => {
            DestroyWindow(state.hwnd);
        }

        _ => {}
    }
}

#[derive(Clone, Copy)]
enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

unsafe fn dock_to_corner(state: &mut AppState, corner: Corner) {
    let work_area = get_desktop_work_area();
    let w = state.framebuffer.width;
    let h = state.framebuffer.height;
    const PADDING: i32 = 20;

    let (nx, ny) = match corner {
        Corner::TopLeft => (work_area.left + PADDING, work_area.top + PADDING),
        Corner::TopRight => (work_area.right - w - PADDING, work_area.top + PADDING),
        Corner::BottomLeft => (work_area.left + PADDING, work_area.bottom - h - PADDING),
        Corner::BottomRight => (work_area.right - w - PADDING, work_area.bottom - h - PADDING),
    };

    state.x = nx;
    state.y = ny;
    state.wanderer.set_home(nx, ny);
    state.physics.set_home(nx, ny);
    state.save_current_config();
    state.render_and_present();
}

unsafe fn change_window_size(state: &mut AppState, new_size: WindowSize) {
    if state.size == new_size {
        return;
    }

    let old_h = state.framebuffer.height;
    let old_w = state.framebuffer.width;

    let new_w = new_size.width();
    let new_h = new_size.height();
    let new_scale = new_size.scale();

    // Anchor bottom edge
    state.y += old_h - new_h;
    state.x += (old_w - new_w) / 2;
    state.size = new_size;

    state.framebuffer.resize(new_w, new_h, new_scale);
    SetWindowPos(
        state.hwnd,
        null_mut(),
        state.x,
        state.y,
        new_w,
        new_h,
        SWP_NOZORDER | SWP_NOACTIVATE,
    );

    state.wanderer.set_home(state.x, state.y);
    state.physics.set_home(state.x, state.y);
    state.save_current_config();
    state.render_and_present();
    state.trim_memory();
}
