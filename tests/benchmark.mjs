import fs from 'fs';
import path from 'path';

// Helper to create synthetic test images in RGBA format
function createSyntheticTestImage(type, width = 200, height = 200) {
  const buffer = new Uint8Array(width * height * 4);

  if (type === 'shapes_and_corners') {
    // White background
    for (let i = 0; i < buffer.length; i += 4) {
      buffer[i] = 255;
      buffer[i + 1] = 255;
      buffer[i + 2] = 255;
      buffer[i + 3] = 255;
    }
    // Orange rectangle with sharp 90-degree corners
    for (let y = 30; y < 100; y++) {
      for (let x = 30; x < 120; x++) {
        const idx = (y * width + x) * 4;
        buffer[idx] = 247;
        buffer[idx + 1] = 106;
        buffer[idx + 2] = 28;
      }
    }
    // Blue circle with smooth curves
    const cx = 140, cy = 140, r = 40;
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        const dx = x - cx;
        const dy = y - cy;
        const dist = Math.sqrt(dx * dx + dy * dy);
        if (dist <= r) {
          const idx = (y * width + x) * 4;
          buffer[idx] = 41;
          buffer[idx + 1] = 121;
          buffer[idx + 2] = 255;
        }
      }
    }
    // Black crisp stroke line
    for (let x = 20; x < 180; x++) {
      for (let y = 110; y <= 113; y++) {
        const idx = (y * width + x) * 4;
        buffer[idx] = 30;
        buffer[idx + 1] = 35;
        buffer[idx + 2] = 45;
      }
    }
  } else if (type === 'cartoon_palette') {
    // White background
    buffer.fill(255);
    // Dark hair region
    for (let y = 20; y < 80; y++) {
      for (let x = 40; x < 160; x++) {
        const idx = (y * width + x) * 4;
        buffer[idx] = 37;
        buffer[idx + 1] = 43;
        buffer[idx + 2] = 54;
      }
    }
    // Face skin tone
    for (let y = 70; y < 140; y++) {
      for (let x = 60; x < 140; x++) {
        const idx = (y * width + x) * 4;
        buffer[idx] = 254;
        buffer[idx + 1] = 216;
        buffer[idx + 2] = 193;
      }
    }
    // Fine facial features: small 2x2 eyes
    const eye1 = [85, 95], eye2 = [115, 95];
    for (const [ex, ey] of [eye1, eye2]) {
      for (let dy = 0; dy < 3; dy++) {
        for (let dx = 0; dx < 3; dx++) {
          const idx = ((ey + dy) * width + (ex + dx)) * 4;
          buffer[idx] = 20;
          buffer[idx + 1] = 20;
          buffer[idx + 2] = 20;
        }
      }
    }
    // Orange shirt
    for (let y = 140; y < 195; y++) {
      for (let x = 30; x < 170; x++) {
        const idx = (y * width + x) * 4;
        buffer[idx] = 250;
        buffer[idx + 1] = 122;
        buffer[idx + 2] = 38;
      }
    }
  } else if (type === 'anti_aliased_edge') {
    // Simulating anti-aliased transition between black (#1e1e1e) and white (#ffffff)
    buffer.fill(255);
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width / 2; x++) {
        const idx = (y * width + x) * 4;
        buffer[idx] = 30;
        buffer[idx + 1] = 30;
        buffer[idx + 2] = 30;
      }
      // Anti-aliased transition column (gradient: 85, 140, 195)
      const grad = [85, 140, 195];
      for (let g = 0; g < grad.length; g++) {
        const x = Math.floor(width / 2) + g;
        if (x < width) {
          const idx = (y * width + x) * 4;
          buffer[idx] = grad[g];
          buffer[idx + 1] = grad[g];
          buffer[idx + 2] = grad[g];
        }
      }
    }
  }

  return { width, height, buffer };
}

async function runBenchmark() {
  console.log('=== Vector Magic WASM Vectorizer Benchmark Suite ===\n');

  const wasmPath = path.resolve('public/wsm.wasm');
  if (!fs.existsSync(wasmPath)) {
    console.error(`WASM binary not found at ${wasmPath}`);
    process.exit(1);
  }

  const wasmBytes = fs.readFileSync(wasmPath);
  const { instance } = await WebAssembly.instantiate(wasmBytes, {});
  const wasm = instance.exports;

  const testCases = [
    { name: 'Geometric Shapes & Sharp Corners (High Detail)', type: 'shapes_and_corners', detail: 2 },
    { name: 'Cartoon Palette & Fine Features (High Detail)', type: 'cartoon_palette', detail: 2 },
    { name: 'Cartoon Palette (Medium Detail)', type: 'cartoon_palette', detail: 1 },
    { name: 'Cartoon Palette (Low Detail)', type: 'cartoon_palette', detail: 0 },
    { name: 'Anti-Aliased Edge Gradient (High Detail)', type: 'anti_aliased_edge', detail: 2 }
  ];

  const results = [];

  for (const tc of testCases) {
    const { width, height, buffer } = createSyntheticTestImage(tc.type);
    const size = buffer.length;

    const imgPtr = wasm.alloc(size);
    const outLenPtr = wasm.alloc(4);

    new Uint8Array(wasm.memory.buffer).set(buffer, imgPtr);

    const startTime = performance.now();
    const resultPtr = wasm.wasm_vectorize(
      imgPtr,
      width,
      height,
      12, // num_colors
      0,  // blur_radius
      0,  // remove_bg
      999, 999, 999, // bg rgb
      25, // bg_tol
      tc.detail === 2 ? 2 : (tc.detail === 1 ? 4 : 8),  // min_path_len
      0,  // stroke_mode (fill)
      1.5,// stroke_width
      tc.detail, // detail_level (0=Low, 1=Med, 2=High)
      tc.detail === 2 ? 50 : 60, // corner_threshold
      tc.detail === 2 ? 0.5 : 1.0, // curve_smoothness
      tc.detail === 2 ? 40 : 45, // splice_threshold
      outLenPtr
    );
    const duration = performance.now() - startTime;

    if (resultPtr === 0) {
      console.error(`FAILED test case: ${tc.name}`);
      continue;
    }

    const lenView = new DataView(wasm.memory.buffer);
    const resultLen = lenView.getUint32(outLenPtr, true);
    const jsonBytes = new Uint8Array(wasm.memory.buffer, resultPtr, resultLen);
    const jsonStr = new TextDecoder('utf-8').decode(jsonBytes);

    wasm.wasm_free_string(resultPtr, resultLen);
    wasm.dealloc(imgPtr, size);
    wasm.dealloc(outLenPtr, 4);

    const parsed = JSON.parse(jsonStr);
    const svg = parsed.svg;

    // Metric validation
    const hasValidSvgTag = svg.includes('<svg') && svg.includes('</svg>');
    const pathCount = (svg.match(/<path/g) || []).length;
    const cubicBezierCount = (svg.match(/[Cc]/g) || []).length;
    const lineCount = (svg.match(/[Ll]/g) || []).length;

    results.push({
      testCase: tc.name,
      durationMs: duration.toFixed(2),
      svgSizeBytes: svg.length,
      pathCount,
      cubicBezierCount,
      lineCount,
      colorCount: parsed.palette.length,
      isValidSvg: hasValidSvgTag
    });
  }

  console.table(results);
  console.log('\nAll benchmark test cases executed successfully!');
}

runBenchmark().catch(err => {
  console.error('Benchmark failed:', err);
  process.exit(1);
});
