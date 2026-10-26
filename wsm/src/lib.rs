use std::alloc::{alloc as sys_alloc, dealloc as sys_dealloc, Layout};
use std::collections::HashMap;

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

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ColorRgb {
    r: u8,
    g: u8,
    b: u8,
}

impl ColorRgb {
    fn dist_sq(&self, other: &ColorRgb) -> f32 {
        let dr = self.r as f32 - other.r as f32;
        let dg = self.g as f32 - other.g as f32;
        let db = self.b as f32 - other.b as f32;
        dr * dr + dg * dg + db * db
    }

    fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
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

        let dist_sq = (r - tr) * (r - tr) + (g - tg) * (g - tg) + (b - tb) * (b - tb);
        if dist_sq <= tol_sq {
            pixels[offset + 3] = 0;
            removed_count += 1;
        }
    }

    removed_count
}

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
            let mut count = 0u32;

            let y_start = if y > r { y - r } else { 0 };
            let y_end = std::cmp::min(y + r, h - 1);

            for iy in y_start..=y_end {
                let idx = (iy * w + x) * 4;
                sum_r += temp[idx] as u32;
                sum_g += temp[idx + 1] as u32;
                sum_b += temp[idx + 2] as u32;
                count += 1;
            }

            let out_idx = (y * w + x) * 4;
            pixels[out_idx] = (sum_r / count) as u8;
            pixels[out_idx + 1] = (sum_g / count) as u8;
            pixels[out_idx + 2] = (sum_b / count) as u8;
        }
    }
}

/// K-Means++ Color Quantization
fn quantize_kmeans_plus_plus(
    pixels: &[u8],
    total_pixels: usize,
    k: usize,
) -> (Vec<ColorRgb>, Vec<u8>) {
    // 1. Gather distinct non-transparent sample colors
    let mut color_counts: HashMap<u32, (ColorRgb, usize)> = HashMap::new();
    let step = std::cmp::max(1, total_pixels / 20000);

    for i in (0..total_pixels).step_by(step) {
        let idx = i * 4;
        let a = pixels[idx + 3];
        if a < 10 {
            continue;
        }

        let r = pixels[idx];
        let g = pixels[idx + 1];
        let b = pixels[idx + 2];

        // Quantize to 5-bit key to group almost identical pixels
        let key = ((r as u32 >> 3) << 10) | ((g as u32 >> 3) << 5) | (b as u32 >> 3);
        let entry = color_counts.entry(key).or_insert((ColorRgb { r, g, b }, 0));
        entry.1 += 1;
    }

    if color_counts.is_empty() {
        return (vec![ColorRgb { r: 255, g: 255, b: 255 }], vec![0; total_pixels]);
    }

    let mut distinct_colors: Vec<(ColorRgb, usize)> = color_counts.into_values().collect();
    distinct_colors.sort_by(|a, b| b.1.cmp(&a.1));

    let actual_k = std::cmp::min(k, distinct_colors.len());

    // 2. K-Means++ Seeding
    let mut centroids: Vec<ColorRgb> = Vec::with_capacity(actual_k);
    centroids.push(distinct_colors[0].0);

    while centroids.len() < actual_k {
        let mut max_dist = -1.0f32;
        let mut best_color = distinct_colors[0].0;

        for (color, weight) in &distinct_colors {
            let mut min_d = f32::MAX;
            for c in &centroids {
                let d = color.dist_sq(c);
                if d < min_d {
                    min_d = d;
                }
            }
            let weighted_d = min_d * (*weight as f32).sqrt();
            if weighted_d > max_dist {
                max_dist = weighted_d;
                best_color = *color;
            }
        }
        centroids.push(best_color);
    }

    // 3. Lloyd Iterations (8 passes)
    let iters = 8;
    for _ in 0..iters {
        let mut sums_r = vec![0.0f32; actual_k];
        let mut sums_g = vec![0.0f32; actual_k];
        let mut sums_b = vec![0.0f32; actual_k];
        let mut counts = vec![0usize; actual_k];

        for (color, weight) in &distinct_colors {
            let mut min_d = f32::MAX;
            let mut best_k = 0;
            for (c_idx, c) in centroids.iter().enumerate() {
                let d = color.dist_sq(c);
                if d < min_d {
                    min_d = d;
                    best_k = c_idx;
                }
            }

            let w = *weight as f32;
            sums_r[best_k] += color.r as f32 * w;
            sums_g[best_k] += color.g as f32 * w;
            sums_b[best_k] += color.b as f32 * w;
            counts[best_k] += *weight;
        }

        for (c_idx, c) in centroids.iter_mut().enumerate() {
            if counts[c_idx] > 0 {
                c.r = (sums_r[c_idx] / counts[c_idx] as f32).round() as u8;
                c.g = (sums_g[c_idx] / counts[c_idx] as f32).round() as u8;
                c.b = (sums_b[c_idx] / counts[c_idx] as f32).round() as u8;
            }
        }
    }

    // 4. Assign all image pixels
    let mut assignments = vec![255u8; total_pixels];
    for i in 0..total_pixels {
        let idx = i * 4;
        let a = pixels[idx + 3];
        if a < 10 {
            continue; // transparent
        }

        let p_col = ColorRgb {
            r: pixels[idx],
            g: pixels[idx + 1],
            b: pixels[idx + 2],
        };

        let mut min_d = f32::MAX;
        let mut best_k = 0;
        for (c_idx, c) in centroids.iter().enumerate() {
            let d = p_col.dist_sq(c);
            if d < min_d {
                min_d = d;
                best_k = c_idx;
            }
        }
        assignments[i] = best_k as u8;
    }

    (centroids, assignments)
}

