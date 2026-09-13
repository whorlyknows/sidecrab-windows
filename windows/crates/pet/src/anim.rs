use common::{HatType, MascotState};
use std::time::{Duration, Instant};

/// Single step inside an animation sequence.
#[derive(Debug, Clone, Copy)]
pub struct AnimStep {
    pub frame_idx: usize,
    pub duration_ms: u64,
    pub dy: i32,
    pub blink: bool,
    pub squint: bool,
    pub half_eyes: bool,
    pub happy_eyes: bool,
    pub eyes_dx: i32,
    pub eyes_dy: i32,
    pub sit_legs: bool,
    pub facing_override: Option<i32>,
    pub mark: Option<&'static str>,
    pub zzz: Option<u8>,
}

impl AnimStep {
    pub const fn new(frame_idx: usize, duration_ms: u64) -> Self {
        Self {
            frame_idx,
            duration_ms,
            dy: 0,
            blink: false,
            squint: false,
            half_eyes: false,
            happy_eyes: false,
            eyes_dx: 0,
            eyes_dy: 0,
            sit_legs: false,
            facing_override: None,
            mark: None,
            zzz: None,
        }
    }

    pub const fn with_dy(mut self, dy: i32) -> Self { self.dy = dy; self }
    pub const fn with_blink(mut self) -> Self { self.blink = true; self }
    pub const fn with_squint(mut self) -> Self { self.squint = true; self }
    pub const fn with_half_eyes(mut self) -> Self { self.half_eyes = true; self }
    pub const fn with_happy_eyes(mut self) -> Self { self.happy_eyes = true; self }
    pub const fn with_eyes(mut self, dx: i32, dy: i32) -> Self { self.eyes_dx = dx; self.eyes_dy = dy; self }
    pub const fn with_sit_legs(mut self) -> Self { self.sit_legs = true; self }
    pub const fn with_facing(mut self, facing: i32) -> Self { self.facing_override = Some(facing); self }
    pub const fn with_mark(mut self, mark: &'static str) -> Self { self.mark = Some(mark); self }
    pub const fn with_zzz(mut self, zzz: u8) -> Self { self.zzz = Some(zzz); self }
}

/// Identifiers for all mascot animation states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AnimId {
    Rest,
    Relax,
    Blink,
    Shuffle,
    Stretch,
    Peek,
    Look,
    Wave,
    Walk,
    Think,
    Work,
    Alert,
    Celebrate,
    Hover,
    HoverWork,
    HoverThink,
    PetWork,
    PetThink,
    Panic,
    Glare,
    Chase,
    Sleep,
    Eat,
}

impl AnimId {
    pub fn is_looping(&self) -> bool {
        match self {
            AnimId::Blink
            | AnimId::Shuffle
            | AnimId::Stretch
            | AnimId::Peek
            | AnimId::Look
            | AnimId::Wave
            | AnimId::Celebrate
            | AnimId::Relax
            | AnimId::Eat => false,
            _ => true,
        }
    }
}

/// Animation state machine and tick controller.
pub struct AnimationController {
    pub current_anim: AnimId,
    pub feed_state: MascotState,
    pub test_override: Option<AnimId>,
    pub step_index: usize,
    pub facing: i32, // 1 = natural (right), -1 = flipped (left)
    pub hat: HatType,
    pub show_thought: bool,
    pub hovering: bool,
    pub traveling: bool,
    pub sleeping: bool,
    pub cursor_eye_dx: i32,
    pub cursor_eye_dy: i32,

    step_elapsed: Duration,
    total_elapsed_ms: u64,
    idle_since: Instant,

    // Micro-life and ambient timers
    next_micro_ms: u64,
    next_blink_ms: u64,
    celebrate_decay_ms: Option<u64>,
    pet_decay_ms: Option<u64>,

    // PRNG seed for deterministic / light randomness
    rng_state: u64,
}

