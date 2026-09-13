use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

#[link(name = "kernel32")]
extern "system" {
    fn GetTickCount() -> u32;
}

pub const STEP_MS: u64 = 28;
pub const WANDER_SPEED: f32 = 3.5; // 3.5 px / step
pub const HOME_SPEED: f32 = 7.0;   // 7 px / step (scurry home)
pub const HOME_EPS: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WanderMode {
    Off,
    RestingAtHome,
    WalkingToTarget,
    PausedAtDestination,
    Homing,
}

/// Queries system-wide inactivity via Win32 GetLastInputInfo.
pub fn get_system_idle_seconds() -> f32 {
    unsafe {
        let mut lii = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if GetLastInputInfo(&mut lii) != 0 {
            let now_tick = GetTickCount();
            let elapsed_ms = now_tick.wrapping_sub(lii.dwTime);
            (elapsed_ms as f32) / 1000.0
        } else {
            0.0
        }
    }
}

pub struct WanderController {
    pub enabled: bool,
    pub mode: WanderMode,
    pub home_x: i32,
    pub home_y: i32,
    pub current_x: f32,
    pub current_y: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub facing: i32,

    last_step: Instant,
    pause_until: Instant,
    idle_since: Instant,
    rng_seed: u64,
}

impl WanderController {
    pub fn new(home_x: i32, home_y: i32) -> Self {
        let now = Instant::now();
        Self {
            enabled: true,
            mode: WanderMode::RestingAtHome,
            home_x,
            home_y,
            current_x: home_x as f32,
            current_y: home_y as f32,
            target_x: home_x as f32,
            target_y: home_y as f32,
            facing: 1,
            last_step: now,
            pause_until: now,
            idle_since: now,
            rng_seed: 0x9876543210FEDCBA,
        }
    }

    fn pseudo_rand(&mut self) -> u64 {
        let mut x = self.rng_seed;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_seed = x;
        x
    }

    pub fn set_home(&mut self, x: i32, y: i32) {
        self.home_x = x;
        self.home_y = y;
        if self.mode == WanderMode::RestingAtHome {
            self.current_x = x as f32;
            self.current_y = y as f32;
        }
    }

    pub fn mark_user_activity(&mut self) {
        self.idle_since = Instant::now();
        if self.mode == WanderMode::WalkingToTarget || self.mode == WanderMode::PausedAtDestination {
            self.mode = WanderMode::Homing;
        }
    }

    /// Explicitly triggers a walk excursion immediately
    pub fn trigger_wander_now(&mut self, work_area: &RECT, win_w: i32, win_h: i32) {
        self.enabled = true;
        self.pick_new_target(work_area, win_w, win_h);
        self.mode = WanderMode::WalkingToTarget;
    }

    /// Ticks wander physics. Returns `Some((new_x, new_y, facing))` if the window moved.
    pub fn update(
        &mut self,
        work_area: &RECT,
        win_w: i32,
        win_h: i32,
        is_claude_busy: bool,
        is_hovered: bool,
    ) -> Option<(i32, i32, i32)> {
        if !self.enabled {
            if self.mode != WanderMode::Off && self.mode != WanderMode::RestingAtHome {
                // Return home immediately
                self.current_x = self.home_x as f32;
                self.current_y = self.home_y as f32;
                self.mode = WanderMode::RestingAtHome;
                return Some((self.home_x, self.home_y, self.facing));
            }
            return None;
        }

        // Hover pauses movement in place
        if is_hovered {
            return None;
        }

        // Check Windows system-wide user input idle time
        let sys_idle_secs = get_system_idle_seconds();
        if sys_idle_secs < 2.0 && (self.mode == WanderMode::WalkingToTarget || self.mode == WanderMode::PausedAtDestination) {
            // User came back! Scurry home
            self.mode = WanderMode::Homing;
        }

        // Claude is busy -> return home
        if is_claude_busy && (self.mode == WanderMode::WalkingToTarget || self.mode == WanderMode::PausedAtDestination) {
            self.mode = WanderMode::Homing;
        }

        let now = Instant::now();
        if now.duration_since(self.last_step) < Duration::from_millis(STEP_MS) {
            return None;
        }
        self.last_step = now;

        match self.mode {
            WanderMode::Off => None,
            WanderMode::RestingAtHome => {
                // Wander if system idle >= 20 seconds and Claude is not busy
                if !is_claude_busy && (sys_idle_secs >= 20.0 || self.idle_since.elapsed().as_secs() >= 25) {
                    self.pick_new_target(work_area, win_w, win_h);
                    self.mode = WanderMode::WalkingToTarget;
                }
                None
            }
            WanderMode::PausedAtDestination => {
                if now >= self.pause_until {
                    // Decide whether to wander more or go home
                    if (self.pseudo_rand() % 3) == 0 {
                        self.mode = WanderMode::Homing;
                    } else {
                        self.pick_new_target(work_area, win_w, win_h);
                        self.mode = WanderMode::WalkingToTarget;
                    }
                }
                None
            }
            WanderMode::WalkingToTarget => {
                let dx = self.target_x - self.current_x;
                let dy = self.target_y - self.current_y;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist <= WANDER_SPEED {
                    self.current_x = self.target_x;
                    self.current_y = self.target_y;
                    self.mode = WanderMode::PausedAtDestination;
                    let pause_ms = 2000 + (self.pseudo_rand() % 3000);
                    self.pause_until = now + Duration::from_millis(pause_ms);
                    return Some((self.current_x.round() as i32, self.current_y.round() as i32, self.facing));
                }

                let angle = dy.atan2(dx);
                self.current_x += angle.cos() * WANDER_SPEED;
                self.current_y += angle.sin() * WANDER_SPEED;
                self.facing = if dx < 0.0 { -1 } else { 1 };

                Some((self.current_x.round() as i32, self.current_y.round() as i32, self.facing))
            }
            WanderMode::Homing => {
                let dx = (self.home_x as f32) - self.current_x;
                let dy = (self.home_y as f32) - self.current_y;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist <= HOME_EPS {
                    self.current_x = self.home_x as f32;
                    self.current_y = self.home_y as f32;
                    self.mode = WanderMode::RestingAtHome;
                    self.idle_since = Instant::now();
                    return Some((self.home_x, self.home_y, 1));
                }

                let angle = dy.atan2(dx);
                self.current_x += angle.cos() * HOME_SPEED;
                self.current_y += angle.sin() * HOME_SPEED;
                self.facing = if dx < 0.0 { -1 } else { 1 };

                Some((self.current_x.round() as i32, self.current_y.round() as i32, self.facing))
            }
        }
    }

    fn pick_new_target(&mut self, work_area: &RECT, win_w: i32, win_h: i32) {
        let min_x = work_area.left + 24;
        let max_x = (work_area.right - win_w - 24).max(min_x);
        let min_y = work_area.top + 24;
        let max_y = (work_area.bottom - win_h - 24).max(min_y);

        let rx = (self.pseudo_rand() % ((max_x - min_x + 1) as u64)) as i32;
        let ry = (self.pseudo_rand() % ((max_y - min_y + 1) as u64)) as i32;

        self.target_x = (min_x + rx) as f32;
        self.target_y = (min_y + ry) as f32;
    }
}
