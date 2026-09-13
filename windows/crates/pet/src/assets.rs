use common::{hat_color_bgra, HatType, COLOR_BUBBLE_BGRA, COLOR_TYPING_DOTS_BGRA, PALETTE_BGRA};

pub const FRAME_W: usize = 51;
pub const FRAME_H: usize = 36;
pub const CANVAS_W: usize = 51;
pub const CANVAS_H: usize = 48;
pub const CRAB_Y: usize = 12;
pub const NUM_FRAMES: usize = 29;

/// The 29 raw sprite frames encoded as palette indices (0..6).
/// Stored in static `.rodata` (53,244 bytes total).
pub static FRAMES_RAW: &[u8] = include_bytes!("frames.bin");

/// Precomputed head anchors (cx, top) for all 29 frames.
/// cx: center-x of the crab's head.
/// top: topmost y pixel of the head (0..35).
pub const HEAD_ANCHORS: [(f32, i32); NUM_FRAMES] = [
    (25.0, 3), // Frame 0
    (25.0, 3), // Frame 1
    (25.0, 3), // Frame 2
    (25.0, 3), // Frame 3
    (25.0, 3), // Frame 4
    (25.0, 0), // Frame 5: Bob-up step
    (25.0, 0), // Frame 6: Bob-up step
    (25.0, 3), // Frame 7: Normal step
    (25.0, 3), // Frame 8: Normal step
    (25.0, 0), // Frame 9: Bob-up step
    (25.0, 0), // Frame 10: Bob-up step
    (25.0, 3), // Frame 11: Normal step
    (25.0, 3), // Frame 12: Normal step
    (25.0, 0), // Frame 13: Bob-up step
    (25.0, 0), // Frame 14: Bob-up step
    (25.0, 3), // Frame 15: Normal step
    (25.0, 3), // Frame 16: Normal step
    (25.0, 0), // Frame 17: Bob-up step
    (25.0, 0), // Frame 18: Bob-up step
    (25.0, 3), // Frame 19: Normal step
    (26.5, 3), // Frame 20: Wave high
    (25.5, 3), // Frame 21: Wave low
    (25.5, 1), // Frame 22: Stretch rise
    (25.5, 0), // Frame 23: Stretch peak
    (34.0, 6), // Frame 24: Laptop typing 1
    (34.0, 6), // Frame 25: Laptop typing 2
    (34.0, 6), // Frame 26: Laptop pondering
    (25.5, 3), // Frame 27: Double claw alert 1
    (25.5, 3), // Frame 28: Double claw alert 2
];

/// Hat matrix definitions.
pub const HAT_TOP: [&str; 8] = [
    "...kkkkkkkk...",
    "...kkkkkkkk...",
    "...kkkkkkkk...",
    "...kkkkkkkk...",
    "...kkkkkkkk...",
    "...kkkkkkkk...",
    "...bbbbbbbb...",
    "kkkkkkkkkkkkkk",
];

pub const HAT_CHEF: [&str; 8] = [
    "...wwwwwww....",
    ".wwwwwwwwwww..",
    "wwwwwwwwwwwww.",
    "wwwwwwwwwwwww.",
    ".wwwwwwwwwww..",
    "..wwwwwwwww...",
    "..ddddddddd...",
    "..ddddddddd...",
];

pub const HAT_FEDORA: [&str; 4] = [
    ".....kkkkkk.....",
    "....kkkkkkkk....",
    "....bbbbbbbb....",
    "kkkkkkkkkkkkkkkk",
];

pub const HAT_HELI0: [&str; 4] = [
    "gggggggggggggg",
    "......kk......",
    "...rrryyrrr...",
    "...rrrrrrrr...",
];

pub const HAT_HELI1: [&str; 4] = [
    ".....gggg.....",
    "......kk......",
    "...rrryyrrr...",
    "...rrrrrrrr...",
];

