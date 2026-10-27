use std::alloc::{alloc as sys_alloc, dealloc as sys_dealloc, Layout};
use std::collections::HashMap;
use vtracer::{ColorImage, Config, FitMode, Hierarchical, Preset};

// ---------------------------------------------------------------------------
// Tunables
// ---------------------------------------------------------------------------

/// Small images are upscaled (Catmull-Rom) so the *longest side* is about this
/// many pixels BEFORE color quantization + tracing. This is the biggest fix for
/// "staircase" edges and non-round circles: vtracer fits curves to pixel
/// boundaries, so more (sub)pixels = smoother curves.
/// Lower it (e.g. 1600) if tracing feels slow, raise it for even smoother output.
const TARGET_LONG_SIDE: usize = 2400;

/// Never upscale by more than this factor.
const MAX_UPSCALE: usize = 4;

/// Replace small, solid, round blobs (eyes, dots, buttons) with perfect circles.
const SNAP_CIRCLES: bool = true;

/// Safety cap on the number of snapped circles.
const MAX_SNAPPED_CIRCLES: usize = 400;

// ---------------------------------------------------------------------------
// WebAssembly memory helpers
// ---------------------------------------------------------------------------

/// Allocate memory buffer inside WebAssembly linear memory
#[no_mangle]
pub extern "C" fn alloc(size: usize) -> *mut u8 {
    let layout = Layout::from_size_align(size, 4).unwrap();
    unsafe { sys_alloc(layout) }
}

/// Deallocate memory buffer inside WebAssembly linear memory
#[no_mangle]
pub extern "C" fn dealloc(ptr: *mut u8, size: usize) {
    if !ptr.is_null() && size > 0 {
        let layout = Layout::from_size_align(size, 4).unwrap();
        unsafe { sys_dealloc(ptr, layout) }
    }
}

/// Free a string returned by wasm_vectorize
#[no_mangle]
pub extern "C" fn wasm_free_string(ptr: *mut u8, len: usize) {
    dealloc(ptr, len);
}

// ---------------------------------------------------------------------------
// Background detection / removal
// ---------------------------------------------------------------------------

/// Detect dominant background color by perimeter mode (most frequent border color)
#[no_mangle]
pub extern "C" fn wasm_detect_background(
    ptr: *const u8,
    width: u32,
    height: u32,
    bg_out_ptr: *mut u8,
) {
    if ptr.is_null() || bg_out_ptr.is_null() || width == 0 || height == 0 {
        return;
    }

    let pixels = unsafe { std::slice::from_raw_parts(ptr, (width * height * 4) as usize) };
    let mut hist: HashMap<u16, (u64, u64, u64, u64)> = HashMap::new();

    let mut sample_pixel = |x: u32, y: u32| {
        let idx = ((y * width + x) * 4) as usize;
        let a = pixels[idx + 3];
        if a > 32 {
            let r = pixels[idx];
            let g = pixels[idx + 1];
            let b = pixels[idx + 2];
            let key = (((r as u16) >> 3) << 10) | (((g as u16) >> 3) << 5) | ((b as u16) >> 3);
            let entry = hist.entry(key).or_insert((0, 0, 0, 0));
            entry.0 += r as u64;
            entry.1 += g as u64;
            entry.2 += b as u64;
            entry.3 += 1;
        }
    };

    // Sample all 4 borders of the image
    for x in 0..width {
        sample_pixel(x, 0);
        sample_pixel(x, height - 1);
    }
    for y in 0..height {
        sample_pixel(0, y);
        sample_pixel(width - 1, y);
    }

    let out = unsafe { std::slice::from_raw_parts_mut(bg_out_ptr, 4) };
    if let Some((_, &(sr, sg, sb, count))) = hist.iter().max_by_key(|(_, v)| v.3) {
        if count > 0 {
            out[0] = (sr / count) as u8;
            out[1] = (sg / count) as u8;
            out[2] = (sb / count) as u8;
            out[3] = 255;
            return;
        }
    }

    out[0] = 255;
    out[1] = 255;
    out[2] = 255;
    out[3] = 255;
}

/// Fast in-place background keyout with Euclidean color distance tolerance
#[no_mangle]
pub extern "C" fn wasm_remove_background(
    ptr: *mut u8,
    len: usize,
    target_r: u8,
    target_g: u8,
    target_b: u8,
    tolerance: u32,
) -> u32 {
    if ptr.is_null() || len == 0 {
        return 0;
    }

    let pixels = unsafe { std::slice::from_raw_parts_mut(ptr, len) };
    let tol_sq = (tolerance * tolerance) as i64;
    let tr = target_r as i64;
    let tg = target_g as i64;
    let tb = target_b as i64;

    let mut removed_count: u32 = 0;
    let total_pixels = len / 4;

    for i in 0..total_pixels {
        let offset = i * 4;
        let a = pixels[offset + 3];
        if a == 0 {
            continue;
        }

        let r = pixels[offset] as i64;
        let g = pixels[offset + 1] as i64;
        let b = pixels[offset + 2] as i64;

        let dist_sq = (r - tr) * (r - tr) + (g - tg) * (g - tg) + (b - tb) * (b - tb);
        if dist_sq <= tol_sq {
            pixels[offset + 3] = 0;
            removed_count += 1;
        }
    }

    removed_count
}

// ---------------------------------------------------------------------------
// Blur
// ---------------------------------------------------------------------------

/// 2D separable box blur filter
#[no_mangle]
pub extern "C" fn wasm_box_blur(ptr: *mut u8, width: u32, height: u32, radius: u32) {
    if ptr.is_null() || width == 0 || height == 0 || radius == 0 {
        return;
    }

    let w = width as usize;
    let h = height as usize;
    let total_pixels = w * h;
    let pixels = unsafe { std::slice::from_raw_parts_mut(ptr, total_pixels * 4) };

    let mut temp = vec![0u8; total_pixels * 4];
    let r = radius as usize;

    // Horizontal pass
    for y in 0..h {
        for x in 0..w {
            let mut sum_r = 0u32;
            let mut sum_g = 0u32;
            let mut sum_b = 0u32;
            let mut sum_a = 0u32;
            let mut count = 0u32;

            let x_start = if x > r { x - r } else { 0 };
            let x_end = std::cmp::min(x + r, w - 1);

            for ix in x_start..=x_end {
                let idx = (y * w + ix) * 4;
                sum_r += pixels[idx] as u32;
                sum_g += pixels[idx + 1] as u32;
                sum_b += pixels[idx + 2] as u32;
                sum_a += pixels[idx + 3] as u32;
                count += 1;
            }

            let out_idx = (y * w + x) * 4;
            temp[out_idx] = (sum_r / count) as u8;
            temp[out_idx + 1] = (sum_g / count) as u8;
            temp[out_idx + 2] = (sum_b / count) as u8;
            temp[out_idx + 3] = (sum_a / count) as u8;
        }
    }

    // Vertical pass
    for x in 0..w {
        for y in 0..h {
            let mut sum_r = 0u32;
            let mut sum_g = 0u32;
            let mut sum_b = 0u32;
            let mut sum_a = 0u32;
            let mut count = 0u32;

            let y_start = if y > r { y - r } else { 0 };
            let y_end = std::cmp::min(y + r, h - 1);

            for iy in y_start..=y_end {
                let idx = (iy * w + x) * 4;
                sum_r += temp[idx] as u32;
                sum_g += temp[idx + 1] as u32;
                sum_b += temp[idx + 2] as u32;
                sum_a += temp[idx + 3] as u32;
                count += 1;
            }

            let out_idx = (y * w + x) * 4;
            pixels[out_idx] = (sum_r / count) as u8;
            pixels[out_idx + 1] = (sum_g / count) as u8;
            pixels[out_idx + 2] = (sum_b / count) as u8;
            pixels[out_idx + 3] = (sum_a / count) as u8;
        }
    }
}

