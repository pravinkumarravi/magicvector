use std::alloc::{alloc as sys_alloc, dealloc as sys_dealloc, Layout};

/// Allocate memory buffer inside WebAssembly linear memory
#[no_mangle]
pub extern "C" fn alloc(size: usize) -> *mut u8 {
    let layout = Layout::from_size_align(size, 4).unwrap();
    unsafe { sys_alloc(layout) }
}

/// Deallocate memory buffer inside WebAssembly linear memory
#[no_mangle]
pub extern "C" fn dealloc(ptr: *mut u8, size: usize) {
    let layout = Layout::from_size_align(size, 4).unwrap();
    unsafe { sys_dealloc(ptr, layout) }
}

/// Detect dominant background color from edge and corner samples
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
    let sample_points: [(u32, u32); 8] = [
        (0, 0),
        (width - 1, 0),
        (0, height - 1),
        (width - 1, height - 1),
        (width / 2, 0),
        (0, height / 2),
        (width - 1, height / 2),
        (width / 2, height - 1),
    ];

    let mut sum_r: u64 = 0;
    let mut sum_g: u64 = 0;
    let mut sum_b: u64 = 0;
    let mut count: u64 = 0;

    for (x, y) in sample_points {
        let idx = ((y * width + x) * 4) as usize;
        let a = pixels[idx + 3];
        if a > 10 {
            sum_r += pixels[idx] as u64;
            sum_g += pixels[idx + 1] as u64;
            sum_b += pixels[idx + 2] as u64;
            count += 1;
        }
    }

    let out = unsafe { std::slice::from_raw_parts_mut(bg_out_ptr, 4) };
    if count == 0 {
        out[0] = 255;
        out[1] = 255;
        out[2] = 255;
        out[3] = 255;
    } else {
        out[0] = (sum_r / count) as u8;
        out[1] = (sum_g / count) as u8;
        out[2] = (sum_b / count) as u8;
        out[3] = 255;
    }
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

        let dr = r - tr;
        let dg = g - tg;
        let db = b - tb;

        let dist_sq = dr * dr + dg * dg + db * db;
        if dist_sq <= tol_sq {
            pixels[offset + 3] = 0;
            removed_count += 1;
        }
    }

    removed_count
}