/// Returns a slice of the given frame's palette indices.
pub fn get_frame(idx: usize) -> &'static [u8] {
    let i = idx % NUM_FRAMES;
    let start = i * (FRAME_W * FRAME_H);
    &FRAMES_RAW[start..start + (FRAME_W * FRAME_H)]
}

/// Eye pixel information for eye transformations.
#[derive(Clone, Copy)]
pub struct EyePixel {
    pub x: usize,
    pub y: usize,
    pub fill_idx: u8,
}

/// Finds all dark eye pixels (index 1) in a frame along with their fill color below.
pub fn get_eye_data(frame: &[u8]) -> (Vec<EyePixel>, [usize; FRAME_W]) {
    let mut pixels = Vec::with_capacity(32);
    let mut bottoms = [0usize; FRAME_W];

    for y in 0..FRAME_H {
        for x in 0..FRAME_W {
            if frame[y * FRAME_W + x] == 1 {
                // Find fill color: first non-1 pixel below in this column
                let mut sy = y + 1;
                while sy < FRAME_H && frame[sy * FRAME_W + x] == 1 {
                    sy += 1;
                }
                let fill_idx = if sy < FRAME_H {
                    frame[sy * FRAME_W + x]
                } else {
                    frame[(FRAME_H - 1) * FRAME_W + x]
                };
                let fill = if fill_idx != 0 && fill_idx != 1 {
                    fill_idx
                } else {
                    6 // Fallback to Coral
                };

                pixels.push(EyePixel { x, y, fill_idx: fill });
                if y > bottoms[x] {
                    bottoms[x] = y;
                }
            }
        }
    }
    (pixels, bottoms)
}

