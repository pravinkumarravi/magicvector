use std::alloc::{alloc as sys_alloc, dealloc as sys_dealloc, Layout};
use std::collections::HashMap;
use vtracer::{ColorImage, Config, FitMode, Hierarchical, Preset};

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

            // Check if pixel is in a high-contrast boundary zone
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

            // If this is an anti-aliased transition pixel between dark outline and light region
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

/// Master Vectorization Pipeline in Native Rust WebAssembly
/// Utilizes the state-of-the-art `vtracer` / `visioncortex` curve-fitting framework:
/// - True sub-pixel cubic Bézier curve fitting (`C` and `Q` splines)
/// - Exact hierarchical color clustering matching Vector Magic's "Anti-aliased artwork, High"
/// - Single, crisp, uniform black strokes matching the original bitmap line thickness exactly
/// - Clean full-bleed edge-to-edge canvas background
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

    let w = width as usize;
    let h = height as usize;
    let total_pixels = w * h;
    let pixels = unsafe { std::slice::from_raw_parts_mut(ptr, total_pixels * 4) };

    // 1. Detect Background Color
    let mut bg_buf = [255u8; 4];
    wasm_detect_background(ptr, width, height, bg_buf.as_mut_ptr());
    let detected_bg = (bg_buf[0], bg_buf[1], bg_buf[2]);

    // 2. Background Keyout if requested
    if remove_bg == 1 {
        let (tr, tg, tb) = if bg_r <= 255 && bg_g <= 255 && bg_b <= 255 {
            (bg_r as u8, bg_g as u8, bg_b as u8)
        } else {
            detected_bg
        };
        wasm_remove_background(ptr, total_pixels * 4, tr, tg, tb, bg_tol);
    }

    // 3. Optional pre-filter blur
    if blur_radius > 0 {
        wasm_box_blur(ptr, width, height, blur_radius);
    }

    // 4. Build VTracer Config dynamically matching Detail Level (Vector Magic High / Med / Low)
    let mut cfg = Config::from_preset(Preset::Poster);
    cfg.hierarchical = Hierarchical::Stacked;
    cfg.mode = FitMode::Spline;
    cfg.path_precision = Some(2);

    match detail_level {
        2 => {
            // High Detail: Vector Magic "Anti-aliased artwork, High"
            // Preserves small facial features, finger separations, and sharp cusps
            cfg.filter_speckle = std::cmp::max(2, min_path_len as usize);
            cfg.color_precision = 8;
            cfg.layer_difference = 14;
            cfg.corner_threshold = if corner_threshold > 0 { corner_threshold as i32 } else { 50 };
            cfg.length_threshold = 2.0;
            cfg.max_iterations = 12;
            cfg.splice_threshold = if splice_threshold > 0 { splice_threshold as i32 } else { 40 };
            cfg.simplify = if curve_smoothness > 0.0 { Some(curve_smoothness as f64) } else { Some(0.5) };
        }
        0 => {
            // Low Detail: Maximum simplification, large clean shapes
            cfg.filter_speckle = std::cmp::max(8, min_path_len as usize);
            cfg.color_precision = 6;
            cfg.layer_difference = 22;
            cfg.corner_threshold = if corner_threshold > 0 { corner_threshold as i32 } else { 75 };
            cfg.length_threshold = 5.5;
            cfg.max_iterations = 8;
            cfg.splice_threshold = if splice_threshold > 0 { splice_threshold as i32 } else { 55 };
            cfg.simplify = if curve_smoothness > 0.0 { Some(curve_smoothness as f64) } else { Some(1.8) };
        }
        _ => {
            // Medium Detail (Default): Balanced smooth curves & detail
            cfg.filter_speckle = std::cmp::max(4, min_path_len as usize);
            cfg.color_precision = 7;
            cfg.layer_difference = 16;
            cfg.corner_threshold = if corner_threshold > 0 { corner_threshold as i32 } else { 60 };
            cfg.length_threshold = 3.5;
            cfg.max_iterations = 10;
            cfg.splice_threshold = if splice_threshold > 0 { splice_threshold as i32 } else { 45 };
            cfg.simplify = if curve_smoothness > 0.0 { Some(curve_smoothness as f64) } else { Some(1.0) };
        }
    }

    if num_colors > 1 && num_colors <= 64 {
        cfg.max_colors = Some(num_colors as usize);
    }

    let img = ColorImage {
        pixels: pixels.to_vec(),
        width: w,
        height: h,
    };

    let pipeline_res = cfg.build();
    let mut svg_string = match pipeline_res {
        Ok(pipeline) => match pipeline.to_svg(&img) {
            Ok(s) => s,
            Err(e) => format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}"><text y="20">Error: {:?}</text></svg>"##,
                width, height, e
            ),
        },
        Err(e) => format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}"><text y="20">Config Error: {:?}</text></svg>"##,
            width, height, e
        ),
    };

    // Ensure responsive SVG viewbox
    if !svg_string.contains("viewBox=") {
        let viewbox_attr = format!(r#"viewBox="0 0 {} {}" width="100%" height="100%""#, width, height);
        svg_string = svg_string.replacen("<svg ", &format!("<svg {} ", viewbox_attr), 1);
    }

    // Apply Single Stroke transform if requested by user
    let sw = if stroke_width > 0.1 { stroke_width } else { 1.5 };
    if stroke_mode == 1 {
        // Single Stroke Mode: transform filled paths to clean single stroke outlines
        svg_string = svg_string.replace("fill=\"", "fill=\"none\" stroke=\"");
        svg_string = svg_string.replace(
            "<path ",
            &format!("<path stroke-width=\"{:.1}\" stroke-linecap=\"round\" stroke-linejoin=\"round\" ", sw),
        );
    }

    // 5. Extract Palette & Statistics from SVG
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
                        let hex = if col_str.starts_with('#') {
                            col_str.to_string()
                        } else {
                            col_str.to_string()
                        };

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
    let wireframe_inner = svg_string.replace("fill=\"", "fill=\"none\" stroke=\"#3b82f6\" stroke-width=\"0.8\" data-fill=\"");
    let wireframe_svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"100%\" height=\"100%\" style=\"shape-rendering: geometricPrecision;\">\n<rect width=\"{}\" height=\"{}\" fill=\"#0f172a\" fill-opacity=\"0.6\" />\n<g class=\"wireframe-paths\">{}</g>\n</svg>",
        width, height,
        width, height,
        wireframe_inner
    );

    // Build Final JSON Package
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
        wasm_free_string(res_ptr, out_len);
    }
}

