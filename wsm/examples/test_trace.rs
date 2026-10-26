use image::GenericImageView;
use std::fs;
use std::path::Path;

fn main() {
    println!("Loading cartoon_girl.png...");
    let img_path = Path::new("../public/samples/cartoon_girl.png");
    let dynamic_img = image::open(img_path).expect("Failed to open image");
    let rgba = dynamic_img.to_rgba8();
    let (width, height) = dynamic_img.dimensions();
    println!("Dimensions: {}x{}", width, height);
    let mut pixels = rgba.into_raw();
    let mut out_len = 0usize;

    println!("Vectorizing with wsm engine...");
    let out_ptr = wsm::wasm_vectorize(
        pixels.as_mut_ptr(),
        width,
        height,
        6,  // num_colors: exactly 6 artwork colors
        0,  // blur_radius
        0,  // remove_bg
        999, 999, 999,
        25,
        2,  // min_path_len
        0,  // stroke_mode: 0 = fill, 1 = stroke, 2 = fill+stroke
        1.0,
        2,  // detail_level: High
        50, // corner_threshold
        0.5,// curve_smoothness
        40, // splice_threshold
        &mut out_len,
    );

    if out_ptr.is_null() || out_len == 0 {
        panic!("wasm_vectorize returned null or 0 length");
    }

    let json_bytes = unsafe { std::slice::from_raw_parts(out_ptr, out_len) };
    let json_str = std::str::from_utf8(json_bytes).expect("Invalid UTF-8");
    let parsed: serde_json::Value = serde_json::from_str(json_str).expect("Invalid JSON");

    let svg = parsed["svg"].as_str().expect("No svg field");
    println!("Generated SVG size: {} bytes", svg.len());
    println!("Stats: {:?}", parsed["stats"]);
    println!("Colors: {:?}", parsed["palette"]);

    fs::write("../public/samples/test_out.svg", svg).expect("Failed to write test_out.svg");
    println!("Wrote output to ../public/samples/test_out.svg!");
}