/// Compositor: Renders a complete mascot frame onto a 51x48 logical canvas.
/// Output format: 51 * 48 pixels in BGRA (pre-multiplied ARGB) format (9,792 bytes).
pub fn composite_logical_frame(
    buf: &mut [[u8; 4]; CANVAS_W * CANVAS_H],
    frame_idx: usize,
    dy: i32,
    facing: i32, // 1 = natural, -1 = flipped
    blink: bool,
    squint: bool,
    half_eyes: bool,
    happy_eyes: bool,
    eyes_dx: i32,
    eyes_dy: i32,
    sit_legs: bool,
    hat: HatType,
    hat_anim_tick: u64, // drives helicopter rotor alternating at 130ms
    thought_phase: Option<usize>, // Some(0..4) when thinking
    mark: Option<&str>, // Some("!") or Some("!?")
    zzz_phase: Option<u8>, // Some(0) or Some(1)
) {
    // 1. Clear buffer to transparent
    for p in buf.iter_mut() {
        *p = [0, 0, 0, 0];
    }

    let frame = get_frame(frame_idx);
    let base_y = (CRAB_Y as i32 + dy).clamp(0, (CANVAS_H - FRAME_H) as i32) as usize;

    // 2. Prepare eye modification masks
    let (eye_pixels, bottoms) = if blink || squint || half_eyes || happy_eyes || eyes_dx != 0 || eyes_dy != 0 {
        get_eye_data(frame)
    } else {
        (Vec::new(), [0usize; FRAME_W])
    };

    // 3. Draw base crab frame
    for cy in 0..FRAME_H {
        for cx in 0..FRAME_W {
            let pixel_idx = frame[cy * FRAME_W + cx];
            if pixel_idx == 0 {
                continue;
            }

            // If sitting, hide the 4 vertical standing legs from Frame 0
            if sit_legs && cy >= 27 {
                if (cx >= 9 && cx <= 12)
                    || (cx >= 17 && cx <= 20)
                    || (cx >= 30 && cx <= 33)
                    || (cx >= 38 && cx <= 41)
                {
                    continue;
                }
            }

            let mut final_color = PALETTE_BGRA[pixel_idx as usize];

            // Eye transformations
            if blink || squint || half_eyes || happy_eyes {
                for ep in &eye_pixels {
                    if ep.x == cx && ep.y == cy {
                        if squint && !blink && !happy_eyes && cy == bottoms[cx] {
                            // Squint: lowest pixel remains open
                            continue;
                        }
                        if half_eyes && !blink && !squint && !happy_eyes && cy >= bottoms[cx] - 1 {
                            // Half eyes: lower 2 rows remain open
                            continue;
                        }
                        // Replace eye pixel with body fill color
                        final_color = PALETTE_BGRA[ep.fill_idx as usize];
                    }
                }
            } else if eyes_dx != 0 || eyes_dy != 0 {
                // Glance: erase old eye pixels, redraw them shifted
                for ep in &eye_pixels {
                    if ep.x == cx && ep.y == cy {
                        final_color = PALETTE_BGRA[ep.fill_idx as usize];
                    }
                }
            }

            let target_x = if facing == -1 {
                (CANVAS_W - 1) as i32 - cx as i32
            } else {
                cx as i32
            };
            let target_y = (base_y + cy) as i32;

            if target_x >= 0 && (target_x as usize) < CANVAS_W && target_y >= 0 && (target_y as usize) < CANVAS_H {
                buf[target_y as usize * CANVAS_W + target_x as usize] = final_color;
            }
        }
    }

    // Draw happy ^^ eyes (curved smiling carets)
    if happy_eyes && !blink {
        let happy_pixels: [(i32, i32); 8] = [
            // Left eye caret
            (14, 8), (15, 8), (13, 9), (16, 9),
            // Right eye caret
            (35, 8), (36, 8), (34, 9), (37, 9),
        ];
        for &(hx, hy) in &happy_pixels {
            let target_x = if facing == -1 {
                (CANVAS_W - 1) as i32 - hx
            } else {
                hx
            };
            let target_y = (base_y as i32) + hy;
            if target_x >= 0 && (target_x as usize) < CANVAS_W && target_y >= 0 && (target_y as usize) < CANVAS_H {
                buf[target_y as usize * CANVAS_W + target_x as usize] = PALETTE_BGRA[1]; // Black caret pixel
            }
        }
    }

    // Draw sitting front legs/paws if seated
    if sit_legs {
        // Front feet / paws resting flat on the floor in front of belly
        // Left paw: cx 14..=21, cy 32..=35
        // Right paw: cx 29..=36, cy 32..=35
        let paws = [
            // Left foot
            (15, 32, 6), (16, 32, 6), (17, 32, 6), (18, 32, 6), (19, 32, 6), (20, 32, 6),
            (14, 33, 6), (15, 33, 5), (16, 33, 6), (17, 33, 6), (18, 33, 6), (19, 33, 5), (20, 33, 6),
            (14, 34, 6), (15, 34, 5), (16, 34, 1), (17, 34, 6), (18, 34, 1), (19, 34, 5), (20, 34, 6),
            (14, 35, 6), (15, 35, 6), (16, 35, 6), (17, 35, 6), (18, 35, 6), (19, 35, 6), (20, 35, 6),
            // Right foot
            (30, 32, 6), (31, 32, 6), (32, 32, 6), (33, 32, 6), (34, 32, 6), (35, 32, 6),
            (30, 33, 6), (31, 33, 5), (32, 33, 6), (33, 33, 6), (34, 33, 5), (35, 33, 6), (36, 33, 6),
            (30, 34, 6), (31, 34, 5), (32, 34, 1), (33, 34, 6), (34, 34, 1), (35, 34, 5), (36, 34, 6),
            (30, 35, 6), (31, 35, 6), (32, 35, 6), (33, 35, 6), (34, 35, 6), (35, 35, 6), (36, 35, 6),
        ];
        for &(px, py, pal_idx) in &paws {
            let target_x = if facing == -1 {
                (CANVAS_W - 1) as i32 - px
            } else {
                px
            };
            let target_y = (base_y as i32) + py;
            if target_x >= 0 && (target_x as usize) < CANVAS_W && target_y >= 0 && (target_y as usize) < CANVAS_H {
                buf[target_y as usize * CANVAS_W + target_x as usize] = PALETTE_BGRA[pal_idx as usize];
            }
        }
    }

    // Redraw shifted pupils for glance if eyes_dx != 0 || eyes_dy != 0
    if (eyes_dx != 0 || eyes_dy != 0) && !happy_eyes && !blink {
        for ep in &eye_pixels {
            let shifted_x = ep.x as i32 + (eyes_dx * facing);
            let target_x = if facing == -1 {
                (CANVAS_W - 1) as i32 - shifted_x
            } else {
                shifted_x
            };
            let shifted_y = (ep.y as i32 + eyes_dy).clamp(0, (FRAME_H - 1) as i32);
            let target_y = (base_y as i32) + shifted_y;
            if target_x >= 0 && (target_x as usize) < CANVAS_W && target_y >= 0 && (target_y as usize) < CANVAS_H {
                buf[target_y as usize * CANVAS_W + target_x as usize] = PALETTE_BGRA[1]; // Black pupil
            }
        }
    }

    // 4. Draw Hat if equipped
    if hat != HatType::None {
        let (anchor_cx, anchor_top) = HEAD_ANCHORS[frame_idx % NUM_FRAMES];
        let hat_rows: &[&str] = match hat {
            HatType::Top => &HAT_TOP,
            HatType::Chef => &HAT_CHEF,
            HatType::Fedora => &HAT_FEDORA,
            HatType::Helicopter => {
                let phase = (hat_anim_tick / 130) % 2;
                if phase == 0 {
                    &HAT_HELI0
                } else {
                    &HAT_HELI1
                }
            }
            HatType::None => &[],
        };

        if !hat_rows.is_empty() {
            let hat_h = hat_rows.len() as i32;
            let hat_w = hat_rows[0].len() as i32;
            let hy = (base_y as i32) + anchor_top - hat_h + 1; // 1px overlap onto head
            let hx = (anchor_cx - (hat_w as f32 / 2.0)).round() as i32;

            for r in 0..hat_h {
                let row_chars = hat_rows[r as usize].as_bytes();
                for c in 0..hat_w {
                    let ch = row_chars[c as usize] as char;
                    if let Some(col) = hat_color_bgra(ch) {
                        let hat_px_x = hx + c;
                        let target_x = if facing == -1 {
                            (CANVAS_W - 1) as i32 - hat_px_x
                        } else {
                            hat_px_x
                        };
                        let target_y = hy + r;
                        if target_x >= 0 && (target_x as usize) < CANVAS_W && target_y >= 0 && (target_y as usize) < CANVAS_H {
                            buf[target_y as usize * CANVAS_W + target_x as usize] = col;
                        }
                    }
                }
            }
        }
    }

    // 5. Draw Thought Bubble (persistent when thinking)
    if let Some(phase) = thought_phase {
        let (anchor_cx, anchor_top) = HEAD_ANCHORS[frame_idx % NUM_FRAMES];
        let cx = if facing == -1 {
            ((CANVAS_W as f32) - anchor_cx).round() as i32
        } else {
            anchor_cx.round() as i32
        };
        let top = (base_y as i32) + anchor_top;

        // Draw bubble dots and cloud
        fill_rect(buf, cx + 2, top - 3, 1, 1, COLOR_BUBBLE_BGRA);
        fill_rect(buf, cx + 4, top - 6, 2, 2, COLOR_BUBBLE_BGRA);
        fill_rect(buf, cx + 6, top - 13, 10, 5, COLOR_BUBBLE_BGRA);
        fill_rect(buf, cx + 7, top - 14, 8, 7, COLOR_BUBBLE_BGRA);

        // Typing dots inside cloud
        for n in 0..phase.min(3) {
            fill_rect(buf, cx + 8 + (n as i32 * 3), top - 11, 1, 1, COLOR_TYPING_DOTS_BGRA);
        }
    }

    // 6. Draw Alert Mark ("!" or "!?")
    if let Some(m) = mark {
        let two = m == "!?";
        let bx: i32 = if two { 39 } else { 44 };
        fill_rect(buf, bx, 1, 3, 6, COLOR_BUBBLE_BGRA); // "!" bar
        fill_rect(buf, bx, 9, 3, 2, COLOR_BUBBLE_BGRA); // "!" dot
        if two {
            let qx = bx + 5;
            fill_rect(buf, qx, 1, 4, 2, COLOR_BUBBLE_BGRA);     // "?" top bar
            fill_rect(buf, qx + 3, 3, 2, 2, COLOR_BUBBLE_BGRA); // right side
            fill_rect(buf, qx + 1, 5, 2, 2, COLOR_BUBBLE_BGRA); // hook to center
            fill_rect(buf, qx + 1, 9, 2, 2, COLOR_BUBBLE_BGRA); // dot
        }
    }

    // 7. Draw Sleep Z's
    if let Some(phase) = zzz_phase {
        let draw_z = |buf: &mut [[u8; 4]; CANVAS_W * CANVAS_H], x: i32, y: i32, s: i32| {
            fill_rect(buf, x, y, s, 1, COLOR_BUBBLE_BGRA); // top bar
            for k in 0..(s - 2) {
                fill_rect(buf, x + s - 2 - k, y + 1 + k, 1, 1, COLOR_BUBBLE_BGRA); // diagonal
            }
            fill_rect(buf, x, y + s - 1, s, 1, COLOR_BUBBLE_BGRA); // bottom bar
        };

        if phase == 0 {
            draw_z(buf, 40, 6, 4);
            draw_z(buf, 45, 1, 5);
        } else {
            draw_z(buf, 41, 4, 4);
            draw_z(buf, 46, 0, 5);
        }
    }
}

