use common::HatType;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::RECT;

use crate::anim::AnimId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicsMode {
    Inactive,
    Fling,
    EasterEggFlight,
    Landing,
}

pub struct PhysicsController {
    pub mode: PhysicsMode,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub home_x: i32,
    pub home_y: i32,
    pub facing: i32,
    pub original_hat: HatType,
    pub flight_hat: Option<HatType>,
    pub start_time: Instant,
    pub last_step: Instant,
    pub flight_duration: Duration,
    pub bounces: u32,
    pub wobble_phase: f32,
}

impl PhysicsController {
    pub fn new(home_x: i32, home_y: i32) -> Self {
        let now = Instant::now();
        Self {
            mode: PhysicsMode::Inactive,
            x: home_x as f32,
            y: home_y as f32,
            vx: 0.0,
            vy: 0.0,
            home_x,
            home_y,
            facing: 1,
            original_hat: HatType::None,
            flight_hat: None,
            start_time: now,
            last_step: now,
            flight_duration: Duration::from_secs(8),
            bounces: 0,
            wobble_phase: 0.0,
        }
    }

    pub fn set_home(&mut self, hx: i32, hy: i32) {
        self.home_x = hx;
        self.home_y = hy;
    }

    pub fn is_active(&self) -> bool {
        self.mode != PhysicsMode::Inactive
    }

    /// Triggers mouse fling / toss physics
    pub fn start_fling(&mut self, cur_x: i32, cur_y: i32, vx: f32, vy: f32, current_hat: HatType) {
        let now = Instant::now();
        self.mode = PhysicsMode::Fling;
        self.x = cur_x as f32;
        self.y = cur_y as f32;
        // Clamp velocity to reasonable physics bounds
        self.vx = vx.clamp(-35.0, 35.0);
        self.vy = vy.clamp(-35.0, 35.0);
        self.facing = if self.vx >= 0.0 { 1 } else { -1 };
        self.original_hat = current_hat;
        self.flight_hat = Some(HatType::Helicopter);
        self.start_time = now;
        self.last_step = now;
        self.flight_duration = Duration::from_millis(5000);
        self.bounces = 0;
        self.wobble_phase = 0.0;
    }

    /// Triggers the full VS Code Pet style easter egg flight across the screen
    pub fn start_easter_egg_flight(&mut self, cur_x: i32, cur_y: i32, current_hat: HatType) {
        let now = Instant::now();
        self.mode = PhysicsMode::EasterEggFlight;
        self.x = cur_x as f32;
        self.y = cur_y as f32;
        // Launch up and outward toward the screen center
        let dir = if cur_x > 900 { -1.0 } else { 1.0 };
        self.vx = dir * 18.0;
        self.vy = -16.0;
        self.facing = if self.vx >= 0.0 { 1 } else { -1 };
        self.original_hat = current_hat;
        self.flight_hat = Some(HatType::Helicopter);
        self.start_time = now;
        self.last_step = now;
        self.flight_duration = Duration::from_millis(9000);
        self.bounces = 0;
        self.wobble_phase = 0.0;
    }