// ---------------------------------------------------------------------------
// Upscaling (Catmull-Rom bicubic, alpha-aware)
// ---------------------------------------------------------------------------

fn catmull_rom_weights(t: f32) -> [f32; 4] {
    let t2 = t * t;
    let t3 = t2 * t;
    [
        -0.5 * t3 + t2 - 0.5 * t,
        1.5 * t3 - 2.5 * t2 + 1.0,
        -1.5 * t3 + 2.0 * t2 + 0.5 * t,
        0.5 * t3 - 0.5 * t2,
    ]
}

/// For every output coordinate: the 4 source indices and their weights.
fn build_taps(out_len: usize, in_len: usize, scale: usize) -> Vec<([usize; 4], [f32; 4])> {
    let mut taps = Vec::with_capacity(out_len);
    let s = scale as f32;
    let max_i = in_len as i32 - 1;
    for o in 0..out_len {
        let src = (o as f32 + 0.5) / s - 0.5;
        let base_f = src.floor();
        let t = src - base_f;
        let base = base_f as i32;
        let wts = catmull_rom_weights(t);
        let mut idx = [0usize; 4];
        for k in 0..4 {
            let i = base - 1 + k as i32;
            idx[k] = i.clamp(0, max_i) as usize;
        }
        taps.push((idx, wts));
    }
    taps
}

