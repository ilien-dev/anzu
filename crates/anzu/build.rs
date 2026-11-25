//! Embeds the app icon in the Windows executable, so Explorer, the taskbar and shortcuts
//! show the Anzu logo instead of the generic one. The icon is drawn from the same geometry
//! as the logo on screen.

#[allow(dead_code)]
#[path = "src/ui/mark.rs"]
mod mark;

use std::path::PathBuf;

/// Sizes Windows picks from, for lists, tiles and high DPI.
const SIZES: [u32; 7] = [16, 20, 24, 32, 48, 64, 256];
/// Each size is drawn this many times larger and averaged down, for smooth edges.
const SUPERSAMPLE: u32 = 4;

fn main() {
    println!("cargo:rerun-if-changed=src/ui/mark.rs");
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let ico = out.join("anzu.ico");
    std::fs::write(&ico, ico_file()).expect("write the icon");
    let rc = out.join("anzu.rc");
    let ico_path = ico.display().to_string().replace('\\', "/");
    std::fs::write(&rc, format!("1 ICON \"{ico_path}\"\n")).expect("write the resource script");
    embed_resource::compile(&rc, embed_resource::NONE).manifest_optional().expect("embed the icon");
}

/// The icon at `size`, drawn larger and averaged down, as RGBA.
fn smooth_icon(size: u32) -> Vec<u8> {
    let big = size * SUPERSAMPLE;
    // The cut follows the final size, not the supersampled one, so small icons get the small cut.
    let shape = mark::Shape::for_size(size as f32 * (1.0 - 2.0 * mark::TILE_INSET));
    let src = mark::icon_rgba_with(big, shape, mark::ICON_BACKGROUND, mark::ICON_MARK);
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            // Average with alpha weighting, so transparent pixels do not darken the edges.
            let (mut r, mut g, mut b, mut a) = (0u32, 0u32, 0u32, 0u32);
            for sy in 0..SUPERSAMPLE {
                for sx in 0..SUPERSAMPLE {
                    let i = (((y * SUPERSAMPLE + sy) * big + x * SUPERSAMPLE + sx) * 4) as usize;
                    let alpha = src[i + 3] as u32;
                    r += src[i] as u32 * alpha;
                    g += src[i + 1] as u32 * alpha;
                    b += src[i + 2] as u32 * alpha;
                    a += alpha;
                }
            }
            let samples = SUPERSAMPLE * SUPERSAMPLE;
            let pixel = match (r.checked_div(a), g.checked_div(a), b.checked_div(a)) {
                (Some(r), Some(g), Some(b)) => [r as u8, g as u8, b as u8, (a / samples) as u8],
                _ => [0, 0, 0, 0],
            };
            pixels.extend_from_slice(&pixel);
        }
    }
    pixels
}

/// One image of the .ico: a 32-bit bottom-up bitmap followed by an empty AND mask.
fn bitmap(size: u32) -> Vec<u8> {
    let rgba = smooth_icon(size);
    let mask_row = size.div_ceil(32) * 4;
    let mut data = Vec::new();
    for value in [40u32, size, size * 2] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&32u16.to_le_bytes());
    for value in [0u32, size * size * 4 + mask_row * size, 0, 0, 0, 0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    for y in (0..size).rev() {
        for x in 0..size {
            let i = ((y * size + x) * 4) as usize;
            data.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i], rgba[i + 3]]);
        }
    }
    data.extend(std::iter::repeat_n(0u8, (mask_row * size) as usize));
    data
}

/// A .ico file with every size of [`SIZES`].
fn ico_file() -> Vec<u8> {
    let images: Vec<(u32, Vec<u8>)> = SIZES.iter().map(|&size| (size, bitmap(size))).collect();
    let mut file = Vec::new();
    file.extend_from_slice(&0u16.to_le_bytes());
    file.extend_from_slice(&1u16.to_le_bytes());
    file.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for (size, data) in &images {
        // A width or height of 256 is written as 0.
        let side = if *size >= 256 { 0 } else { *size as u8 };
        file.extend_from_slice(&[side, side, 0, 0]);
        file.extend_from_slice(&1u16.to_le_bytes());
        file.extend_from_slice(&32u16.to_le_bytes());
        file.extend_from_slice(&(data.len() as u32).to_le_bytes());
        file.extend_from_slice(&offset.to_le_bytes());
        offset += data.len() as u32;
    }
    for (_, data) in images {
        file.extend_from_slice(&data);
    }
    file
}