/// Helper to fill a rectangle on the 51x48 logical canvas.
fn fill_rect(
    buf: &mut [[u8; 4]; CANVAS_W * CANVAS_H],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    col: [u8; 4],
) {
    for r in 0..h {
        for c in 0..w {
            let px = x + c;
            let py = y + r;
            if px >= 0 && (px as usize) < CANVAS_W && py >= 0 && (py as usize) < CANVAS_H {
                buf[py as usize * CANVAS_W + px as usize] = col;
            }
        }
    }
}

/// Draws a cute 5x5 pixelated heart at logical canvas coordinates (hx, hy).
pub fn draw_heart(buf: &mut [[u8; 4]; CANVAS_W * CANVAS_H], hx: i32, hy: i32) {
    const HEART_MAP: [&str; 5] = [
        ".#.#.",
        "#####",
        "#####",
        ".###.",
        "..#..",
    ];
    for (r, row) in HEART_MAP.iter().enumerate() {
        for (c, ch) in row.chars().enumerate() {
            if ch == '#' {
                let px = hx + c as i32;
                let py = hy + r as i32;
                if px >= 0 && (px as usize) < CANVAS_W && py >= 0 && (py as usize) < CANVAS_H {
                    buf[py as usize * CANVAS_W + px as usize] = common::COLOR_HEART_BGRA;
                }
            }
        }
    }
}