/// Smoothly upscale an RGBA8 image by an integer factor.
/// Uses premultiplied alpha so transparent edges do not bleed color.
fn upscale_catmull_rom(src: &[u8], w: usize, h: usize, scale: usize) -> Vec<u8> {
    let ow = w * scale;
    let oh = h * scale;
    let xt = build_taps(ow, w, scale);
    let yt = build_taps(oh, h, scale);

    // Horizontal pass -> f32 premultiplied buffer (ow x h)
    let mut tmp = vec![0f32; ow * h * 4];
    for y in 0..h {
        let row = y * w * 4;
        for (ox, (idx, wt)) in xt.iter().enumerate() {
            let mut acc = [0f32; 4];
            for k in 0..4 {
                let p = row + idx[k] * 4;
                let a = src[p + 3] as f32;
                let af = a / 255.0;
                acc[0] += wt[k] * src[p] as f32 * af;
                acc[1] += wt[k] * src[p + 1] as f32 * af;
                acc[2] += wt[k] * src[p + 2] as f32 * af;
                acc[3] += wt[k] * a;
            }
            let o = (y * ow + ox) * 4;
            tmp[o..o + 4].copy_from_slice(&acc);
        }
    }

    // Vertical pass -> u8 straight alpha
    let mut out = vec![0u8; ow * oh * 4];
    for (oy, (idx, wt)) in yt.iter().enumerate() {
        for ox in 0..ow {
            let mut acc = [0f32; 4];
            for k in 0..4 {
                let p = (idx[k] * ow + ox) * 4;
                for c in 0..4 {
                    acc[c] += wt[k] * tmp[p + c];
                }
            }
            let a = acc[3].clamp(0.0, 255.0);
            let o = (oy * ow + ox) * 4;
            if a >= 0.5 {
                let inv = 255.0 / a;
                out[o] = (acc[0] * inv).round().clamp(0.0, 255.0) as u8;
                out[o + 1] = (acc[1] * inv).round().clamp(0.0, 255.0) as u8;
                out[o + 2] = (acc[2] * inv).round().clamp(0.0, 255.0) as u8;
                out[o + 3] = a.round() as u8;
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Color quantization
// ---------------------------------------------------------------------------

/// Perceptually weighted color distance squared (accounting for human eye sensitivity to green > red > blue)
#[inline(always)]
fn color_dist_sq(r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) -> u64 {
    let dr = r1 as i64 - r2 as i64;
    let dg = g1 as i64 - g2 as i64;
    let db = b1 as i64 - b2 as i64;
    (2 * dr * dr + 4 * dg * dg + 3 * db * db) as u64
}

/// Quantize pixels to dominant perceptual color clusters with anti-aliasing halo suppression.
///
/// `isolation` controls the final boundary-majority filter: a pixel is replaced only if it has
/// `<= isolation` same-colored neighbors. Use 1 for native-resolution images and 0 for upscaled
/// images (where only truly isolated noise pixels should be removed, so small round shapes keep
/// their outline).
pub fn quantize_palette_perceptual(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    k: usize,
    isolation: usize,
) -> Vec<(u8, u8, u8)> {
    if k == 0 || pixels.is_empty() {
        return Vec::new();
    }

    let total_pixels = width * height;

    // 1. Collect color histogram in 5-bit quantized space (32x32x32 bins)
    let mut hist: HashMap<u16, (u64, u64, u64, u64)> = HashMap::new();
    for i in 0..total_pixels {
        let idx = i * 4;
        if pixels[idx + 3] < 32 {
            continue; // ignore transparent
        }
        let r = pixels[idx];
        let g = pixels[idx + 1];
        let b = pixels[idx + 2];
        let key = (((r as u16) >> 3) << 10) | (((g as u16) >> 3) << 5) | ((b as u16) >> 3);
        let entry = hist.entry(key).or_insert((0, 0, 0, 0));
        entry.0 += r as u64;
        entry.1 += g as u64;
        entry.2 += b as u64;
        entry.3 += 1;
    }

    if hist.is_empty() {
        return vec![(255, 255, 255)];
    }

    // Minimum pixel count for a color bin to be considered a genuine cluster center (suppresses gradient fringe bins)
    let min_bin_count = std::cmp::max(20, (total_pixels as f64 * 0.001) as u64);

    let mut candidate_bins: Vec<(u8, u8, u8, u64)> = hist
        .values()
        .filter(|v| v.3 >= min_bin_count)
        .map(|&(sr, sg, sb, count)| ((sr / count) as u8, (sg / count) as u8, (sb / count) as u8, count))
        .collect();
    candidate_bins.sort_by(|a, b| b.3.cmp(&a.3));

    // Fallback if image has very few dominant bins
    if candidate_bins.is_empty() {
        candidate_bins = hist
            .values()
            .map(|&(sr, sg, sb, count)| ((sr / count) as u8, (sg / count) as u8, (sb / count) as u8, count))
            .collect();
        candidate_bins.sort_by(|a, b| b.3.cmp(&a.3));
    }

    let target_k = std::cmp::min(k, candidate_bins.len());
    let mut centers: Vec<(u8, u8, u8)> = Vec::with_capacity(target_k);

    // Pick top most frequent color
    centers.push((candidate_bins[0].0, candidate_bins[0].1, candidate_bins[0].2));

    // K-Means++ initialization with minimum perceptual distance constraint to prevent halo clustering
    let min_dist_threshold = 900u64; // distance ~30 in perceptual color space
    while centers.len() < target_k {
        let mut best_bin = None;
        let mut max_weighted_dist = 0u64;

        for (b_idx, bin) in candidate_bins.iter().enumerate() {
            let mut min_dist_to_center = u64::MAX;
            for center in &centers {
                let d = color_dist_sq(bin.0, bin.1, bin.2, center.0, center.1, center.2);
                if d < min_dist_to_center {
                    min_dist_to_center = d;
                }
            }
            // Only select bins that are not virtually identical to an existing center
            if min_dist_to_center >= min_dist_threshold {
                let weighted = min_dist_to_center.saturating_mul(bin.3);
                if weighted > max_weighted_dist {
                    max_weighted_dist = weighted;
                    best_bin = Some(b_idx);
                }
            }
        }

        match best_bin {
            Some(idx) => centers.push((candidate_bins[idx].0, candidate_bins[idx].1, candidate_bins[idx].2)),
            None => break, // All significant colors are already represented!
        }
    }

    // 2. Refine centers with 3 K-Means iterations
    let all_bins: Vec<(u8, u8, u8, u64)> = hist
        .values()
        .map(|&(sr, sg, sb, count)| ((sr / count) as u8, (sg / count) as u8, (sb / count) as u8, count))
        .collect();

    for _ in 0..3 {
        let mut sums: Vec<(u64, u64, u64, u64)> = vec![(0, 0, 0, 0); centers.len()];
        for bin in &all_bins {
            let mut min_d = u64::MAX;
            let mut best_c = 0;
            for (c_idx, c) in centers.iter().enumerate() {
                let d = color_dist_sq(bin.0, bin.1, bin.2, c.0, c.1, c.2);
                if d < min_d {
                    min_d = d;
                    best_c = c_idx;
                }
            }
            sums[best_c].0 += bin.0 as u64 * bin.3;
            sums[best_c].1 += bin.1 as u64 * bin.3;
            sums[best_c].2 += bin.2 as u64 * bin.3;
            sums[best_c].3 += bin.3;
        }

        for (c_idx, sum) in sums.iter().enumerate() {
            if sum.3 > 0 {
                centers[c_idx] = (
                    (sum.0 / sum.3) as u8,
                    (sum.1 / sum.3) as u8,
                    (sum.2 / sum.3) as u8,
                );
            }
        }
    }

    // 3. Remap all pixels to nearest dominant color
    for i in 0..total_pixels {
        let idx = i * 4;
        if pixels[idx + 3] < 32 {
            continue;
        }
        let r = pixels[idx];
        let g = pixels[idx + 1];
        let b = pixels[idx + 2];

        let mut min_d = u64::MAX;
        let mut best_c = 0;
        for (c_idx, c) in centers.iter().enumerate() {
            let d = color_dist_sq(r, g, b, c.0, c.1, c.2);
            if d < min_d {
                min_d = d;
                best_c = c_idx;
            }
        }
        pixels[idx] = centers[best_c].0;
        pixels[idx + 1] = centers[best_c].1;
        pixels[idx + 2] = centers[best_c].2;
    }

    // 4. Boundary Majority Filter Pass (eliminates halo transition fringes / isolated noise pixels)
    if width >= 3 && height >= 3 {
        let copy = pixels.to_vec();
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                let idx = (y * width + x) * 4;
                if copy[idx + 3] < 32 {
                    continue;
                }
                let cur_r = copy[idx];
                let cur_g = copy[idx + 1];
                let cur_b = copy[idx + 2];

                let mut same_count = 0usize;
                let mut neighbor_counts: [(u8, u8, u8, u32); 8] = [(0, 0, 0, 0); 8];
                let mut unique_neighbors = 0;

                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let n_idx =
                            (((y as i32 + dy) as usize) * width + ((x as i32 + dx) as usize)) * 4;
                        if copy[n_idx + 3] < 32 {
                            continue;
                        }
                        let nr = copy[n_idx];
                        let ng = copy[n_idx + 1];
                        let nb = copy[n_idx + 2];

                        if nr == cur_r && ng == cur_g && nb == cur_b {
                            same_count += 1;
                        } else {
                            let mut found = false;
                            for i in 0..unique_neighbors {
                                if neighbor_counts[i].0 == nr
                                    && neighbor_counts[i].1 == ng
                                    && neighbor_counts[i].2 == nb
                                {
                                    neighbor_counts[i].3 += 1;
                                    found = true;
                                    break;
                                }
                            }
                            if !found && unique_neighbors < 8 {
                                neighbor_counts[unique_neighbors] = (nr, ng, nb, 1);
                                unique_neighbors += 1;
                            }
                        }
                    }
                }

                if same_count <= isolation && unique_neighbors > 0 {
                    let mut max_n_count = 0;
                    let mut best_n = (cur_r, cur_g, cur_b);
                    for i in 0..unique_neighbors {
                        if neighbor_counts[i].3 > max_n_count {
                            max_n_count = neighbor_counts[i].3;
                            best_n = (neighbor_counts[i].0, neighbor_counts[i].1, neighbor_counts[i].2);
                        }
                    }
                    if max_n_count >= 3 {
                        pixels[idx] = best_n.0;
                        pixels[idx + 1] = best_n.1;
                        pixels[idx + 2] = best_n.2;
                    }
                }
            }
        }
    }

    centers
}