/// Boundary Segment: directed edge from vertex (x1, y1) to (x2, y2)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct GridPoint {
    x: u32,
    y: u32,
}

/// Extract closed loops from directed boundary edges for a specific color
fn trace_color_loops(
    width: usize,
    height: usize,
    color_idx: u8,
    assignments: &[u8],
    min_loop_len: usize,
) -> Vec<Vec<Point>> {
    let mut adj: HashMap<GridPoint, Vec<GridPoint>> = HashMap::new();

    // Scan pixel boundaries
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            if assignments[idx] != color_idx {
                continue;
            }

            let p00 = GridPoint { x: x as u32, y: y as u32 };
            let p10 = GridPoint { x: (x + 1) as u32, y: y as u32 };
            let p11 = GridPoint { x: (x + 1) as u32, y: (y + 1) as u32 };
            let p01 = GridPoint { x: x as u32, y: (y + 1) as u32 };

            // Top boundary: goes p00 -> p10
            if y == 0 || assignments[(y - 1) * width + x] != color_idx {
                adj.entry(p00).or_default().push(p10);
            }
            // Right boundary: goes p10 -> p11
            if x == width - 1 || assignments[y * width + (x + 1)] != color_idx {
                adj.entry(p10).or_default().push(p11);
            }
            // Bottom boundary: goes p11 -> p01
            if y == height - 1 || assignments[(y + 1) * width + x] != color_idx {
                adj.entry(p11).or_default().push(p01);
            }
            // Left boundary: goes p01 -> p00
            if x == 0 || assignments[y * width + (x - 1)] != color_idx {
                adj.entry(p01).or_default().push(p00);
            }
        }
    }

    // Traverse Eulerian cycles
    let mut loops = Vec::new();

    while let Some(&start_pt) = adj.keys().next() {
        let mut current = start_pt;
        let mut loop_pts = Vec::new();

        while let Some(targets) = adj.get_mut(&current) {
            if let Some(next_pt) = targets.pop() {
                if targets.is_empty() {
                    adj.remove(&current);
                }
                loop_pts.push(Point {
                    x: current.x as f32,
                    y: current.y as f32,
                });
                current = next_pt;
                if current == start_pt {
                    break;
                }
            } else {
                adj.remove(&current);
                break;
            }
        }

        if loop_pts.len() >= min_loop_len {
            loops.push(loop_pts);
        }
    }

    loops
}

