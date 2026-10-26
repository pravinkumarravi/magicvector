// @ts-ignore
import ImageTracer from 'imagetracerjs';
import {
  RgbColor,
  PaletteItem,
  VectorOptions,
  PresetConfig,
  PresetKey,
  Tracedata,
  VectorizeResult
} from '../types';
import {
  initWasm,
  isWasmReady,
  wasmDetectBackground,
  wasmRemoveBackground,
  wasmVectorizeFull
} from './wasmEngine';

// Convert RGB to HEX
export function rgbToHex(r: number, g: number, b: number): string {
  const toHex = (n: number) => {
    const hex = Math.max(0, Math.min(255, Math.round(n))).toString(16);
    return hex.length === 1 ? '0' + hex : hex;
  };
  return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
}

// Convert HEX to RGB
export function hexToRgb(hex: string): RgbColor {
  const clean = hex.replace('#', '');
  if (clean.length === 3) {
    return {
      r: parseInt(clean[0] + clean[0], 16),
      g: parseInt(clean[1] + clean[1], 16),
      b: parseInt(clean[2] + clean[2], 16),
      a: 255
    };
  }
  return {
    r: parseInt(clean.substring(0, 2), 16) || 0,
    g: parseInt(clean.substring(2, 4), 16) || 0,
    b: parseInt(clean.substring(4, 6), 16) || 0,
    a: 255
  };
}

// Color distance (Euclidean in RGB space)
export function colorDistance(c1: RgbColor, c2: RgbColor): number {
  const dr = c1.r - c2.r;
  const dg = c1.g - c2.g;
  const db = c1.b - c2.b;
  return Math.sqrt(dr * dr + dg * dg + db * db);
}

// Preset mapping matching VectorMagic categories
export const PRESETS: Record<PresetKey, PresetConfig> = {
  artwork: {
    name: 'Anti-aliased Artwork',
    desc: 'High-detail cubic Bézier curves for illustrations, drawings & comics (Vector Magic High)',
    icon: 'Palette',
    options: {
      numberofcolors: 16,
      pathomit: 2,
      detailLevel: 'high',
      blurradius: 0,
      cornerThreshold: 50,
      curveSmoothness: 0.5,
      rightangleenhance: false,
      linefilter: true,
      layering: 0
    }
  },
  logo: {
    name: 'Logo / Graphic',
    desc: 'Clean vector contours & crisp curves for logos and icons',
    icon: 'Shield',
    options: {
      numberofcolors: 8,
      pathomit: 4,
      ltres: 0.5,
      qtres: 1.0,
      rightangleenhance: false,
      linefilter: true,
      blurradius: 1,
      blurdelta: 20,
      roundcoords: 2,
      layering: 0
    }
  },
  photo: {
    name: 'Photo / Gradient',
    desc: 'Rich multi-palette quantization for complex photos',
    icon: 'Image',
    options: {
      numberofcolors: 32,
      pathomit: 2,
      ltres: 1.0,
      qtres: 1.5,
      rightangleenhance: false,
      linefilter: false,
      blurradius: 2,
      blurdelta: 40,
      roundcoords: 1,
      layering: 0
    }
  },
  pixel: {
    name: 'Pixel Art / Crisp',
    desc: 'Strict grid preservation without curve smoothing',
    icon: 'Grid',
    options: {
      numberofcolors: 12,
      pathomit: 0,
      ltres: 0.01,
      qtres: 0.01,
      rightangleenhance: true,
      linefilter: false,
      blurradius: 0,
      blurdelta: 0,
      roundcoords: 0,
      layering: 0
    }
  },
  silhouette: {
    name: 'Monochrome / Silhouette',
    desc: 'Bilevel black and white stencil or stamp tracing',
    icon: 'Moon',
    options: {
      numberofcolors: 2,
      pathomit: 4,
      ltres: 0.2,
      qtres: 0.5,
      rightangleenhance: true,
      linefilter: false,
      blurradius: 0,
      blurdelta: 20,
      roundcoords: 2,
      layering: 0
    }
  }
};

/**
 * Automatically detect dominant corner/border background color
 */
export function detectBackgroundColor(imageData: ImageData): RgbColor {
  const wasmBg = wasmDetectBackground(imageData);
  if (wasmBg) return wasmBg;

  const { data, width, height } = imageData;
  const samplePoints: [number, number][] = [
    [0, 0],
    [width - 1, 0],
    [0, height - 1],
    [width - 1, height - 1],
    [Math.floor(width / 2), 0],
    [0, Math.floor(height / 2)],
    [width - 1, Math.floor(height / 2)],
    [Math.floor(width / 2), height - 1]
  ];

  let sumR = 0, sumG = 0, sumB = 0, count = 0;
  for (const [x, y] of samplePoints) {
    const idx = (y * width + x) * 4;
    if (data[idx + 3] > 10) {
      sumR += data[idx];
      sumG += data[idx + 1];
      sumB += data[idx + 2];
      count++;
    }
  }

  if (count === 0) return { r: 255, g: 255, b: 255, a: 255 };

  return {
    r: Math.round(sumR / count),
    g: Math.round(sumG / count),
    b: Math.round(sumB / count),
    a: 255
  };
}