impl AnimationController {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            current_anim: AnimId::Rest,
            feed_state: MascotState::Idle,
            test_override: None,
            step_index: 0,
            facing: 1,
            hat: HatType::None,
            show_thought: false,
            hovering: false,
            traveling: false,
            sleeping: false,
            cursor_eye_dx: 0,
            cursor_eye_dy: 0,
            step_elapsed: Duration::ZERO,
            total_elapsed_ms: 0,
            idle_since: now,
            next_micro_ms: 4000,
            next_blink_ms: 2500,
            celebrate_decay_ms: None,
            pet_decay_ms: None,
            rng_state: 0x123456789ABCDEF0,
        }
    }

    fn pseudo_rand(&mut self) -> u64 {
        // Xorshift64
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        x
    }

    fn rand_range(&mut self, min: u64, max: u64) -> u64 {
        if max <= min {
            return min;
        }
        min + (self.pseudo_rand() % (max - min))
    }

    /// Plays a new animation.
    pub fn play(&mut self, anim: AnimId) {
        if self.current_anim == anim && anim.is_looping() {
            return;
        }
        self.current_anim = anim;
        self.step_index = 0;
        self.step_elapsed = Duration::ZERO;
    }

    pub fn set_test_override(&mut self, anim: AnimId) {
        self.test_override = Some(anim);
        self.show_thought = anim == AnimId::Think || anim == AnimId::HoverThink || anim == AnimId::PetThink;
        self.play(anim);
    }

    pub fn clear_test_override(&mut self) {
        self.test_override = None;
        self.reapply_state();
    }

    pub fn reapply_state(&mut self) {
        if let Some(anim) = self.test_override {
            self.show_thought = anim == AnimId::Think || anim == AnimId::HoverThink || anim == AnimId::PetThink;
            self.play(anim);
            return;
        }

        match self.feed_state {
            MascotState::Thinking => {
                self.sleeping = false;
                self.show_thought = true;
                self.play(AnimId::Think);
            }
            MascotState::Tool | MascotState::Working => {
                self.sleeping = false;
                self.show_thought = false;
                self.play(AnimId::Work);
            }
            MascotState::Permission | MascotState::Alert => {
                self.sleeping = false;
                self.show_thought = false;
                self.play(AnimId::Alert);
            }
            MascotState::Done | MascotState::Celebrate => {
                self.sleeping = false;
                self.show_thought = false;
                self.celebrate_decay_ms = Some(self.total_elapsed_ms + 1500);
                self.play(AnimId::Celebrate);
            }
            MascotState::Sleep => {
                self.sleeping = true;
                self.show_thought = false;
                self.play(AnimId::Sleep);
            }
            _ => {
                self.show_thought = false;
                if self.sleeping {
                    self.play(AnimId::Sleep);
                } else {
                    self.play(AnimId::Rest);
                }
            }
        }
    }

    /// Updates feed state from Claude Code (`idle`, `thinking`, `tool`/`working`, `permission`/`alert`, `done`/`celebrate`).
    pub fn apply_feed_state(&mut self, state: MascotState) {
        self.feed_state = state;

        if self.test_override.is_some() {
            return;
        }

        if self.hovering {
            if state == MascotState::Done {
                self.celebrate_decay_ms = Some(self.total_elapsed_ms + 1500);
            }
            return;
        }

        if self.traveling {
            return;
        }

        self.reapply_state();
    }

    /// Hover entered or left.
    pub fn set_hover(&mut self, hover: bool) {
        if self.hovering == hover {
            return;
        }
        self.hovering = hover;
        if hover {
            self.sleeping = false;
            self.idle_since = Instant::now();
            let hover_anim = if self.traveling {
                AnimId::Hover
            } else if self.current_anim == AnimId::Work || self.current_anim == AnimId::HoverWork || self.feed_state == MascotState::Tool || self.feed_state == MascotState::Working {
                AnimId::HoverWork
            } else if self.current_anim == AnimId::Think || self.current_anim == AnimId::HoverThink || self.show_thought || self.feed_state == MascotState::Thinking {
                AnimId::HoverThink
            } else {
                AnimId::Hover
            };
            self.play(hover_anim);
        } else {
            self.cursor_eye_dx = 0;
            self.cursor_eye_dy = 0;
            if !self.traveling {
                self.reapply_state();
            }
        }
    }

    /// User petted the mascot (click while seated bounces without leaving desk).
    pub fn pet(&mut self) {
        self.sleeping = false;
        self.idle_since = Instant::now();
        let seated = !self.traveling && (self.feed_state == MascotState::Tool || self.feed_state == MascotState::Working || self.feed_state == MascotState::Thinking || self.show_thought);
        let pet_anim = if seated && (self.feed_state == MascotState::Tool || self.feed_state == MascotState::Working) {
            AnimId::PetWork
        } else if seated {
            AnimId::PetThink
        } else {
            AnimId::Celebrate
        };
        self.play(pet_anim);
        self.pet_decay_ms = Some(self.total_elapsed_ms + 700);
    }

    /// Advances the animation by delta time.
    pub fn update(&mut self, dt: Duration) {
        let dt_ms = dt.as_millis() as u64;
        self.total_elapsed_ms += dt_ms;
        self.step_elapsed += dt;

        // Check celebrate decay -> reset to idle so it doesn't loop celebrate forever
        if let Some(decay) = self.celebrate_decay_ms {
            if self.total_elapsed_ms >= decay {
                self.celebrate_decay_ms = None;
                self.feed_state = MascotState::Idle;
                if !self.hovering && !self.traveling {
                    self.reapply_state();
                }
            }
        }

        // Check pet decay
        if let Some(decay) = self.pet_decay_ms {
            if self.total_elapsed_ms >= decay {
                self.pet_decay_ms = None;
                if self.hovering {
                    self.play(AnimId::Hover);
                } else if !self.traveling {
                    self.reapply_state();
                }
            }
        }

        // Idle micro-life scheduler and narcolepsy sleep check
        if self.current_anim == AnimId::Rest && !self.hovering && !self.traveling {
            // Check 3 min (180,000 ms) idle timeout
            if self.idle_since.elapsed().as_millis() >= 180_000 {
                self.sleeping = true;
                self.play(AnimId::Sleep);
            } else if self.total_elapsed_ms >= self.next_micro_ms {
                // Trigger micro-life across fun standing mascot animations
                let micros = [
                    AnimId::Look,
                    AnimId::Shuffle,
                    AnimId::Stretch,
                    AnimId::Peek,
                    AnimId::Wave,
                    AnimId::Celebrate,
                    AnimId::Think,
                    AnimId::Work,
                    AnimId::Alert,
                    AnimId::Glare,
                    AnimId::Chase,
                ];
                let idx = (self.pseudo_rand() as usize) % micros.len();
                self.play(micros[idx]);
                self.next_micro_ms = self.total_elapsed_ms + self.rand_range(4000, 7000);
            }
        }

        // Advance animation step
        let steps = self.get_steps(self.current_anim);
        if self.step_index >= steps.len() {
            self.step_index = 0;
            self.step_elapsed = Duration::ZERO;
        }

        // Apply facing override if step specifies one
        if let Some(fo) = steps[self.step_index].facing_override {
            self.facing = fo;
        }

        let current_step_duration = Duration::from_millis(steps[self.step_index].duration_ms);
        if self.step_elapsed >= current_step_duration {
            self.step_elapsed -= current_step_duration;
            self.step_index += 1;
            if self.step_index >= steps.len() {
                if self.current_anim.is_looping() || self.test_override.is_some() {
                    self.step_index = 0;
                } else {
                    // Non-looping finished -> return to Hover if hovering, else Rest
                    if self.hovering {
                        self.play(AnimId::Hover);
                    } else {
                        self.play(AnimId::Rest);
                    }
                }
            }
        }
    }

    /// Returns the current step and ambient flags to render.
    pub fn current_render_step(&mut self) -> (AnimStep, bool, Option<usize>) {
        let steps = self.get_steps(self.current_anim);
        let mut step = steps[self.step_index.min(steps.len() - 1)];

        // Pupil tracking towards cursor in 2D
        if self.cursor_eye_dx != 0 && !step.blink && !step.squint && step.eyes_dx == 0 {
            step.eyes_dx = self.cursor_eye_dx;
        }
        if self.cursor_eye_dy != 0 && !step.blink && !step.squint && step.eyes_dy == 0 {
            step.eyes_dy = self.cursor_eye_dy;
        }

        // Ambient blink check
        let mut ambient_blink = false;
        if !step.blink && !step.squint && !step.half_eyes && step.eyes_dx == 0 && step.eyes_dy == 0 {
            if self.total_elapsed_ms >= self.next_blink_ms {
                if self.total_elapsed_ms < self.next_blink_ms + 140 {
                    ambient_blink = true;
                } else {
                    self.next_blink_ms = self.total_elapsed_ms + self.rand_range(3500, 9000);
                }
            }
        }

        // Thought bubble phase
        let thought_phase = if self.show_thought && self.current_anim != AnimId::Panic && self.current_anim != AnimId::Chase {
            Some(((self.total_elapsed_ms / 450) % 4) as usize)
        } else {
            None
        };

        (step, ambient_blink, thought_phase)
    }

    pub fn total_ms(&self) -> u64 {
        self.total_elapsed_ms
    }

    /// Retrieves step definitions for a given animation.
    fn get_steps(&self, anim: AnimId) -> &'static [AnimStep] {
        match anim {
            AnimId::Rest => &REST_STEPS,
            AnimId::Relax => &RELAX_STEPS,
            AnimId::Blink => &BLINK_STEPS,
            AnimId::Shuffle => &SHUFFLE_STEPS,
            AnimId::Stretch => &STRETCH_STEPS,
            AnimId::Peek => &PEEK_STEPS,
            AnimId::Look => &LOOK_STEPS,
            AnimId::Wave => &WAVE_STEPS,
            AnimId::Walk => &WALK_STEPS,
            AnimId::Think => &THINK_STEPS,
            AnimId::Work => &WORK_STEPS,
            AnimId::Alert => &ALERT_STEPS,
            AnimId::Celebrate => &CELEBRATE_STEPS,
            AnimId::Hover => &HOVER_STEPS,
            AnimId::HoverWork => &HOVER_WORK_STEPS,
            AnimId::HoverThink => &HOVER_THINK_STEPS,
            AnimId::PetWork => &PET_WORK_STEPS,
            AnimId::PetThink => &PET_THINK_STEPS,
            AnimId::Panic => &PANIC_STEPS,
            AnimId::Glare => &GLARE_STEPS,
            AnimId::Chase => &CHASE_STEPS,
            AnimId::Sleep => &SLEEP_STEPS,
            AnimId::Eat => &EAT_STEPS,
        }
    }
}

