use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::RECT;

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
    next_wander_delay_ms: u64,
    walk_started: Instant,
    walk_duration_ms: u64,
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
            next_wander_delay_ms: 30_000,
            walk_started: now,
            walk_duration_ms: 8_000,
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
        self.current_x = x as f32;
        self.current_y = y as f32;
    }

    pub fn mark_user_activity(&mut self) {
        self.idle_since = Instant::now();
        self.next_wander_delay_ms = 30_000;
    }

    pub fn interrupt(&mut self) {
        self.mode = WanderMode::RestingAtHome;
        self.set_home(self.current_x.round() as i32, self.current_y.round() as i32);
        self.idle_since = Instant::now();
        self.next_wander_delay_ms = 30_000;
    }

    /// Explicitly triggers a walk excursion immediately
    pub fn trigger_wander_now(&mut self, work_area: &RECT, win_w: i32, win_h: i32) {
        self.enabled = true;
        self.pick_new_target(work_area, win_w, win_h);
        self.walk_started = Instant::now();
        self.walk_duration_ms = 8000;
        self.mode = WanderMode::WalkingToTarget;
    }

    pub fn start_direct_walk(&mut self, target_x: i32, target_y: i32) {
        self.enabled = true;
        self.target_x = target_x as f32;
        self.target_y = target_y as f32;
        self.facing = if (target_x as f32) >= self.current_x { 1 } else { -1 };
        self.walk_started = Instant::now();
        self.walk_duration_ms = 15_000;
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

        // Hover pauses movement in place so user can interact
        if is_hovered {
            return None;
        }

        // Claude is busy -> return home to laptop
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
                // Spontaneous wander excursion after 30s of uninterrupted idle
                if !is_claude_busy && self.idle_since.elapsed().as_millis() >= (self.next_wander_delay_ms as u128) {
                    self.pick_new_target(work_area, win_w, win_h);
                    self.walk_started = now;
                    self.walk_duration_ms = 7500 + (self.pseudo_rand() % 1000); // around 8s
                    self.mode = WanderMode::WalkingToTarget;
                }
                None
            }
            WanderMode::PausedAtDestination => {
                if now >= self.pause_until {
                    // Stood back up! Transition to upright idle, wait 30s, repeat cycle
                    self.mode = WanderMode::RestingAtHome;
                    self.idle_since = now;
                    self.next_wander_delay_ms = 30_000;
                    self.set_home(self.current_x.round() as i32, self.current_y.round() as i32);
                }
                None
            }
            WanderMode::WalkingToTarget => {
                let dx = self.target_x - self.current_x;
                let dy = self.target_y - self.current_y;
                let dist = (dx * dx + dy * dy).sqrt();

                let walk_time_elapsed = now.duration_since(self.walk_started).as_millis() >= (self.walk_duration_ms as u128);

                if walk_time_elapsed || dist <= WANDER_SPEED {
                    self.mode = WanderMode::PausedAtDestination;
                    // Sit down for a bit (~9.0 seconds for the full 3-phase sit & relax)
                    self.pause_until = now + Duration::from_millis(9000);
                    self.set_home(self.current_x.round() as i32, self.current_y.round() as i32);
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
                    self.next_wander_delay_ms = 30_000;
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
        let min_x = work_area.left as f32;
        let max_x = ((work_area.right - win_w) as f32).max(min_x);
        let min_y = work_area.top as f32;
        let floor_y = ((work_area.bottom - win_h) as f32).max(min_y);

        let roll = self.pseudo_rand() % 10;
        if roll < 5 {
            // 50%: Stroll along the bottom floor / taskbar edge
            let span = (max_x - min_x).max(1.0);
            let rx = (self.pseudo_rand() as f32) % span;
            self.target_x = min_x + rx;
            self.target_y = floor_y;
        } else if roll < 8 {
            // 30%: Visit an exact corner of the screen!
            let corner = self.pseudo_rand() % 4;
            match corner {
                0 => { self.target_x = min_x; self.target_y = min_y; } // Top-Left
                1 => { self.target_x = max_x; self.target_y = min_y; } // Top-Right
                2 => { self.target_x = min_x; self.target_y = floor_y; } // Bottom-Left
                _ => { self.target_x = max_x; self.target_y = floor_y; } // Bottom-Right
            }
        } else if roll < 9 {
            // 10%: Stroll along screen borders
            let on_right = (self.pseudo_rand() % 2) == 0;
            self.target_x = if on_right { max_x } else { min_x };
            let span_y = (floor_y - min_y).max(1.0);
            let ry = (self.pseudo_rand() as f32) % span_y;
            self.target_y = min_y + ry;
        } else {
            // 10%: Return directly to home base
            self.target_x = self.home_x as f32;
            self.target_y = self.home_y as f32;
        }
    }
}
