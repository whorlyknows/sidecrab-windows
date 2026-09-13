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
    pub eyes_dx: i32,
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
            eyes_dx: 0,
            mark: None,
            zzz: None,
        }
    }
}

/// Identifiers for all mascot animation states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AnimId {
    Rest,
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
}

impl AnimId {
    pub fn is_looping(&self) -> bool {
        match self {
            AnimId::Blink
            | AnimId::Shuffle
            | AnimId::Stretch
            | AnimId::Peek
            | AnimId::Look
            | AnimId::Wave => false,
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
        } else if !self.traveling {
            self.reapply_state();
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

        // Check celebrate decay
        if let Some(decay) = self.celebrate_decay_ms {
            if self.total_elapsed_ms >= decay {
                self.celebrate_decay_ms = None;
                if !self.hovering && !self.traveling {
                    self.reapply_state();
                }
            }
        }

        // Check pet decay
        if let Some(decay) = self.pet_decay_ms {
            if self.total_elapsed_ms >= decay {
                self.pet_decay_ms = None;
                if !self.hovering && !self.traveling {
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
                // Trigger micro-life
                let micros = [
                    AnimId::Look,
                    AnimId::Look,
                    AnimId::Shuffle,
                    AnimId::Stretch,
                    AnimId::Peek,
                    AnimId::Wave,
                ];
                let idx = (self.pseudo_rand() as usize) % micros.len();
                self.play(micros[idx]);
                self.next_micro_ms = self.total_elapsed_ms + self.rand_range(3000, 9000);
            }
        }

        // Advance animation step
        let steps = self.get_steps(self.current_anim);
        if self.step_index >= steps.len() {
            self.step_index = 0;
            self.step_elapsed = Duration::ZERO;
        }

        let current_step_duration = Duration::from_millis(steps[self.step_index].duration_ms);
        if self.step_elapsed >= current_step_duration {
            self.step_elapsed -= current_step_duration;
            self.step_index += 1;
            if self.step_index >= steps.len() {
                if self.current_anim.is_looping() {
                    self.step_index = 0;
                } else {
                    // Micro-idle finished -> return to Rest
                    self.play(AnimId::Rest);
                }
            }
        }
    }

    /// Returns the current step and ambient flags to render.
    pub fn current_render_step(&mut self) -> (AnimStep, bool, Option<usize>) {
        let steps = self.get_steps(self.current_anim);
        let step = steps[self.step_index.min(steps.len() - 1)];

        // Ambient blink check
        let mut ambient_blink = false;
        if !step.blink && !step.squint && !step.half_eyes && step.eyes_dx == 0 {
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
        }
    }
}

// ── Static Animation Step Tables ─────────────────────────────────────────────

pub static REST_STEPS: [AnimStep; 1] = [AnimStep::new(0, 60000)];

pub static BLINK_STEPS: [AnimStep; 3] = [
    AnimStep { frame_idx: 0, duration_ms: 160, dy: 0, blink: true, squint: false, half_eyes: false, eyes_dx: 0, mark: None, zzz: None },
    AnimStep::new(0, 120),
    AnimStep { frame_idx: 0, duration_ms: 140, dy: 0, blink: true, squint: false, half_eyes: false, eyes_dx: 0, mark: None, zzz: None },
];

pub static SHUFFLE_STEPS: [AnimStep; 4] = [
    AnimStep::new(5, 260),
    AnimStep::new(6, 260),
    AnimStep::new(5, 260),
    AnimStep::new(0, 120),
];

pub static STRETCH_STEPS: [AnimStep; 4] = [
    AnimStep::new(22, 260),
    AnimStep::new(23, 850),
    AnimStep::new(22, 220),
    AnimStep::new(0, 140),
];

pub static PEEK_STEPS: [AnimStep; 4] = [
    AnimStep::new(8, 500),
    AnimStep::new(0, 150),
    AnimStep::new(16, 500),
    AnimStep::new(0, 120),
];

pub static LOOK_STEPS: [AnimStep; 4] = [
    AnimStep { frame_idx: 0, duration_ms: 550, dy: 0, blink: false, squint: false, half_eyes: false, eyes_dx: -2, mark: None, zzz: None },
    AnimStep::new(0, 200),
    AnimStep { frame_idx: 0, duration_ms: 550, dy: 0, blink: false, squint: false, half_eyes: false, eyes_dx: 2, mark: None, zzz: None },
    AnimStep::new(0, 150),
];

pub static WAVE_STEPS: [AnimStep; 6] = [
    AnimStep::new(20, 180),
    AnimStep::new(21, 180),
    AnimStep::new(20, 180),
    AnimStep::new(21, 180),
    AnimStep::new(20, 200),
    AnimStep::new(0, 120),
];

pub static WALK_STEPS: [AnimStep; 15] = [
    AnimStep::new(5, 70),  AnimStep::new(6, 70),  AnimStep::new(7, 70),
    AnimStep::new(8, 70),  AnimStep::new(9, 70),  AnimStep::new(10, 70),
    AnimStep::new(11, 70), AnimStep::new(12, 70), AnimStep::new(13, 70),
    AnimStep::new(14, 70), AnimStep::new(15, 70), AnimStep::new(16, 70),
    AnimStep::new(17, 70), AnimStep::new(18, 70), AnimStep::new(19, 70),
];

pub static THINK_STEPS: [AnimStep; 1] = [AnimStep::new(26, 60000)];

pub static WORK_STEPS: [AnimStep; 2] = [
    AnimStep::new(24, 130),
    AnimStep::new(25, 130),
];

pub static ALERT_STEPS: [AnimStep; 2] = [
    AnimStep { frame_idx: 27, duration_ms: 220, dy: 0, blink: false, squint: false, half_eyes: false, eyes_dx: 0, mark: Some("!?"), zzz: None },
    AnimStep { frame_idx: 28, duration_ms: 220, dy: 0, blink: false, squint: false, half_eyes: false, eyes_dx: 0, mark: Some("!?"), zzz: None },
];

pub static CELEBRATE_STEPS: [AnimStep; 4] = [
    AnimStep { frame_idx: 0, duration_ms: 130, dy: -4, blink: false, squint: false, half_eyes: false, eyes_dx: 0, mark: None, zzz: None },
    AnimStep::new(0, 130),
    AnimStep { frame_idx: 5, duration_ms: 130, dy: -4, blink: false, squint: false, half_eyes: false, eyes_dx: 0, mark: None, zzz: None },
    AnimStep::new(5, 130),
];

pub static HOVER_STEPS: [AnimStep; 1] = [
    AnimStep { frame_idx: 0, duration_ms: 60000, dy: 3, blink: false, squint: true, half_eyes: false, eyes_dx: 0, mark: None, zzz: None },
];

pub static HOVER_WORK_STEPS: [AnimStep; 1] = [
    AnimStep { frame_idx: 24, duration_ms: 60000, dy: 0, blink: false, squint: true, half_eyes: false, eyes_dx: 0, mark: None, zzz: None },
];

pub static HOVER_THINK_STEPS: [AnimStep; 1] = [
    AnimStep { frame_idx: 26, duration_ms: 60000, dy: 0, blink: false, squint: true, half_eyes: false, eyes_dx: 0, mark: None, zzz: None },
];

pub static PET_WORK_STEPS: [AnimStep; 4] = [
    AnimStep { frame_idx: 24, duration_ms: 140, dy: -3, blink: false, squint: false, half_eyes: false, eyes_dx: 0, mark: None, zzz: None },
    AnimStep::new(24, 140),
    AnimStep { frame_idx: 24, duration_ms: 120, dy: -2, blink: false, squint: false, half_eyes: false, eyes_dx: 0, mark: None, zzz: None },
    AnimStep::new(24, 160),
];

pub static PET_THINK_STEPS: [AnimStep; 4] = [
    AnimStep { frame_idx: 26, duration_ms: 140, dy: -3, blink: false, squint: false, half_eyes: false, eyes_dx: 0, mark: None, zzz: None },
    AnimStep::new(26, 140),
    AnimStep { frame_idx: 26, duration_ms: 120, dy: -2, blink: false, squint: false, half_eyes: false, eyes_dx: 0, mark: None, zzz: None },
    AnimStep::new(26, 160),
];

pub static PANIC_STEPS: [AnimStep; 15] = [
    AnimStep::new(5, 45),  AnimStep::new(6, 45),  AnimStep::new(7, 45),
    AnimStep::new(8, 45),  AnimStep::new(9, 45),  AnimStep::new(10, 45),
    AnimStep::new(11, 45), AnimStep::new(12, 45), AnimStep::new(13, 45),
    AnimStep::new(14, 45), AnimStep::new(15, 45), AnimStep::new(16, 45),
    AnimStep::new(17, 45), AnimStep::new(18, 45), AnimStep::new(19, 45),
];

pub static GLARE_STEPS: [AnimStep; 1] = [
    AnimStep { frame_idx: 0, duration_ms: 60000, dy: 0, blink: false, squint: true, half_eyes: false, eyes_dx: 0, mark: None, zzz: None },
];

pub static CHASE_STEPS: [AnimStep; 15] = [
    AnimStep { frame_idx: 5, duration_ms: 45, dy: 0, blink: false, squint: false, half_eyes: true, eyes_dx: 0, mark: Some("!"), zzz: None },
    AnimStep { frame_idx: 6, duration_ms: 45, dy: 0, blink: false, squint: false, half_eyes: true, eyes_dx: 0, mark: Some("!"), zzz: None },
    AnimStep { frame_idx: 7, duration_ms: 45, dy: 0, blink: false, squint: false, half_eyes: true, eyes_dx: 0, mark: Some("!"), zzz: None },
    AnimStep { frame_idx: 8, duration_ms: 45, dy: 0, blink: false, squint: false, half_eyes: true, eyes_dx: 0, mark: Some("!"), zzz: None },
    AnimStep { frame_idx: 9, duration_ms: 45, dy: 0, blink: false, squint: false, half_eyes: true, eyes_dx: 0, mark: Some("!"), zzz: None },
    AnimStep { frame_idx: 10, duration_ms: 45, dy: 0, blink: false, squint: false, half_eyes: true, eyes_dx: 0, mark: Some("!"), zzz: None },
    AnimStep { frame_idx: 11, duration_ms: 45, dy: 0, blink: false, squint: false, half_eyes: true, eyes_dx: 0, mark: Some("!"), zzz: None },
    AnimStep { frame_idx: 12, duration_ms: 45, dy: 0, blink: false, squint: false, half_eyes: true, eyes_dx: 0, mark: Some("!"), zzz: None },
    AnimStep { frame_idx: 13, duration_ms: 45, dy: 0, blink: false, squint: false, half_eyes: true, eyes_dx: 0, mark: Some("!"), zzz: None },
    AnimStep { frame_idx: 14, duration_ms: 45, dy: 0, blink: false, squint: false, half_eyes: true, eyes_dx: 0, mark: Some("!"), zzz: None },
    AnimStep { frame_idx: 15, duration_ms: 45, dy: 0, blink: false, squint: false, half_eyes: true, eyes_dx: 0, mark: Some("!"), zzz: None },
    AnimStep { frame_idx: 16, duration_ms: 45, dy: 0, blink: false, squint: false, half_eyes: true, eyes_dx: 0, mark: Some("!"), zzz: None },
    AnimStep { frame_idx: 17, duration_ms: 45, dy: 0, blink: false, squint: false, half_eyes: true, eyes_dx: 0, mark: Some("!"), zzz: None },
    AnimStep { frame_idx: 18, duration_ms: 45, dy: 0, blink: false, squint: false, half_eyes: true, eyes_dx: 0, mark: Some("!"), zzz: None },
    AnimStep { frame_idx: 19, duration_ms: 45, dy: 0, blink: false, squint: false, half_eyes: true, eyes_dx: 0, mark: Some("!"), zzz: None },
];

pub static SLEEP_STEPS: [AnimStep; 2] = [
    AnimStep { frame_idx: 0, duration_ms: 900, dy: 2, blink: false, squint: true, half_eyes: false, eyes_dx: 0, mark: None, zzz: Some(0) },
    AnimStep { frame_idx: 0, duration_ms: 900, dy: 2, blink: false, squint: true, half_eyes: false, eyes_dx: 0, mark: None, zzz: Some(1) },
];
