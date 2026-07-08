//! Extract an app's shell icon (.lnk/.exe) as a small base64 PNG.
use std::ffi::c_void;

use base64::Engine;
use image::ImageEncoder;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL};

/// Icon for a file path -> data: URL, or None.
pub fn extract(path: &str) -> Option<String> {
    unsafe {
        let wpath: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
        let mut shfi = SHFILEINFOW::default();
        let r = SHGetFileInfoW(
            PCWSTR(wpath.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut shfi),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        );
        if r == 0 || shfi.hIcon.is_invalid() {
            return None;
        }
        let hicon = shfi.hIcon;
        let sz = 32i32;

        let screen = GetDC(None);
        let memdc = CreateCompatibleDC(Some(screen));
        let mut bi = BITMAPINFO::default();
        bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = sz;
        bi.bmiHeader.biHeight = -sz; // top-down
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = BI_RGB.0 as u32;
        let mut bits: *mut c_void = std::ptr::null_mut();
        let dib = match CreateDIBSection(Some(memdc), &bi, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(d) => d,
            Err(_) => {
                let _ = DestroyIcon(hicon);
                let _ = DeleteDC(memdc);
                ReleaseDC(None, screen);
                return None;
            }
        };
        let old = SelectObject(memdc, HGDIOBJ(dib.0));
        let _ = DrawIconEx(memdc, 0, 0, hicon, sz, sz, 0, None, DI_NORMAL);

        let n = (sz * sz * 4) as usize;
        let buf = std::slice::from_raw_parts(bits as *const u8, n).to_vec();

        SelectObject(memdc, old);
        let _ = DeleteObject(HGDIOBJ(dib.0));
        let _ = DeleteDC(memdc);
        ReleaseDC(None, screen);
        let _ = DestroyIcon(hicon);

        // BGRA -> RGBA; if the icon left no alpha, treat it as opaque.
        let any_alpha = buf.chunks_exact(4).any(|p| p[3] != 0);
        let mut img = image::RgbaImage::new(sz as u32, sz as u32);
        for (i, px) in buf.chunks_exact(4).enumerate() {
            let a = if any_alpha { px[3] } else { 255 };
            img.put_pixel(
                (i as u32) % (sz as u32),
                (i as u32) / (sz as u32),
                image::Rgba([px[2], px[1], px[0], a]),
            );
        }
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(img.as_raw(), sz as u32, sz as u32, image::ExtendedColorType::Rgba8)
            .ok()?;
        Some(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&png)
        ))
    }
}