/// Draws a cute 6x6 pixel chocolate chip cookie snack at logical canvas coordinates (sx, sy).
pub fn draw_snack(buf: &mut [[u8; 4]; CANVAS_W * CANVAS_H], sx: i32, sy: i32) {
    const SNACK_MAP: [&str; 6] = [
        ".cccc.",
        "cooccc",
        "ccoccc",
        "cccooc",
        "cooccc",
        ".cccc.",
    ];
    for (r, row) in SNACK_MAP.iter().enumerate() {
        for (c, ch) in row.chars().enumerate() {
            let color = match ch {
                'c' => [35, 120, 205, 255], // Cookie golden brown BGRA
                'o' => [15, 45, 80, 255],   // Dark chocolate chip
                _ => continue,
            };
            let px = sx + c as i32;
            let py = sy + r as i32;
            if px >= 0 && (px as usize) < CANVAS_W && py >= 0 && (py as usize) < CANVAS_H {
                buf[py as usize * CANVAS_W + px as usize] = color;
            }
        }
    }
}

/// Draws a floating pixel-art Z letter for sleep at logical canvas coordinates (zx, zy).
pub fn draw_zzz(buf: &mut [[u8; 4]; CANVAS_W * CANVAS_H], zx: i32, zy: i32, small: bool) {
    let color = [240, 210, 90, 255]; // Soft dreamy amber BGRA
    let map: &[&str] = if small {
        &[
            "###",
            "..#",
            ".#.",
            "#..",
            "###",
        ]
    } else {
        &[
            "####",
            "...#",
            "..#.",
            ".#..",
            "####",
        ]
    };
    for (r, row) in map.iter().enumerate() {
        for (c, ch) in row.chars().enumerate() {
            if ch == '#' {
                let px = zx + c as i32;
                let py = zy + r as i32;
                if px >= 0 && (px as usize) < CANVAS_W && py >= 0 && (py as usize) < CANVAS_H {
                    buf[py as usize * CANVAS_W + px as usize] = color;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_happy_eyes_and_heart_rendering() {
        let mut buf = [[0u8; 4]; CANVAS_W * CANVAS_H];
        composite_logical_frame(
            &mut buf, 0, 0, 1, false, false, false, true, 0, 0, false,
            HatType::None, 0, None, None, None,
        );
        // Happy carets draw black pixels on row 8 & 9
        let black_count = buf.iter().filter(|&&p| p == PALETTE_BGRA[1]).count();
        assert_eq!(black_count, 8, "Happy eyes must draw exactly 8 caret pixels");

        // Test heart drawing
        draw_heart(&mut buf, 10, 10);
        let heart_count = buf.iter().filter(|&&p| p == common::COLOR_HEART_BGRA).count();
        assert_eq!(heart_count, 16, "5x5 heart must draw exactly 16 pink pixels");
    }

    #[test]
    fn test_sit_legs_and_snack_rendering() {
        let mut buf = [[0u8; 4]; CANVAS_W * CANVAS_H];
        composite_logical_frame(
            &mut buf, 0, 5, 1, false, false, false, true, 0, 0, true,
            HatType::None, 0, None, None, None,
        );
        let has_opaque = buf.iter().any(|p| p[3] > 0);
        assert!(has_opaque, "Sitting frame must render opaque pixels");

        // Test snack
        draw_snack(&mut buf, 20, 20);
        let snack_pixels = buf.iter().filter(|&&p| p == [35, 120, 205, 255]).count();
        assert!(snack_pixels > 0, "Snack cookie must render cookie pixels");

        // Test zzz
        draw_zzz(&mut buf, 25, 5, false);
        let zzz_pixels = buf.iter().filter(|&&p| p == [240, 210, 90, 255]).count();
        assert!(zzz_pixels > 0, "ZZZ must render amber pixels");
    }

    #[test]
    fn test_frames_embedded_size_and_count() {
        for i in 0..NUM_FRAMES {
            let f = get_frame(i);
            assert_eq!(f.len(), FRAME_W * FRAME_H);
            let has_opaque = f.iter().any(|&p| p != 0);
            assert!(has_opaque, "Frame {} must contain opaque pixels", i);
        }
    }

    #[test]
    fn test_head_anchors_bounds() {
        assert_eq!(HEAD_ANCHORS.len(), NUM_FRAMES);
        for (idx, &(cx, top)) in HEAD_ANCHORS.iter().enumerate() {
            assert!(cx >= 20.0 && cx <= 36.0, "Frame {} cx {} out of bounds", idx, cx);
            assert!(top >= 0 && top <= 6, "Frame {} top {} out of bounds", idx, top);
        }
    }

    #[test]
    fn test_composite_logical_frame_with_all_hats() {
        let mut buf = [[0u8; 4]; CANVAS_W * CANVAS_H];

        // 1. None hat
        composite_logical_frame(
            &mut buf, 0, 0, 1, false, false, false, false, 0, 0, false,
            HatType::None, 0, None, None, None,
        );
        let opaque_count_none = buf.iter().filter(|p| p[3] > 0).count();
        assert!(opaque_count_none > 100, "Base frame 0 must have opaque pixels");

        // 2. Top hat
        composite_logical_frame(
            &mut buf, 0, 0, 1, false, false, false, false, 0, 0, false,
            HatType::Top, 0, None, None, None,
        );
        let opaque_count_top = buf.iter().filter(|p| p[3] > 0).count();
        assert!(opaque_count_top > opaque_count_none, "Top hat must add opaque pixels");

        // 3. Chef hat
        composite_logical_frame(
            &mut buf, 0, 0, 1, false, false, false, false, 0, 0, false,
            HatType::Chef, 0, None, None, None,
        );
        let opaque_count_chef = buf.iter().filter(|p| p[3] > 0).count();
        assert!(opaque_count_chef > opaque_count_none, "Chef hat must add opaque pixels");

        // 4. Fedora
        composite_logical_frame(
            &mut buf, 0, 0, 1, false, false, false, false, 0, 0, false,
            HatType::Fedora, 0, None, None, None,
        );
        let opaque_count_fedora = buf.iter().filter(|p| p[3] > 0).count();
        assert!(opaque_count_fedora > opaque_count_none, "Fedora must add opaque pixels");

        // 5. Helicopter (Phase 0 and Phase 1)
        composite_logical_frame(
            &mut buf, 0, 0, 1, false, false, false, false, 0, 0, false,
            HatType::Helicopter, 0, None, None, None,
        );
        let opaque_count_heli0 = buf.iter().filter(|p| p[3] > 0).count();
        assert!(opaque_count_heli0 > opaque_count_none, "Heli0 must add opaque pixels");

        composite_logical_frame(
            &mut buf, 0, 0, 1, false, false, false, false, 0, 0, false,
            HatType::Helicopter, 130, None, None, None,
        );
        let opaque_count_heli1 = buf.iter().filter(|p| p[3] > 0).count();
        assert!(opaque_count_heli1 > opaque_count_none, "Heli1 must add opaque pixels");
    }

    #[test]
    fn test_overlays_thought_bubble_and_alerts() {
        let mut buf = [[0u8; 4]; CANVAS_W * CANVAS_H];

        // Thought bubble
        composite_logical_frame(
            &mut buf, 26, 0, 1, false, false, false, false, 0, 0, false,
            HatType::None, 0, Some(2), None, None,
        );
        let bubble_pixel_found = buf.iter().any(|&p| p == COLOR_BUBBLE_BGRA);
        assert!(bubble_pixel_found, "Thought bubble must render bubble color pixels");

        // Alert mark "!?"
        composite_logical_frame(
            &mut buf, 27, 0, 1, false, false, false, false, 0, 0, false,
            HatType::None, 0, None, Some("!?"), None,
        );
        let alert_found = buf.iter().any(|&p| p == COLOR_BUBBLE_BGRA);
        assert!(alert_found, "Alert mark must render bubble color pixels");

        // Sleep Z's
        composite_logical_frame(
            &mut buf, 0, 2, 1, false, true, false, false, 0, 0, false,
            HatType::None, 0, None, None, Some(0),
        );
        let z_found = buf.iter().any(|&p| p == COLOR_BUBBLE_BGRA);
        assert!(z_found, "Sleep Z's must render bubble color pixels");
    }
}