/**
 * Pre-process image data (e.g. background removal mask, transparency)
 */
export function processImageData(
  sourceImageData: ImageData,
  { removeBackground = false, bgColor = null, bgTolerance = 25 }: { removeBackground?: boolean; bgColor?: RgbColor | null; bgTolerance?: number }
): { processedData: ImageData; detectedBg: RgbColor } {
  const targetBg = bgColor || detectBackgroundColor(sourceImageData);

  if (removeBackground) {
    if (isWasmReady()) {
      const { processedData } = wasmRemoveBackground(sourceImageData, targetBg, bgTolerance);
      return { processedData, detectedBg: targetBg };
    }

    const { width, height } = sourceImageData;
    const processed = new ImageData(new Uint8ClampedArray(sourceImageData.data), width, height);
    const data = processed.data;

    for (let i = 0; i < data.length; i += 4) {
      const r = data[i];
      const g = data[i + 1];
      const b = data[i + 2];
      const a = data[i + 3];

      if (a === 0) continue;

      const dist = Math.sqrt(
        (r - targetBg.r) ** 2 +
        (g - targetBg.g) ** 2 +
        (b - targetBg.b) ** 2
      );

      if (dist <= bgTolerance) {
        data[i + 3] = 0; // make transparent
      }
    }
    return { processedData: processed, detectedBg: targetBg };
  }

  return { processedData: sourceImageData, detectedBg: targetBg };
}

/**
 * Main Vectorization function
 */
export async function vectorizeImage(imageData: ImageData, userOptions: VectorOptions = {}): Promise<VectorizeResult> {
  const startTime = performance.now();

  // Initialize WebAssembly engine if not already loaded
  await initWasm();
  const wasmAccelerated = isWasmReady();

  const presetKey: PresetKey = userOptions.preset || 'logo';
  const presetConfig = PRESETS[presetKey]?.options || PRESETS.logo.options;

  // Merge options
  const options: VectorOptions = {
    ...presetConfig,
    ...userOptions,
    scale: 1,
    viewbox: true,
    desc: false
  };

  // 1. NATIVE RUST WEBASSEMBLY FULL PIPELINE
  if (wasmAccelerated) {
    const customBg = options.customBgColor ? hexToRgb(options.customBgColor) : null;
    const wasmRes = wasmVectorizeFull(imageData, {
      numColors: options.numberofcolors || 12,
      blurRadius: Math.round(options.blurradius || 0),
      removeBackground: !!options.removeBackground,
      bgColor: customBg,
      bgTolerance: options.bgTolerance || 25,
      pathOmit: options.pathomit !== undefined ? options.pathomit : 2,
      strokeMode: options.strokeMode || 'fill',
      strokeWidth: options.strokeWidth || 1.5,
      detailLevel: options.detailLevel || 'high',
      cornerThreshold: options.cornerThreshold || 0,
      curveSmoothness: options.curveSmoothness !== undefined ? options.curveSmoothness : 0.0,
      spliceThreshold: options.spliceThreshold || 0
    });

    if (wasmRes && wasmRes.svg) {
      const durationMs = Math.round(performance.now() - startTime);
      return {
        svgString: wasmRes.svg,
        wireframeSvg: wasmRes.wireframe,
        tracedata: { layers: [], palette: [], width: wasmRes.stats.width, height: wasmRes.stats.height },
        palette: wasmRes.palette.map(p => ({
          hex: p.hex,
          rgb: { r: p.r, g: p.g, b: p.b, a: 255 },
          pathCount: p.pathCount,
          nodeCount: p.nodeCount
        })),
        detectedBg: { ...wasmRes.detectedBg, a: 255 },
        stats: {
          width: wasmRes.stats.width,
          height: wasmRes.stats.height,
          pathCount: wasmRes.stats.pathCount,
          nodeCount: wasmRes.stats.nodeCount,
          colorCount: wasmRes.stats.colorCount,
          durationMs,
          svgSize: new Blob([wasmRes.svg]).size,
          wasmAccelerated: true
        }
      };
    }
  }

  // 2. Fallback to JS ImageTracer if Wasm is not ready
  let targetData = imageData;
  let detectedBg: RgbColor = { r: 255, g: 255, b: 255, a: 255 };

  if (options.removeBackground) {
    const res = processImageData(imageData, {
      removeBackground: true,
      bgColor: options.customBgColor ? hexToRgb(options.customBgColor) : null,
      bgTolerance: options.bgTolerance || 25
    });
    targetData = res.processedData;
    detectedBg = res.detectedBg;
  }

  // Handle custom palette in JS fallback
  if (options.customPalette && options.customPalette.length > 0) {
    options.pal = options.customPalette.map(c => ({
      r: c.r,
      g: c.g,
      b: c.b,
      a: c.a !== undefined ? c.a : 255
    }));
  }

  // Run ImageTracer
  const tracedata: Tracedata = ImageTracer.imagedataToTracedata(targetData, options);

  // Generate clean SVG markup
  let rawSvg: string = ImageTracer.getsvgstring(tracedata, options);

  // Optimize SVG attributes: ensure clean viewBox & scalable responsive design
  const { width, height } = imageData;
  const viewBoxAttr = `viewBox="0 0 ${width} ${height}"`;

  if (!rawSvg.includes('viewBox=')) {
    rawSvg = rawSvg.replace('<svg ', `<svg ${viewBoxAttr} width="100%" height="100%" `);
  } else {
    rawSvg = rawSvg.replace(/<svg\s+([^>]+)>/, (match) => {
      let updated = match;
      if (!updated.includes('width=')) updated = updated.replace('<svg ', '<svg width="100%" height="100%" ');
      return updated;
    });
  }

  // Calculate palette and metrics
  let pathCount = 0;
  let nodeCount = 0;
  const paletteMap = new Map<string, PaletteItem>();

  if (tracedata && tracedata.layers && tracedata.palette) {
    tracedata.layers.forEach((paths, layerIdx) => {
      const color = tracedata.palette[layerIdx] || { r: 0, g: 0, b: 0, a: 255 };
      const hex = rgbToHex(color.r, color.g, color.b);

      if (!paletteMap.has(hex)) {
        paletteMap.set(hex, {
          hex,
          rgb: color,
          pathCount: 0,
          nodeCount: 0
        });
      }

      const pEntry = paletteMap.get(hex)!;

      if (Array.isArray(paths)) {
        pathCount += paths.length;
        pEntry.pathCount += paths.length;

        paths.forEach((p) => {
          if (p && p.segments) {
            nodeCount += p.segments.length;
            pEntry.nodeCount += p.segments.length;
          }
        });
      }
    });
  }

  const palette = Array.from(paletteMap.values()).sort((a, b) => b.pathCount - a.pathCount);

  // Generate wireframe SVG markup for Vector Magic Hurricane inspection mode
  const wireframeSvg = generateWireframeSvg(tracedata, width, height);

  const durationMs = Math.round(performance.now() - startTime);

  return {
    svgString: rawSvg,
    wireframeSvg,
    tracedata,
    palette,
    detectedBg,
    stats: {
      width,
      height,
      pathCount,
      nodeCount,
      colorCount: palette.length,
      durationMs,
      svgSize: new Blob([rawSvg]).size,
      wasmAccelerated
    }
  };
}