// ── Static Animation Step Tables ─────────────────────────────────────────────

pub static REST_STEPS: [AnimStep; 1] = [AnimStep::new(0, 60000)];

/// 3-Phase Seated Posture:
/// 1. Sinks down to floor (~440ms)
/// 2. Front legs appear on floor; gentle breathing with happy ^^ and relaxed eyes (~8s)
/// 3. Inverse stand-up sequence returning to upright resting idle (~440ms)
pub static RELAX_STEPS: [AnimStep; 11] = [
    // Phase 1: Going Down
    AnimStep::new(0, 140).with_dy(2),
    AnimStep::new(0, 140).with_dy(4),
    AnimStep::new(0, 160).with_dy(5).with_sit_legs().with_happy_eyes(),

    // Phase 2: Seated Breathing (~8.0 seconds total)
    AnimStep::new(0, 2000).with_dy(5).with_sit_legs().with_happy_eyes(),
    AnimStep::new(0, 1800).with_dy(4).with_sit_legs().with_half_eyes(),
    AnimStep::new(0, 2200).with_dy(5).with_sit_legs().with_happy_eyes(),
    AnimStep::new(0, 2000).with_dy(4).with_sit_legs().with_half_eyes(),

    // Phase 3: Getting Up Inverse
    AnimStep::new(0, 160).with_dy(5).with_sit_legs().with_happy_eyes(),
    AnimStep::new(0, 140).with_dy(4),
    AnimStep::new(0, 140).with_dy(2),
    AnimStep::new(0, 250).with_dy(0),
];