    /// Ticks physics step.
    /// Returns `Some((new_x, new_y, facing, anim_to_play, hat_override))` while in motion,
    /// or `None` when inactive or finished.
    pub fn update(
        &mut self,
        work_area: &RECT,
        win_w: i32,
        win_h: i32,
    ) -> Option<(i32, i32, i32, AnimId, Option<HatType>)> {
        if self.mode == PhysicsMode::Inactive {
            return None;
        }

        let now = Instant::now();
        let dt_sec = now.duration_since(self.last_step).as_secs_f32().min(0.1);
        self.last_step = now;

        let elapsed = now.duration_since(self.start_time);
        self.wobble_phase += dt_sec * 6.0;

        let min_x = work_area.left as f32;
        let max_x = (work_area.right - win_w) as f32;
        let min_y = work_area.top as f32;
        let max_y = (work_area.bottom - win_h) as f32;

        match self.mode {
            PhysicsMode::Inactive => None,

            PhysicsMode::Fling | PhysicsMode::EasterEggFlight => {
                // Apply flight forces
                if self.mode == PhysicsMode::Fling {
                    // Slight gravity for realistic toss arc
                    self.vy += 22.0 * dt_sec;
                    // Air friction damping
                    self.vx *= 0.985;
                    self.vy *= 0.985;
                } else {
                    // Easter egg flight: buoyant helicopter thrust
                    self.vx *= 0.992;
                    // Gentle vertical sinusoidal buoyancy
                    self.vy += (self.wobble_phase.sin() * 8.0) * dt_sec;
                    self.vy *= 0.988;
                }

                self.x += self.vx * (dt_sec * 60.0);
                self.y += self.vy * (dt_sec * 60.0);

                // Elastic wall collisions
                let elasticity = 0.82;
                if self.x <= min_x {
                    self.x = min_x;
                    self.vx = -self.vx * elasticity;
                    self.facing = 1;
                    self.bounces += 1;
                } else if self.x >= max_x {
                    self.x = max_x;
                    self.vx = -self.vx * elasticity;
                    self.facing = -1;
                    self.bounces += 1;
                }

                if self.y <= min_y {
                    self.y = min_y;
                    self.vy = -self.vy * elasticity;
                    self.bounces += 1;
                } else if self.y >= max_y {
                    self.y = max_y;
                    self.vy = -self.vy * elasticity;
                    self.bounces += 1;
                }

                if self.vx.abs() > 0.5 {
                    self.facing = if self.vx > 0.0 { 1 } else { -1 };
                }

                // Check transition to Landing
                let total_speed = (self.vx * self.vx + self.vy * self.vy).sqrt();
                if elapsed >= self.flight_duration || (self.mode == PhysicsMode::Fling && total_speed < 1.2 && elapsed.as_millis() > 1200) {
                    self.mode = PhysicsMode::Landing;
                }

                let anim = if self.mode == PhysicsMode::Fling && self.bounces == 0 {
                    AnimId::Panic
                } else {
                    AnimId::Celebrate
                };

                Some((self.x.round() as i32, self.y.round() as i32, self.facing, anim, self.flight_hat))
            }

            PhysicsMode::Landing => {
                // Smooth homing glide back to (home_x, home_y)
                let dx = self.home_x as f32 - self.x;
                let dy = self.home_y as f32 - self.y;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist <= 6.0 {
                    // Arrived home! Land with happy celebration
                    self.x = self.home_x as f32;
                    self.y = self.home_y as f32;
                    self.mode = PhysicsMode::Inactive;
                    return Some((
                        self.home_x,
                        self.home_y,
                        1,
                        AnimId::Celebrate,
                        Some(self.original_hat),
                    ));
                }

                // Glide towards home at 7 px/frame
                let speed = 7.5f32;
                self.x += (dx / dist) * speed;
                self.y += (dy / dist) * speed;
                self.facing = if dx >= 0.0 { 1 } else { -1 };

                Some((
                    self.x.round() as i32,
                    self.y.round() as i32,
                    self.facing,
                    AnimId::Walk,
                    self.flight_hat,
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_physics_initialization_and_fling() {
        let mut pc = PhysicsController::new(100, 200);
        assert!(!pc.is_active());

        pc.start_fling(100, 200, 15.0, -10.0, HatType::Top);
        assert!(pc.is_active());
        assert_eq!(pc.mode, PhysicsMode::Fling);
        assert_eq!(pc.facing, 1);
        assert_eq!(pc.original_hat, HatType::Top);
        assert_eq!(pc.flight_hat, Some(HatType::Helicopter));
    }

    #[test]
    fn test_physics_easter_egg_and_bounce() {
        let mut pc = PhysicsController::new(500, 500);
        pc.start_easter_egg_flight(500, 500, HatType::Fedora);
        assert!(pc.is_active());
        assert_eq!(pc.mode, PhysicsMode::EasterEggFlight);

        let work_area = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };

        // Simulate several physics frames
        for _ in 0..10 {
            std::thread::sleep(Duration::from_millis(5));
            let step = pc.update(&work_area, 153, 144);
            assert!(step.is_some());
        }
    }
}