/// Snap anti-aliased edge transition pixels to prevent fuzzy halos along high-contrast outlines
#[no_mangle]
pub extern "C" fn wasm_clean_edges(ptr: *mut u8, width: u32, height: u32, threshold: u32) {
    if ptr.is_null() || width < 3 || height < 3 {
        return;
    }

    let w = width as usize;
    let h = height as usize;
    let pixels = unsafe { std::slice::from_raw_parts_mut(ptr, w * h * 4) };
    let copy = pixels.to_vec();
    let th = threshold as i64;

    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let idx = (y * w + x) * 4;
            let a = copy[idx + 3];
            if a < 32 {
                continue;
            }

            let r = copy[idx] as i64;
            let g = copy[idx + 1] as i64;
            let b = copy[idx + 2] as i64;
            let lum = (r * 299 + g * 587 + b * 114) / 1000;

            let mut dark_neighbors = 0;
            let mut light_neighbors = 0;
            let mut closest_dark = (0u8, 0u8, 0u8);
            let mut closest_light = (255u8, 255u8, 255u8);

            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let n_idx = (((y as i32 + dy) as usize) * w + ((x as i32 + dx) as usize)) * 4;
                    if copy[n_idx + 3] < 32 {
                        continue;
                    }
                    let nr = copy[n_idx];
                    let ng = copy[n_idx + 1];
                    let nb = copy[n_idx + 2];
                    let n_lum = (nr as i64 * 299 + ng as i64 * 587 + nb as i64 * 114) / 1000;

                    if n_lum < 60 {
                        dark_neighbors += 1;
                        closest_dark = (nr, ng, nb);
                    } else if n_lum > 160 {
                        light_neighbors += 1;
                        closest_light = (nr, ng, nb);
                    }
                }
            }

            if dark_neighbors >= 2 && light_neighbors >= 2 {
                if lum < 128 - th {
                    pixels[idx] = closest_dark.0;
                    pixels[idx + 1] = closest_dark.1;
                    pixels[idx + 2] = closest_dark.2;
                } else if lum > 128 + th {
                    pixels[idx] = closest_light.0;
                    pixels[idx + 1] = closest_light.1;
                    pixels[idx + 2] = closest_light.2;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Circle snapping (eyes, dots, buttons -> perfect circles)
// ---------------------------------------------------------------------------

struct SnappedCircle {
    cx: f32,
    cy: f32,
    r: f32,
    rgb: (u8, u8, u8),
}

/// Finds small, solid, round, single-color blobs in the QUANTIZED image, erases them from the
/// bitmap (filling with the dominant surrounding color so the tracer does not draw them) and
/// returns them as exact circles. They are re-added to the SVG as `<path>` arcs afterwards.
///
/// Conservative on purpose: a blob only qualifies if it
///   - does not touch the image border,
///   - is between 4px and 10% of the image (in original-pixel terms) across,
///   - has an aspect ratio near 1:1,
///   - fills ~78.5% (pi/4) of its bounding box (so it has no holes and is not a square/diamond).
/// Blobs that merge with other same-colored regions (e.g. an eye touching a black outline) or
/// contain a highlight are left to the normal tracer.
fn snap_circles(pixels: &mut [u8], w: usize, h: usize, scale: usize) -> Vec<SnappedCircle> {
    let mut circles: Vec<SnappedCircle> = Vec::new();
    let total = w * h;
    if total == 0 {
        return circles;
    }

    let max_diam = ((w.min(h) as f32) * 0.10) as usize;
    let min_diam = 4 * scale;
    if max_diam <= min_diam {
        return circles;
    }
    let max_store = max_diam * max_diam;

    let mut visited = vec![false; total];
    let mut stack: Vec<usize> = Vec::new();

    for start in 0..total {
        if visited[start] {
            continue;
        }
        let si = start * 4;
        if pixels[si + 3] < 32 {
            visited[start] = true;
            continue;
        }
        let (cr, cg, cb) = (pixels[si], pixels[si + 1], pixels[si + 2]);

        let mut members: Vec<usize> = Vec::new();
        let mut too_big = false;
        let mut area = 0usize;
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (usize::MAX, 0usize, usize::MAX, 0usize);
        let (mut sum_x, mut sum_y) = (0f64, 0f64);

        visited[start] = true;
        stack.push(start);
        while let Some(p) = stack.pop() {
            let x = p % w;
            let y = p / w;
            area += 1;
            sum_x += x as f64;
            sum_y += y as f64;
            if x < min_x {
                min_x = x;
            }
            if x > max_x {
                max_x = x;
            }
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
            if !too_big {
                if members.len() < max_store {
                    members.push(p);
                } else {
                    too_big = true;
                    members.clear();
                }
            }

            let mut nb = [usize::MAX; 4];
            if x > 0 {
                nb[0] = p - 1;
            }
            if x + 1 < w {
                nb[1] = p + 1;
            }
            if y > 0 {
                nb[2] = p - w;
            }
            if y + 1 < h {
                nb[3] = p + w;
            }
            for &q in &nb {
                if q == usize::MAX || visited[q] {
                    continue;
                }
                let qi = q * 4;
                if pixels[qi + 3] >= 32
                    && pixels[qi] == cr
                    && pixels[qi + 1] == cg
                    && pixels[qi + 2] == cb
                {
                    visited[q] = true;
                    stack.push(q);
                }
            }
        }

        if too_big || circles.len() >= MAX_SNAPPED_CIRCLES {
            continue;
        }
        if min_x == 0 || min_y == 0 || max_x + 1 == w || max_y + 1 == h {
            continue;
        }

        let bw = max_x - min_x + 1;
        let bh = max_y - min_y + 1;
        if bw < min_diam || bh < min_diam || bw > max_diam || bh > max_diam {
            continue;
        }
        let aspect = bw as f32 / bh as f32;
        if !(0.85..=1.18).contains(&aspect) {
            continue;
        }
        let fill = area as f32 / (bw * bh) as f32;
        if !(0.65..=0.85).contains(&fill) {
            continue;
        }

        // Pick the most common neighboring color to paint over the blob.
        let mut counts: Vec<((u8, u8, u8), u32)> = Vec::new();
        for &p in &members {
            let x = p % w;
            let y = p / w;
            let mut nb = [usize::MAX; 4];
            if x > 0 {
                nb[0] = p - 1;
            }
            if x + 1 < w {
                nb[1] = p + 1;
            }
            if y > 0 {
                nb[2] = p - w;
            }
            if y + 1 < h {
                nb[3] = p + w;
            }
            for &q in &nb {
                if q == usize::MAX {
                    continue;
                }
                let qi = q * 4;
                if pixels[qi + 3] < 32 {
                    continue;
                }
                let col = (pixels[qi], pixels[qi + 1], pixels[qi + 2]);
                if col == (cr, cg, cb) {
                    continue; // inside the blob
                }
                if let Some(e) = counts.iter_mut().find(|e| e.0 == col) {
                    e.1 += 1;
                } else {
                    counts.push((col, 1));
                }
            }
        }
        let fill_col = counts.iter().max_by_key(|e| e.1).map(|e| e.0);

        for &p in &members {
            let pi = p * 4;
            match fill_col {
                Some((r, g, b)) => {
                    pixels[pi] = r;
                    pixels[pi + 1] = g;
                    pixels[pi + 2] = b;
                }
                None => pixels[pi + 3] = 0,
            }
        }

        circles.push(SnappedCircle {
            cx: (sum_x / area as f64) as f32 + 0.5,
            cy: (sum_y / area as f64) as f32 + 0.5,
            r: ((area as f32) / std::f32::consts::PI).sqrt(),
            rgb: (cr, cg, cb),
        });
    }

    circles
}

/// Detect whether vtracer prints hex colors upper- or lower-case, so snapped circles use the
/// same spelling (keeps the palette / color toggles in the UI consistent).
fn svg_uses_uppercase_hex(svg: &str) -> bool {
    for (pos, _) in svg.match_indices("fill=\"#") {
        for c in svg[pos + 7..].chars().take(6) {
            if c.is_ascii_uppercase() {
                return true;
            }
            if c.is_ascii_lowercase() {
                return false;
            }
        }
    }
    false
}

/// Append circles as `<path>` arcs (not `<circle>`), so all existing JS code that filters / recolors
/// `<path fill="...">` elements keeps working unchanged.
fn append_circles(svg: String, circles: &[SnappedCircle]) -> String {
    if circles.is_empty() {
        return svg;
    }
    let upper = svg_uses_uppercase_hex(&svg);
    let mut markup = String::new();
    for c in circles {
        let hex = if upper {
            format!("#{:02X}{:02X}{:02X}", c.rgb.0, c.rgb.1, c.rgb.2)
        } else {
            format!("#{:02x}{:02x}{:02x}", c.rgb.0, c.rgb.1, c.rgb.2)
        };
        let d = c.r * 2.0;
        markup.push_str(&format!(
            "<path d=\"M{:.2},{:.2} a{:.2},{:.2} 0 1,0 {:.2},0 a{:.2},{:.2} 0 1,0 -{:.2},0 Z\" fill=\"{}\"/>\n",
            c.cx - c.r,
            c.cy,
            c.r,
            c.r,
            d,
            c.r,
            c.r,
            d,
            hex
        ));
    }

    match svg.rfind("</svg>") {
        Some(pos) => {
            let mut out = String::with_capacity(svg.len() + markup.len() + 1);
            out.push_str(&svg[..pos]);
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&markup);
            out.push_str(&svg[pos..]);
            out
        }
        None => svg,
    }
}

// ---------------------------------------------------------------------------
// Border cleanup helper
// ---------------------------------------------------------------------------

#[inline]
fn snap_px_to_bg(pixels: &mut [u8], idx: usize, bg: (u8, u8, u8), tol_sq: u64) {
    if pixels[idx + 3] > 32 {
        let d = color_dist_sq(pixels[idx], pixels[idx + 1], pixels[idx + 2], bg.0, bg.1, bg.2);
        if d <= tol_sq {
            pixels[idx] = bg.0;
            pixels[idx + 1] = bg.1;
            pixels[idx + 2] = bg.2;
        }
    }
}

// ---------------------------------------------------------------------------
// Master pipeline
// ---------------------------------------------------------------------------

/// Master Vectorization Pipeline in Native Rust WebAssembly
///
/// What changed compared to the old version (all to get smooth lines and round circles):
///  1. The bitmap is upscaled (Catmull-Rom) BEFORE quantization/tracing. Anti-aliased edge pixels,
///     once snapped to the nearest palette color at high resolution, give sub-pixel accurate edges,
///     so vtracer fits real curves instead of pixel staircases.
///  2. Destructive preprocessing (clean_edges + 1-neighbor majority filter) is skipped / softened
///     when upscaling, so tiny shapes like eyes are not eaten or made lumpy.
///  3. Pixel-based tracer thresholds are scaled with the upscale factor.
///  4. Small solid round blobs are replaced by mathematically perfect circles.
///  5. The SVG viewBox is in upscaled coordinates; it still scales to any size, and stats still
///     report the ORIGINAL image width/height.
#[no_mangle]
pub extern "C" fn wasm_vectorize(
    ptr: *mut u8,
    width: u32,
    height: u32,
    num_colors: u32,
    blur_radius: u32,
    remove_bg: u32,
    bg_r: u32,
    bg_g: u32,
    bg_b: u32,
    bg_tol: u32,
    min_path_len: u32,
    stroke_mode: u32,
    stroke_width: f32,
    detail_level: u32,     // 0 = Low, 1 = Medium, 2 = High
    corner_threshold: u32, // 0 = auto (from detail_level), or explicit degrees e.g. 30..90
    curve_smoothness: f32, // 0.0 = auto, or explicit e.g. 0.1..5.0
    splice_threshold: u32, // 0 = auto, or explicit e.g. 20..70
    out_len: *mut usize,
) -> *mut u8 {
    if ptr.is_null() || out_len.is_null() || width == 0 || height == 0 {
        return std::ptr::null_mut();
    }

    // ---- 0. Upscale for smooth curve fitting -------------------------------------------------
    let src_w = width as usize;
    let src_h = height as usize;
    let long_side = src_w.max(src_h);
    let scale = (TARGET_LONG_SIDE / long_side).clamp(1, MAX_UPSCALE);
    let sf = scale as f64;

    let mut upscaled: Vec<u8> = if scale > 1 {
        let src = unsafe { std::slice::from_raw_parts(ptr as *const u8, src_w * src_h * 4) };
        upscale_catmull_rom(src, src_w, src_h, scale)
    } else {
        Vec::new()
    };
    let work_ptr: *mut u8 = if scale > 1 { upscaled.as_mut_ptr() } else { ptr };

    // Working (possibly upscaled) dimensions
    let w = src_w * scale;
    let h = src_h * scale;
    let w32 = w as u32;
    let h32 = h as u32;
    let total_pixels = w * h;

    // ---- 1. Detect background color ---------------------------------------------------------
    let mut bg_buf = [255u8; 4];
    wasm_detect_background(work_ptr, w32, h32, bg_buf.as_mut_ptr());
    let detected_bg = (bg_buf[0], bg_buf[1], bg_buf[2]);

    // ---- 2. Background keyout if requested --------------------------------------------------
    if remove_bg == 1 {
        let (tr, tg, tb) = if bg_r <= 255 && bg_g <= 255 && bg_b <= 255 {
            (bg_r as u8, bg_g as u8, bg_b as u8)
        } else {
            detected_bg
        };
        wasm_remove_background(work_ptr, total_pixels * 4, tr, tg, tb, bg_tol);
    }

    // ---- 3. Pre-clean anti-aliased edge fringes (native resolution only) ---------------------
    // At upscaled resolution the quantizer snaps edge pixels cleanly by itself, and this filter
    // is what used to make tiny shapes lumpy.
    if scale == 1 {
        wasm_clean_edges(work_ptr, w32, h32, 15);
    }

    // ---- 4. Border cleanup + quantization + circle snapping ----------------------------------
    let k = if num_colors >= 2 && num_colors <= 64 { num_colors as usize } else { 12 };

    let circles: Vec<SnappedCircle> = {
        let pixels = unsafe { std::slice::from_raw_parts_mut(work_ptr, total_pixels * 4) };

        // Clean outer perimeter rows/columns if close to detected background
        let border = 2 * scale;
        let bg_tol_sq = (bg_tol * bg_tol * 2) as u64;
        for y in 0..h {
            if y < border || y + border >= h {
                for x in 0..w {
                    snap_px_to_bg(pixels, (y * w + x) * 4, detected_bg, bg_tol_sq);
                }
            } else {
                for x in 0..border.min(w) {
                    snap_px_to_bg(pixels, (y * w + x) * 4, detected_bg, bg_tol_sq);
                }
                for x in w.saturating_sub(border)..w {
                    snap_px_to_bg(pixels, (y * w + x) * 4, detected_bg, bg_tol_sq);
                }
            }
        }

        // Guard: if there are no opaque pixels, return an empty SVG instead of panicking in vtracer
        let opaque_count = (0..total_pixels).filter(|&i| pixels[i * 4 + 3] >= 32).count();
        if opaque_count == 0 {
            let empty_svg = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="100%" height="100%"></svg>"#,
                w, h
            );
            let wireframe_svg = empty_svg.clone();
            let json = format!(
                "{{\"svg\":{},\"wireframe\":{},\"palette\":[],\"detectedBg\":{{\"r\":{},\"g\":{},\"b\":{}}},\"stats\":{{\"width\":{},\"height\":{},\"pathCount\":0,\"nodeCount\":0,\"colorCount\":0,\"svgSize\":{}}}}}",
                serde_json_escape(&empty_svg),
                serde_json_escape(&wireframe_svg),
                detected_bg.0, detected_bg.1, detected_bg.2,
                width, height, empty_svg.len()
            );
            let bytes = json.into_bytes();
            let len = bytes.len();
            unsafe { *out_len = len; }
            let out_ptr = alloc(len);
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_ptr, len); }
            return out_ptr;
        }

        // Quantize to K dominant perceptual colors (eliminates muddy intermediate layers)
        let isolation = if scale > 1 { 0 } else { 1 };
        quantize_palette_perceptual(pixels, w, h, k, isolation);

        // Replace small solid round blobs by perfect circles
        if SNAP_CIRCLES {
            snap_circles(pixels, w, h, scale)
        } else {
            Vec::new()
        }
    };

    // ---- 5. Optional pre-filter blur (radius is in ORIGINAL pixels) --------------------------
    if blur_radius > 0 {
        wasm_box_blur(work_ptr, w32, h32, blur_radius * scale as u32);
    }

    // ---- 6. Build VTracer Config (pixel-based values scale with the upscale factor) ----------
    let mut cfg = Config::from_preset(Preset::Poster);
    cfg.hierarchical = Hierarchical::Stacked;
    cfg.mode = FitMode::Spline;
    cfg.path_precision = Some(2);
    cfg.color_precision = 8;
    cfg.max_colors = Some(k);

    match detail_level {
        2 => {
            // High Detail: Vector Magic "Anti-aliased artwork, High"
            cfg.filter_speckle = std::cmp::max(2, min_path_len as usize) * scale;
            cfg.layer_difference = 12;
            cfg.corner_threshold = if corner_threshold > 0 { corner_threshold as i32 } else { 60 };
            cfg.length_threshold = (3.0 * sf) as _;
            cfg.max_iterations = 12;
            cfg.splice_threshold = if splice_threshold > 0 { splice_threshold as i32 } else { 45 };
            cfg.simplify = if curve_smoothness > 0.0 { Some(curve_smoothness as f64) } else { Some(0.4) };
        }
        0 => {
            // Low Detail: Maximum simplification, large clean shapes
            cfg.filter_speckle = std::cmp::max(8, min_path_len as usize) * scale;
            cfg.layer_difference = 24;
            cfg.corner_threshold = if corner_threshold > 0 { corner_threshold as i32 } else { 80 };
            cfg.length_threshold = (6.0 * sf) as _;
            cfg.max_iterations = 8;
            cfg.splice_threshold = if splice_threshold > 0 { splice_threshold as i32 } else { 55 };
            cfg.simplify = if curve_smoothness > 0.0 { Some(curve_smoothness as f64) } else { Some(1.8) };
        }
        _ => {
            // Medium Detail (Default): Balanced smooth curves & detail
            cfg.filter_speckle = std::cmp::max(4, min_path_len as usize) * scale;
            cfg.layer_difference = 16;
            cfg.corner_threshold = if corner_threshold > 0 { corner_threshold as i32 } else { 70 };
            cfg.length_threshold = (4.5 * sf) as _;
            cfg.max_iterations = 10;
            cfg.splice_threshold = if splice_threshold > 0 { splice_threshold as i32 } else { 45 };
            cfg.simplify = if curve_smoothness > 0.0 { Some(curve_smoothness as f64) } else { Some(0.8) };
        }
    }

    if num_colors > 1 && num_colors <= 64 {
        cfg.max_colors = Some(num_colors as usize);
    }

    let img = {
        let pixels = unsafe { std::slice::from_raw_parts(work_ptr as *const u8, total_pixels * 4) };
        ColorImage {
            pixels: pixels.to_vec(),
            width: w,
            height: h,
        }
    };

    let pipeline_res = cfg.build();
    let mut svg_string = match pipeline_res {
        Ok(pipeline) => match pipeline.to_svg(&img) {
            Ok(s) => s,
            Err(e) => format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}"><text y="20">Error: {:?}</text></svg>"##,
                w, h, e
            ),
        },
        Err(e) => format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}"><text y="20">Config Error: {:?}</text></svg>"##,
            w, h, e
        ),
    };

    // Normalize SVG opening tag: ensure viewBox exists and remove duplicate width/height attributes
    if let Some(svg_start) = svg_string.find("<svg") {
        if let Some(rel_end) = svg_string[svg_start..].find('>') {
            let tag_end = svg_start + rel_end;
            let tag = &svg_string[svg_start..=tag_end];

            // Strip all existing width="..." and height="..." attributes from the tag
            let mut cleaned = String::new();
            let mut i = 0usize;
            while i < tag.len() {
                // find next width or height attr
                let rest = &tag[i..];
                let wpos = rest.find("width=\"");
                let hpos = rest.find("height=\"");
                let next = match (wpos, hpos) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                };
                if let Some(off) = next {
                    let abs = i + off;
                    // find the attribute name start (skip any leading space)
                    let mut attr_start = abs;
                    while attr_start > i && tag.as_bytes()[attr_start - 1] == b' ' {
                        attr_start -= 1;
                    }
                    // copy up to attr_start
                    cleaned.push_str(&tag[i..attr_start]);
                    // find closing quote
                    let after = &tag[abs..];
                    if let Some(qrel) = after.find('"') {
                        // find second quote
                        if let Some(q2) = after[qrel + 1..].find('"') {
                            let close = abs + qrel + 1 + q2 + 1;
                            i = close;
                            // skip trailing spaces
                            while i < tag.len() && tag.as_bytes()[i] == b' ' {
                                i += 1;
                            }
                            continue;
                        }
                    }
                    // fallback: copy rest
                    cleaned.push_str(&tag[i..]);
                    i = tag.len();
                } else {
                    cleaned.push_str(&tag[i..]);
                    break;
                }
            }

            // Ensure viewBox exists (in WORKING = upscaled coordinates, same aspect ratio as the source)
            if !cleaned.contains("viewBox=") {
                // insert before closing '>'
                if let Some(gt) = cleaned.rfind('>') {
                    let vb = format!(r#" viewBox="0 0 {} {}""#, w, h);
                    cleaned.insert_str(gt, &vb);
                }
            }

            // Add responsive width/height before final '>'
            if let Some(gt) = cleaned.rfind('>') {
                cleaned.insert_str(gt, r#" width="100%" height="100%""#);
            }

            let new_svg = format!("{}{}{}", &svg_string[..svg_start], cleaned, &svg_string[tag_end + 1..]);
            svg_string = new_svg;
        }
    }

    // Add perfect circles (eyes, dots...) on top of the traced layers
    svg_string = append_circles(svg_string, &circles);

    // Stroke Modes:
    // 0 = Vector Magic Standard (Solid Fill + configurable Seam-Sealing Stroke):
    //     Every path keeps its solid fill AND gets a matching stroke with round linejoin.
    //     This eliminates hairline gaps between adjacent vector shapes.
    //     Setting stroke_width to 0 removes the stroke (pure fill).
    // 1 = Pure Solid Fill (fill only, no stroke attributes)
    // 2 = Single Stroke Outlines (fill="none" stroke="..." stroke-width="sw")
    if stroke_mode == 0 && stroke_width > 0.0 {
        let seam = stroke_width * scale as f32;
        let mut enhanced = String::with_capacity(svg_string.len() + svg_string.len() / 6);
        for line in svg_string.lines() {
            if line.contains("<path") && line.contains("fill=\"") {
                if let Some(fill_start) = line.find("fill=\"") {
                    let after_fill = &line[fill_start + 6..];
                    if let Some(quote_end) = after_fill.find('"') {
                        let fill_val = &after_fill[..quote_end];
                        if fill_val != "none" {
                            let stroke_attr = format!(
                                r#" stroke="{}" stroke-width="{:.2}" stroke-linejoin="round""#,
                                fill_val, seam
                            );
                            let mod_line = line.replacen(
                                &format!(r#"fill="{}""#, fill_val),
                                &format!(r#"fill="{}"{}"#, fill_val, stroke_attr),
                                1
                            );
                            enhanced.push_str(&mod_line);
                            enhanced.push('\n');
                            continue;
                        }
                    }
                }
            }
            enhanced.push_str(line);
            enhanced.push('\n');
        }
        svg_string = enhanced;
    } else if stroke_mode == 2 {
        let sw = (if stroke_width > 0.0 { stroke_width } else { 1.5 }) * scale as f32;
        svg_string = svg_string.replace("fill=\"", "fill=\"none\" stroke=\"");
        svg_string = svg_string.replace(
            "<path ",
            &format!("<path stroke-width=\"{:.2}\" stroke-linecap=\"round\" stroke-linejoin=\"round\" ", sw),
        );
    }

    // 7. Extract Palette & Statistics from SVG
    let mut palette_map: HashMap<String, (u8, u8, u8, usize, usize)> = HashMap::new();
    let mut total_paths = 0usize;
    let mut total_nodes = 0usize;

    for line in svg_string.lines() {
        if line.contains("<path") {
            total_paths += 1;

            // Count Bézier nodes / segments
            let node_markers = line.matches('C').count()
                + line.matches('c').count()
                + line.matches('Q').count()
                + line.matches('q').count()
                + line.matches('L').count()
                + line.matches('l').count();
            let nodes = if node_markers > 0 { node_markers } else { 1 };
            total_nodes += nodes;

            // Extract hex or rgb color
            if let Some(fill_start) = line.find(r#"fill=""#) {
                let after_fill = &line[fill_start + 6..];
                if let Some(quote_end) = after_fill.find('"') {
                    let col_str = &after_fill[..quote_end];
                    if col_str != "none" && !col_str.is_empty() {
                        let hex = col_str.to_string();

                        let (r, g, b) = parse_hex_or_rgb(&hex);
                        let entry = palette_map.entry(hex).or_insert((r, g, b, 0, 0));
                        entry.3 += 1;
                        entry.4 += nodes;
                    }
                }
            }
        }
    }

    // Format Palette JSON
    let mut pal_json = String::from("[");
    let mut first = true;
    for (hex, (r, g, b, p_count, n_count)) in &palette_map {
        if !first {
            pal_json.push(',');
        }
        first = false;
        pal_json.push_str(&format!(
            r#"{{"hex":"{}","r":{},"g":{},"b":{},"pathCount":{},"nodeCount":{}}}"#,
            hex, r, g, b, p_count, n_count
        ));
    }
    pal_json.push(']');

    // Build Wireframe SVG
    let wire_sw = 0.8 * scale as f32;
    let wireframe_inner = svg_string.replace(
        "fill=\"",
        &format!("fill=\"none\" stroke=\"#3b82f6\" stroke-width=\"{:.1}\" data-fill=\"", wire_sw),
    );
    let wireframe_svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"100%\" height=\"100%\" style=\"shape-rendering: geometricPrecision;\">\n<rect width=\"{}\" height=\"{}\" fill=\"#0f172a\" fill-opacity=\"0.6\" />\n<g class=\"wireframe-paths\">{}</g>\n</svg>",
        w, h,
        w, h,
        wireframe_inner
    );

    // Build Final JSON Package (stats report the ORIGINAL image size)
    let json = format!(
        "{{\"svg\":{},\"wireframe\":{},\"palette\":{},\"detectedBg\":{{\"r\":{},\"g\":{},\"b\":{}}},\"stats\":{{\"width\":{},\"height\":{},\"pathCount\":{},\"nodeCount\":{},\"colorCount\":{},\"svgSize\":{}}}}}",
        serde_json_escape(&svg_string),
        serde_json_escape(&wireframe_svg),
        pal_json,
        detected_bg.0,
        detected_bg.1,
        detected_bg.2,
        width,
        height,
        total_paths,
        total_nodes,
        palette_map.len(),
        svg_string.len()
    );

    let bytes = json.into_bytes();
    let len = bytes.len();
    unsafe {
        *out_len = len;
    }

    let out_ptr = alloc(len);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_ptr, len);
    }

    out_ptr
}