pub static BLINK_STEPS: [AnimStep; 4] = [
    AnimStep::new(0, 160).with_blink(),
    AnimStep::new(0, 120),
    AnimStep::new(0, 140).with_blink(),
    AnimStep::new(0, 500),
];

pub static SHUFFLE_STEPS: [AnimStep; 5] = [
    AnimStep::new(5, 260),
    AnimStep::new(6, 260),
    AnimStep::new(5, 260),
    AnimStep::new(0, 120),
    AnimStep::new(0, 500),
];

pub static STRETCH_STEPS: [AnimStep; 5] = [
    AnimStep::new(22, 260),
    AnimStep::new(23, 850),
    AnimStep::new(22, 220),
    AnimStep::new(0, 140),
    AnimStep::new(0, 500),
];

/// Bilateral Peek: Visibly turns body and peeks LEFT, blinks, returns center, then turns and peeks RIGHT!
pub static PEEK_STEPS: [AnimStep; 6] = [
    AnimStep::new(0, 600).with_facing(-1).with_eyes(-2, 0),
    AnimStep::new(0, 160).with_facing(-1).with_blink(),
    AnimStep::new(0, 220).with_facing(1).with_eyes(0, 0),
    AnimStep::new(0, 600).with_facing(1).with_eyes(2, 0),
    AnimStep::new(0, 160).with_facing(1).with_blink(),
    AnimStep::new(0, 300).with_eyes(0, 0),
];