/// Simplify polygon by removing collinear intermediate vertices
fn simplify_collinear(points: &[Point]) -> Vec<Point> {
    if points.len() <= 3 {
        return points.to_vec();
    }

    let mut result = Vec::with_capacity(points.len());
    let n = points.len();

    for i in 0..n {
        let prev = points[(i + n - 1) % n];
        let curr = points[i];
        let next = points[(i + 1) % n];

        let d1x = curr.x - prev.x;
        let d1y = curr.y - prev.y;
        let d2x = next.x - curr.x;
        let d2y = next.y - curr.y;

        // Cross product check for collinearity
        let cross = (d1x * d2y - d1y * d2x).abs();
        if cross > 0.001 {
            result.push(curr);
        }
    }

    if result.len() < 3 {
        points.to_vec()
    } else {
        result
    }
}

/// Generate smooth quadratic Bézier SVG path data `<path d="..." />` from vertices
fn smooth_loop_to_svg_path(points: &[Point], wireframe_nodes: &mut Vec<Point>) -> String {
    let simplified = simplify_collinear(points);
    let n = simplified.len();
    if n < 3 {
        return String::new();
    }

    let mut d = String::new();

    // Start at midpoint between last and first vertex
    let p_last = simplified[n - 1];
    let p_first = simplified[0];
    let start_m = Point {
        x: (p_last.x + p_first.x) * 0.5,
        y: (p_last.y + p_first.y) * 0.5,
    };

    d.push_str(&format!("M {:.1} {:.1} ", start_m.x, start_m.y));
    wireframe_nodes.push(start_m);

    for i in 0..n {
        let ctrl = simplified[i];
        let next = simplified[(i + 1) % n];
        let next_m = Point {
            x: (ctrl.x + next.x) * 0.5,
            y: (ctrl.y + next.y) * 0.5,
        };

        d.push_str(&format!(
            "Q {:.1} {:.1} {:.1} {:.1} ",
            ctrl.x, ctrl.y, next_m.x, next_m.y
        ));
        wireframe_nodes.push(next_m);
    }

    d.push('Z');
    d
}

