use std::ptr::null_mut;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::assets::{CANVAS_H, CANVAS_W};

/// 32-bit ARGB Top-Down DIB Section Framebuffer for native layered window compositing.
pub struct LayeredFrameBuffer {
    pub mem_dc: HDC,
    pub hbitmap: HBITMAP,
    pub old_bitmap: HGDIOBJ,
    pub bits: *mut u8,
    pub width: i32,
    pub height: i32,
    pub scale: u32,
}

impl LayeredFrameBuffer {
    pub unsafe fn new(width: i32, height: i32, scale: u32) -> Self {
        let screen_dc = GetDC(null_mut());
        let mem_dc = CreateCompatibleDC(screen_dc);
        ReleaseDC(null_mut(), screen_dc);

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // Negative value creates a Top-Down DIB
                biPlanes: 1,
                biBitCount: 32,    // 32-bit ARGB (BGRA in memory)
                biCompression: BI_RGB,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD { rgbBlue: 0, rgbGreen: 0, rgbRed: 0, rgbReserved: 0 }],
        };

        let mut bits: *mut core::ffi::c_void = null_mut();
        let hbitmap = CreateDIBSection(
            mem_dc,
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            null_mut(),
            0,
        );

        let old_bitmap = SelectObject(mem_dc, hbitmap);

        Self {
            mem_dc,
            hbitmap,
            old_bitmap,
            bits: bits as *mut u8,
            width,
            height,
            scale,
        }
    }

    /// Blits a 51x48 logical canvas onto the physical DIB buffer using integer nearest-neighbor scaling.
    pub unsafe fn blit_from_logical(&mut self, logical: &[[u8; 4]; CANVAS_W * CANVAS_H]) {
        if self.bits.is_null() {
            return;
        }

        let s = self.scale as usize;
        let w = self.width as usize;
        let h = self.height as usize;

        // Perform nearest-neighbor scaling directly into DIB memory
        for py in 0..h {
            let ly = (py / s).min(CANVAS_H - 1);
            let row_offset = py * w;
            let logical_row_offset = ly * CANVAS_W;

            for px in 0..w {
                let lx = (px / s).min(CANVAS_W - 1);
                let col = logical[logical_row_offset + lx];
                let dst_ptr = self.bits.add((row_offset + px) * 4);
                std::ptr::copy_nonoverlapping(col.as_ptr(), dst_ptr, 4);
            }
        }
    }

    /// Per-pixel hit testing: checks whether the pixel at local coordinates is opaque (alpha > 0).
    pub unsafe fn is_pixel_opaque(&self, local_x: i32, local_y: i32) -> bool {
        if self.bits.is_null() {
            return false;
        }
        if local_x < 0 || local_x >= self.width || local_y < 0 || local_y >= self.height {
            return false;
        }

        let offset = (local_y as usize * self.width as usize + local_x as usize) * 4;
        let alpha = *self.bits.add(offset + 3); // Byte 3 is Alpha in BGRA
        alpha > 0
    }

    /// Presents the buffer to the Desktop Window Manager via `UpdateLayeredWindow`.
    pub unsafe fn present(&self, hwnd: HWND, x: i32, y: i32) -> BOOL {
        let pt_dst = POINT { x, y };
        let sz = SIZE {
            cx: self.width,
            cy: self.height,
        };
        let pt_src = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,        // Master alpha
            AlphaFormat: AC_SRC_ALPHA as u8, // Pre-multiplied 32-bit ARGB
        };

        UpdateLayeredWindow(
            hwnd,
            null_mut(),
            &pt_dst,
            &sz,
            self.mem_dc,
            &pt_src,
            0,
            &blend,
            ULW_ALPHA,
        )
    }

    /// Reallocates the DIB section to match a new size.
    pub unsafe fn resize(&mut self, width: i32, height: i32, scale: u32) {
        if self.width == width && self.height == height && self.scale == scale {
            return;
        }
        let mut new_fb = Self::new(width, height, scale);
        std::mem::swap(self, &mut new_fb);
        // `new_fb` now holds the old framebuffer and will drop/destroy the old GDI handles cleanly.
    }

    pub unsafe fn destroy(&mut self) {
        if !self.mem_dc.is_null() {
            SelectObject(self.mem_dc, self.old_bitmap);
            DeleteObject(self.hbitmap);
            DeleteDC(self.mem_dc);
            self.mem_dc = null_mut();
            self.hbitmap = null_mut();
            self.old_bitmap = null_mut();
            self.bits = null_mut();
        }
    }
}

impl Drop for LayeredFrameBuffer {
    fn drop(&mut self) {
        unsafe {
            self.destroy();
        }
    }
}