pub static LOOK_STEPS: [AnimStep; 5] = [
    AnimStep::new(0, 550).with_eyes(-2, 0),
    AnimStep::new(0, 200),
    AnimStep::new(0, 550).with_eyes(2, 0),
    AnimStep::new(0, 150),
    AnimStep::new(0, 500),
];

pub static WAVE_STEPS: [AnimStep; 7] = [
    AnimStep::new(20, 180),
    AnimStep::new(21, 180),
    AnimStep::new(20, 180),
    AnimStep::new(21, 180),
    AnimStep::new(20, 200),
    AnimStep::new(0, 120),
    AnimStep::new(0, 500),
];

pub static WALK_STEPS: [AnimStep; 15] = [
    AnimStep::new(5, 70),  AnimStep::new(6, 70),  AnimStep::new(7, 70),
    AnimStep::new(8, 70),  AnimStep::new(9, 70),  AnimStep::new(10, 70),
    AnimStep::new(11, 70), AnimStep::new(12, 70), AnimStep::new(13, 70),
    AnimStep::new(14, 70), AnimStep::new(15, 70), AnimStep::new(16, 70),
    AnimStep::new(17, 70), AnimStep::new(18, 70), AnimStep::new(19, 70),
];

pub static THINK_STEPS: [AnimStep; 2] = [
    AnimStep::new(0, 750).with_eyes(1, -1),
    AnimStep::new(0, 750).with_eyes(-1, -1),
];

pub static WORK_STEPS: [AnimStep; 2] = [
    AnimStep::new(24, 130),
    AnimStep::new(25, 130),
];

pub static ALERT_STEPS: [AnimStep; 2] = [
    AnimStep::new(27, 220).with_mark("!?"),
    AnimStep::new(28, 220).with_mark("!?"),
];

pub static CELEBRATE_STEPS: [AnimStep; 5] = [
    AnimStep::new(0, 140).with_dy(-5).with_happy_eyes(),
    AnimStep::new(0, 120).with_dy(-2).with_happy_eyes(),
    AnimStep::new(0, 140).with_dy(0).with_happy_eyes(),
    AnimStep::new(0, 140).with_dy(-4).with_happy_eyes(),
    AnimStep::new(0, 180).with_dy(0).with_happy_eyes(),
];

