//! 32-bit ARGB pre-multiplied palette and overlay colors for Sidecrab.
//!
//! Windows DIB sections with `AC_SRC_ALPHA` expect pixels in BGRA byte order
//! where R, G, and B are pre-multiplied by Alpha:
//! `C_stored = floor((C_straight * A + 127) / 255)`.
//! For opaque colors (A = 255), the pre-multiplied values match straight values.

/// Base 6-color palette (index 0 is transparent, indices 1..6 are opaque body colors).
/// Stored in BGRA byte order: `[Blue, Green, Red, Alpha]`.
pub const PALETTE_BGRA: [[u8; 4]; 7] = [
    [0, 0, 0, 0],         // 0: Transparent
    [0, 0, 0, 255],       // 1: Black (#000000)
    [62, 54, 52, 255],    // 2: Dark gray (#34363e)
    [100, 88, 84, 255],   // 3: Slate gray (#545864)
    [200, 170, 150, 255], // 4: Light blue (#96aac8)
    [77, 105, 191, 255],  // 5: Terracotta (#bf694d)
    [87, 119, 217, 255],  // 6: Coral (#d97757)
];

/// Overlay color for thought bubbles, sleep Z's, and "!?" alert marks: #f2e7dc
pub const COLOR_BUBBLE_BGRA: [u8; 4] = [220, 231, 242, 255];

/// Overlay color for typing dots inside thought bubble: #6b6b74
pub const COLOR_TYPING_DOTS_BGRA: [u8; 4] = [116, 107, 107, 255];

/// Overlay color for floating pixel hearts: #ff3b77 (vibrant pink BGRA)
pub const COLOR_HEART_BGRA: [u8; 4] = [119, 59, 255, 255];

/// Hat colors map (BGRA pre-multiplied):
/// - 'k': #1d1c22 (black)
/// - 'b': #7a4a28 (hat band brown)
/// - 'w': #f2efe9 (chef white)
/// - 'd': #d8d3c8 (chef shade)
/// - 'r': #c8372d (heli cap red)
/// - 'y': #e8b83a (heli cap accent)
/// - 'g': #5a5f6a (rotor gray)
pub fn hat_color_bgra(c: char) -> Option<[u8; 4]> {
    match c {
        'k' => Some([34, 28, 29, 255]),
        'b' => Some([40, 74, 122, 255]),
        'w' => Some([233, 239, 242, 255]),
        'd' => Some([200, 211, 216, 255]),
        'r' => Some([45, 55, 200, 255]),
        'y' => Some([58, 184, 232, 255]),
        'g' => Some([106, 95, 90, 255]),
        _ => None,
    }
}