fn parse_hex_or_rgb(col: &str) -> (u8, u8, u8) {
    if col.starts_with('#') && col.len() >= 7 {
        let r = u8::from_str_radix(&col[1..3], 16).unwrap_or(0);
        let g = u8::from_str_radix(&col[3..5], 16).unwrap_or(0);
        let b = u8::from_str_radix(&col[5..7], 16).unwrap_or(0);
        (r, g, b)
    } else {
        (128, 128, 128)
    }
}

fn serde_json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_allocation() {
        let ptr = alloc(128);
        assert!(!ptr.is_null());
        dealloc(ptr, 128);
    }

    #[test]
    fn test_parse_hex() {
        assert_eq!(parse_hex_or_rgb("#ff0000"), (255, 0, 0));
        assert_eq!(parse_hex_or_rgb("#00ff00"), (0, 255, 0));
        assert_eq!(parse_hex_or_rgb("#0000ff"), (0, 0, 255));
    }

    #[test]
    fn test_serde_escape() {
        let escaped = serde_json_escape("hello \"world\"\nnext");
        assert_eq!(escaped, "\"hello \\\"world\\\"\\nnext\"");
    }

    #[test]
    fn test_detect_background() {
        // 10x10 white image
        let mut pixels = vec![255u8; 10 * 10 * 4];
        let mut bg_out = [0u8; 4];
        wasm_detect_background(pixels.as_mut_ptr(), 10, 10, bg_out.as_mut_ptr());
        assert_eq!(bg_out[0], 255);
        assert_eq!(bg_out[1], 255);
        assert_eq!(bg_out[2], 255);
    }

    #[test]
    fn test_remove_background() {
        // 2x2 image: 3 white pixels, 1 red pixel
        let mut pixels = vec![
            255, 255, 255, 255, // white
            255, 255, 255, 255, // white
            255, 255, 255, 255, // white
            255, 0, 0, 255,     // red
        ];
        let removed = wasm_remove_background(pixels.as_mut_ptr(), pixels.len(), 255, 255, 255, 10);
        assert_eq!(removed, 3);
        assert_eq!(pixels[3], 0);  // first white pixel alpha is 0
        assert_eq!(pixels[15], 255); // red pixel alpha remains 255
    }

    #[test]
    fn test_upscale_preserves_flat_color() {
        let src = vec![10u8, 20, 30, 255].repeat(4 * 4);
        let out = upscale_catmull_rom(&src, 4, 4, 3);
        assert_eq!(out.len(), 12 * 12 * 4);
        for px in out.chunks(4) {
            assert_eq!(px, &[10, 20, 30, 255]);
        }
    }

    #[test]
    fn test_wasm_vectorize_high_detail() {
        let width = 50u32;
        let height = 50u32;
        let mut pixels = vec![255u8; (width * height * 4) as usize];

        // Draw dark square inside
        for y in 10..40 {
            for x in 10..40 {
                let idx = ((y * width + x) * 4) as usize;
                pixels[idx] = 30;
                pixels[idx + 1] = 30;
                pixels[idx + 2] = 30;
            }
        }

        let mut out_len = 0usize;
        let res_ptr = wasm_vectorize(
            pixels.as_mut_ptr(),
            width,
            height,
            4,
            0,
            0,
            999, 999, 999,
            25,
            2,
            0,
            1.5,
            2, // High detail
            50,
            0.5,
            40,
            &mut out_len,
        );

        assert!(!res_ptr.is_null());
        assert!(out_len > 0);

        let json_str = unsafe {
            let slice = std::slice::from_raw_parts(res_ptr, out_len);
            std::str::from_utf8(slice).unwrap().to_string()
        };

        assert!(json_str.contains("\"svg\":"));
        assert!(json_str.contains("<path"));
        // stats keep reporting the ORIGINAL size even though tracing ran upscaled
        assert!(json_str.contains("\"width\":50,\"height\":50"));
        wasm_free_string(res_ptr, out_len);
    }

    #[test]
    fn test_small_disk_becomes_perfect_circle() {
        let width = 160u32;
        let height = 160u32;
        let mut pixels = vec![255u8; (width * height * 4) as usize];

        // Small black "eye": radius 5px at the center of a white 160x160 image
        let (cx, cy, r) = (80i32, 80i32, 5i32);
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                if (x - cx) * (x - cx) + (y - cy) * (y - cy) <= r * r {
                    let idx = ((y as u32 * width + x as u32) * 4) as usize;
                    pixels[idx] = 0;
                    pixels[idx + 1] = 0;
                    pixels[idx + 2] = 0;
                }
            }
        }

        let mut out_len = 0usize;
        let res_ptr = wasm_vectorize(
            pixels.as_mut_ptr(),
            width,
            height,
            4,
            0,
            0,
            999, 999, 999,
            25,
            2,
            1, // pure fill
            1.5,
            2,
            50,
            0.5,
            40,
            &mut out_len,
        );
        assert!(!res_ptr.is_null());

        let json_str = unsafe {
            let slice = std::slice::from_raw_parts(res_ptr, out_len);
            std::str::from_utf8(slice).unwrap().to_string()
        };
        // Perfect circles are emitted as two SVG arcs
        assert!(json_str.contains("0 1,0"), "expected snapped circle arcs in SVG");
        wasm_free_string(res_ptr, out_len);
    }
}