pub static HOVER_STEPS: [AnimStep; 1] = [
    AnimStep::new(0, 60000).with_dy(2).with_happy_eyes(),
];

pub static HOVER_WORK_STEPS: [AnimStep; 1] = [
    AnimStep::new(24, 60000).with_squint(),
];

pub static HOVER_THINK_STEPS: [AnimStep; 1] = [
    AnimStep::new(0, 60000).with_squint(),
];

pub static PET_WORK_STEPS: [AnimStep; 4] = [
    AnimStep::new(24, 140).with_dy(-3).with_happy_eyes(),
    AnimStep::new(24, 140),
    AnimStep::new(24, 120).with_dy(-2).with_happy_eyes(),
    AnimStep::new(24, 160),
];

pub static PET_THINK_STEPS: [AnimStep; 4] = [
    AnimStep::new(0, 140).with_dy(-3).with_happy_eyes(),
    AnimStep::new(0, 140),
    AnimStep::new(0, 120).with_dy(-2).with_happy_eyes(),
    AnimStep::new(0, 160),
];

pub static PANIC_STEPS: [AnimStep; 15] = [
    AnimStep::new(5, 45),  AnimStep::new(6, 45),  AnimStep::new(7, 45),
    AnimStep::new(8, 45),  AnimStep::new(9, 45),  AnimStep::new(10, 45),
    AnimStep::new(11, 45), AnimStep::new(12, 45), AnimStep::new(13, 45),
    AnimStep::new(14, 45), AnimStep::new(15, 45), AnimStep::new(16, 45),
    AnimStep::new(17, 45), AnimStep::new(18, 45), AnimStep::new(19, 45),
];

pub static GLARE_STEPS: [AnimStep; 1] = [
    AnimStep::new(0, 60000).with_squint(),
];

pub static CHASE_STEPS: [AnimStep; 15] = [
    AnimStep::new(5, 45).with_half_eyes().with_mark("!"),
    AnimStep::new(6, 45).with_half_eyes().with_mark("!"),
    AnimStep::new(7, 45).with_half_eyes().with_mark("!"),
    AnimStep::new(8, 45).with_half_eyes().with_mark("!"),
    AnimStep::new(9, 45).with_half_eyes().with_mark("!"),
    AnimStep::new(10, 45).with_half_eyes().with_mark("!"),
    AnimStep::new(11, 45).with_half_eyes().with_mark("!"),
    AnimStep::new(12, 45).with_half_eyes().with_mark("!"),
    AnimStep::new(13, 45).with_half_eyes().with_mark("!"),
    AnimStep::new(14, 45).with_half_eyes().with_mark("!"),
    AnimStep::new(15, 45).with_half_eyes().with_mark("!"),
    AnimStep::new(16, 45).with_half_eyes().with_mark("!"),
    AnimStep::new(17, 45).with_half_eyes().with_mark("!"),
    AnimStep::new(18, 45).with_half_eyes().with_mark("!"),
    AnimStep::new(19, 45).with_half_eyes().with_mark("!"),
];

pub static SLEEP_STEPS: [AnimStep; 2] = [
    AnimStep::new(0, 900).with_dy(2).with_squint().with_zzz(0),
    AnimStep::new(0, 900).with_dy(2).with_squint().with_zzz(1),
];

pub static EAT_STEPS: [AnimStep; 6] = [
    AnimStep::new(0, 200).with_dy(2).with_happy_eyes().with_eyes(0, 1),
    AnimStep::new(0, 200).with_dy(0).with_eyes(0, 1),
    AnimStep::new(0, 200).with_dy(2).with_happy_eyes().with_eyes(0, 1),
    AnimStep::new(0, 200).with_dy(0).with_eyes(0, 1),
    AnimStep::new(0, 280).with_dy(-3).with_happy_eyes(),
    AnimStep::new(0, 250).with_dy(0).with_happy_eyes(),
];