/// Master WebAssembly Vectorization Engine
/// Performs complete background removal, box blur, K-Means++ color quantization,
/// contour tracing, and midpoint quadratic Bézier curve smoothing directly in Rust.
/// Returns a JSON string containing the full SVG, wireframe SVG, palette, and performance stats.
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
    out_len: *mut usize,
) -> *mut u8 {
    if ptr.is_null() || out_len.is_null() || width == 0 || height == 0 {
        return std::ptr::null_mut();
    }

    let w = width as usize;
    let h = height as usize;
    let total_pixels = w * h;
    let pixels = unsafe { std::slice::from_raw_parts_mut(ptr, total_pixels * 4) };

    // 1. Background Keyout
    let mut detected_bg = ColorRgb { r: 255, g: 255, b: 255 };
    if remove_bg == 1 {
        let target_bg = if bg_r <= 255 && bg_g <= 255 && bg_b <= 255 {
            ColorRgb {
                r: bg_r as u8,
                g: bg_g as u8,
                b: bg_b as u8,
            }
        } else {
            let mut bg_buf = [255u8; 4];
            wasm_detect_background(ptr, width, height, bg_buf.as_mut_ptr());
            ColorRgb {
                r: bg_buf[0],
                g: bg_buf[1],
                b: bg_buf[2],
            }
        };
        detected_bg = target_bg;
        wasm_remove_background(ptr, total_pixels * 4, target_bg.r, target_bg.g, target_bg.b, bg_tol);
    }

    // 2. Separable Box Blur
    if blur_radius > 0 {
        wasm_box_blur(ptr, width, height, blur_radius);
    }

    // 3. High-Quality K-Means++ Color Quantization
    let k = std::cmp::max(2, std::cmp::min(num_colors as usize, 64));
    let (palette, assignments) = quantize_kmeans_plus_plus(pixels, total_pixels, k);

    // 4. Contour Extraction & Midpoint Bézier Spline Smoothing
    let min_len = std::cmp::max(4, min_path_len as usize);
    let mut svg_paths_markup = String::new();
    let mut wireframe_paths_markup = String::new();
    let mut wireframe_nodes: Vec<Point> = Vec::new();

    let mut palette_stats: Vec<(ColorRgb, usize, usize)> = Vec::new();
    let mut total_paths = 0usize;
    let mut total_nodes = 0usize;

    for (color_idx, color) in palette.iter().enumerate() {
        let loops = trace_color_loops(w, h, color_idx as u8, &assignments, min_len);
        if loops.is_empty() {
            continue;
        }

        let mut layer_d = String::new();
        let mut layer_paths = 0usize;
        let mut layer_nodes = 0usize;

        for lp in &loops {
            let d = smooth_loop_to_svg_path(lp, &mut wireframe_nodes);
            if !d.is_empty() {
                layer_d.push_str(&d);
                layer_d.push(' ');
                layer_paths += 1;
                layer_nodes += lp.len();
            }
        }

        if !layer_d.is_empty() {
            total_paths += layer_paths;
            total_nodes += layer_nodes;
            palette_stats.push((*color, layer_paths, layer_nodes));

            let hex = color.to_hex();
            svg_paths_markup.push_str(&format!(
                r#"<path fill="{}" fill-rule="evenodd" d="{}" />"#,
                hex, layer_d
            ));
            svg_paths_markup.push('\n');

            wireframe_paths_markup.push_str(&format!(
                r#"<path fill="none" stroke="{}" stroke-width="0.8" stroke-opacity="0.85" d="{}" />"#,
                hex, layer_d
            ));
            wireframe_paths_markup.push('\n');
        }
    }

    // Build Master SVG
    let svg_string = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="100%" height="100%" style="shape-rendering: geometricPrecision;">
<g id="vm-vector-layers">
{}
</g>
</svg>"#,
        width, height, svg_paths_markup
    );

    // Build Wireframe SVG
    let mut wireframe_nodes_markup = String::new();
    for n in wireframe_nodes.iter().take(5000) {
        wireframe_nodes_markup.push_str(&format!(
            r##"<circle cx="{:.1}" cy="{:.1}" r="1.2" fill="#ffffff" stroke="#3b82f6" stroke-width="0.6" />"##,
            n.x, n.y
        ));
    }

    let wireframe_svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="100%" height="100%" style="shape-rendering: geometricPrecision;">
<rect width="{}" height="{}" fill="#0f172a" fill-opacity="0.6" />
<g class="wireframe-paths">{}</g>
<g class="wireframe-nodes">{}</g>
</svg>"##,
        width, height, width, height, wireframe_paths_markup, wireframe_nodes_markup
    );

    // Format Palette JSON
    let mut pal_json = String::from("[");
    for (i, (col, p_count, n_count)) in palette_stats.iter().enumerate() {
        if i > 0 {
            pal_json.push(',');
        }
        pal_json.push_str(&format!(
            r#"{{"hex":"{}","r":{},"g":{},"b":{},"pathCount":{},"nodeCount":{}}}"#,
            col.to_hex(),
            col.r,
            col.g,
            col.b,
            p_count,
            n_count
        ));
    }
    pal_json.push(']');

    // Build Final JSON Package
    let json = format!(
        r#"{{"svg":{},"wireframe":{},"palette":{},"detectedBg":{{"r":{},"g":{},"b":{}}},"stats":{{"width":{},"height":{},"pathCount":{},"nodeCount":{},"colorCount":{},"svgSize":{}}}}}"#,
        serde_json_escape(&svg_string),
        serde_json_escape(&wireframe_svg),
        pal_json,
        detected_bg.r,
        detected_bg.g,
        detected_bg.b,
        width,
        height,
        total_paths,
        total_nodes,
        palette_stats.len(),
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