/// Fast separable 2D box blur filter on raw pixel buffer
#[no_mangle]
pub extern "C" fn wasm_box_blur(ptr: *mut u8, width: u32, height: u32, radius: u32) {
    if ptr.is_null() || width == 0 || height == 0 || radius == 0 {
        return;
    }

    let w = width as usize;
    let h = height as usize;
    let r = radius as usize;
    let size = w * h;

    let pixels = unsafe { std::slice::from_raw_parts_mut(ptr, size * 4) };
    let mut temp = vec![0u8; size * 4];

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

/// Heavy-duty K-Means color quantization in WebAssembly
/// Clusters all image pixels into `k` dominant colors, quantizes the image in-place,
/// and returns the extracted palette in `palette_out_ptr`.
#[no_mangle]
pub extern "C" fn wasm_quantize_kmeans(
    ptr: *mut u8,
    width: u32,
    height: u32,
    k: u32,
    max_iters: u32,
    palette_out_ptr: *mut u8,
) -> u32 {
    if ptr.is_null() || palette_out_ptr.is_null() || width == 0 || height == 0 || k == 0 {
        return 0;
    }

    let k_val = std::cmp::min(k as usize, 64);
    let total_pixels = (width * height) as usize;
    let pixels = unsafe { std::slice::from_raw_parts_mut(ptr, total_pixels * 4) };

    // Initial centroids sampling evenly across pixels
    let mut centroids: Vec<[f32; 3]> = Vec::with_capacity(k_val);
    let step = total_pixels / (k_val + 1);

    for i in 0..k_val {
        let p_idx = std::cmp::min((i + 1) * step, total_pixels - 1) * 4;
        centroids.push([
            pixels[p_idx] as f32,
            pixels[p_idx + 1] as f32,
            pixels[p_idx + 2] as f32,
        ]);
    }

    let iters = std::cmp::min(max_iters as usize, 12);
    let mut assignments = vec![0u8; total_pixels];

    for _ in 0..iters {
        // Assignment phase
        for i in 0..total_pixels {
            let offset = i * 4;
            let r = pixels[offset] as f32;
            let g = pixels[offset + 1] as f32;
            let b = pixels[offset + 2] as f32;

            let mut min_dist = f32::MAX;
            let mut best_k = 0;

            for (c_idx, c) in centroids.iter().enumerate() {
                let dr = r - c[0];
                let dg = g - c[1];
                let db = b - c[2];
                let d = dr * dr + dg * dg + db * db;
                if d < min_dist {
                    min_dist = d;
                    best_k = c_idx;
                }
            }
            assignments[i] = best_k as u8;
        }

        // Update phase
        let mut sums = vec![[0.0f32; 3]; k_val];
        let mut counts = vec![0u32; k_val];

        for i in 0..total_pixels {
            let c_idx = assignments[i] as usize;
            let offset = i * 4;
            sums[c_idx][0] += pixels[offset] as f32;
            sums[c_idx][1] += pixels[offset + 1] as f32;
            sums[c_idx][2] += pixels[offset + 2] as f32;
            counts[c_idx] += 1;
        }

        for (c_idx, c) in centroids.iter_mut().enumerate() {
            if counts[c_idx] > 0 {
                c[0] = sums[c_idx][0] / counts[c_idx] as f32;
                c[1] = sums[c_idx][1] / counts[c_idx] as f32;
                c[2] = sums[c_idx][2] / counts[c_idx] as f32;
            }
        }
    }

    // In-place quantization write-back
    for i in 0..total_pixels {
        let c_idx = assignments[i] as usize;
        let offset = i * 4;
        let c = &centroids[c_idx];
        pixels[offset] = c[0].round() as u8;
        pixels[offset + 1] = c[1].round() as u8;
        pixels[offset + 2] = c[2].round() as u8;
    }

    // Write palette output
    let palette_out = unsafe { std::slice::from_raw_parts_mut(palette_out_ptr, k_val * 4) };
    for i in 0..k_val {
        let offset = i * 4;
        let c = &centroids[i];
        palette_out[offset] = c[0].round() as u8;
        palette_out[offset + 1] = c[1].round() as u8;
        palette_out[offset + 2] = c[2].round() as u8;
        palette_out[offset + 3] = 255;
    }

    k_val as u32
}

/// Despeckle filter to remove isolated pixel islands (turdsize noise suppression)
#[no_mangle]
pub extern "C" fn wasm_despeckle(ptr: *mut u8, width: u32, height: u32, _threshold: u32) {
    if ptr.is_null() || width < 3 || height < 3 {
        return;
    }

    let w = width as usize;
    let h = height as usize;
    let size = w * h;
    let pixels = unsafe { std::slice::from_raw_parts_mut(ptr, size * 4) };

    // Pass over internal pixels: if an isolated pixel is surrounded by identical neighbors,
    // snap it to the dominant surrounding color
    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            let idx = (y * w + x) * 4;
            let left_idx = (y * w + (x - 1)) * 4;
            let right_idx = (y * w + (x + 1)) * 4;
            let top_idx = ((y - 1) * w + x) * 4;
            let bot_idx = ((y + 1) * w + x) * 4;

            // If left, right, top, and bottom are identical to each other but different from center:
            if pixels[left_idx] == pixels[right_idx]
                && pixels[left_idx] == pixels[top_idx]
                && pixels[left_idx] == pixels[bot_idx]
                && pixels[left_idx + 1] == pixels[right_idx + 1]
                && pixels[left_idx + 1] == pixels[top_idx + 1]
                && pixels[left_idx + 1] == pixels[bot_idx + 1]
                && (pixels[idx] != pixels[left_idx] || pixels[idx + 1] != pixels[left_idx + 1])
            {
                pixels[idx] = pixels[left_idx];
                pixels[idx + 1] = pixels[left_idx + 1];
                pixels[idx + 2] = pixels[left_idx + 2];
            }
        }
    }
}