/**
 * Generate wireframe / node vector outline for Vector Magic Hurricane inspection mode
 */
export function generateWireframeSvg(tracedata: Tracedata, width: number, height: number): string {
  if (!tracedata || !tracedata.layers) return '';

  let pathsMarkup = '';
  let nodesMarkup = '';

  const colors = [
    '#3b82f6', '#10b981', '#f59e0b', '#ef4444',
    '#8b5cf6', '#ec4899', '#06b6d4', '#14b8a6'
  ];

  for (let layerIdx = 0; layerIdx < tracedata.layers.length; layerIdx++) {
    const paths = tracedata.layers[layerIdx];
    const strokeColor = colors[layerIdx % colors.length];

    if (!Array.isArray(paths)) continue;

    for (const path of paths) {
      if (!path.segments || path.segments.length === 0) continue;

      let d = '';
      const segs = path.segments;

      d += `M ${segs[0].x1} ${segs[0].y1} `;
      nodesMarkup += `<circle cx="${segs[0].x1}" cy="${segs[0].y1}" r="1.5" fill="#ffffff" stroke="${strokeColor}" stroke-width="0.75" />`;

      for (let s = 0; s < segs.length; s++) {
        const seg = segs[s];
        if (seg.type === 'L' || seg.type === 'line') {
          d += `L ${seg.x2} ${seg.y2} `;
          nodesMarkup += `<circle cx="${seg.x2}" cy="${seg.y2}" r="1.2" fill="#ffffff" stroke="${strokeColor}" stroke-width="0.6" />`;
        } else if (seg.type === 'Q' || seg.type === 'spline') {
          d += `Q ${seg.cx} ${seg.cy} ${seg.x2} ${seg.y2} `;
          nodesMarkup += `<circle cx="${seg.cx}" cy="${seg.cy}" r="0.8" fill="${strokeColor}" opacity="0.7" />`;
          nodesMarkup += `<circle cx="${seg.x2}" cy="${seg.y2}" r="1.2" fill="#ffffff" stroke="${strokeColor}" stroke-width="0.6" />`;
        }
      }
      d += 'Z';

      pathsMarkup += `<path d="${d}" fill="none" stroke="${strokeColor}" stroke-width="0.8" stroke-opacity="0.85" />`;
    }
  }

  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" width="100%" height="100%" style="shape-rendering: geometricPrecision;">
    <rect width="${width}" height="${height}" fill="#0f172a" fill-opacity="0.5" />
    <g class="wireframe-paths">${pathsMarkup}</g>
    <g class="wireframe-nodes">${nodesMarkup}</g>
  </svg>`;
}
