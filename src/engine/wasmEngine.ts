import { RgbColor } from '../types';

interface WasmExports {
  memory: WebAssembly.Memory;
  alloc: (size: number) => number;
  dealloc: (ptr: number, size: number) => void;
  wasm_detect_background: (ptr: number, width: number, height: number, bgOutPtr: number) => void;
  wasm_remove_background: (ptr: number, len: number, tr: number, tg: number, tb: number, tol: number) => number;
  wasm_box_blur: (ptr: number, width: number, height: number, radius: number) => void;
  wasm_clean_edges: (ptr: number, width: number, height: number, threshold: number) => void;
  wasm_free_string: (ptr: number, len: number) => void;
  wasm_vectorize: (
    ptr: number,
    width: number,
    height: number,
    numColors: number,
    blurRadius: number,
    removeBg: number,
    bgR: number,
    bgG: number,
    bgB: number,
    bgTol: number,
    minPathLen: number,
    strokeMode: number,
    strokeWidth: number,
    detailLevel: number,
    cornerThreshold: number,
    curveSmoothness: number,
    spliceThreshold: number,
    outLenPtr: number
  ) => number;
}

let wasmInstance: WasmExports | null = null;
let isInitializing = false;
let initPromise: Promise<WasmExports | null> | null = null;

export function isWasmReady(): boolean {
  return wasmInstance !== null;
}

/**
 * Initialize and instantiate the WebAssembly module
 */
export async function initWasm(): Promise<WasmExports | null> {
  if (wasmInstance) return wasmInstance;
  if (initPromise) return initPromise;

  initPromise = (async () => {
    try {
      isInitializing = true;
      const response = await fetch('/wsm.wasm');
      let module: WebAssembly.WebAssemblyInstantiatedSource;

      if ('instantiateStreaming' in WebAssembly) {
        try {
          module = await WebAssembly.instantiateStreaming(response, {});
        } catch {
          const buffer = await (await fetch('/wsm.wasm')).arrayBuffer();
          module = await WebAssembly.instantiate(buffer, {});
        }
      } else {
        const buffer = await response.arrayBuffer();
        module = await WebAssembly.instantiate(buffer, {});
      }

      wasmInstance = module.instance.exports as unknown as WasmExports;
      console.log('⚡ WebAssembly Heavy-Duty Engine initialized successfully');
      return wasmInstance;
    } catch (err) {
      console.warn('WebAssembly initialization fallback to JS:', err);
      return null;
    } finally {
      isInitializing = false;
    }
  })();

  return initPromise;
}

/**
 * Detect dominant background color using WebAssembly
 */
export function wasmDetectBackground(imageData: ImageData): RgbColor | null {
  if (!wasmInstance) return null;

  const { width, height, data } = imageData;
  const size = data.length;

  const ptr = wasmInstance.alloc(size);
  const bgOutPtr = wasmInstance.alloc(4);

  try {
    const memory = new Uint8Array(wasmInstance.memory.buffer);
    memory.set(data, ptr);

    wasmInstance.wasm_detect_background(ptr, width, height, bgOutPtr);

    const outMem = new Uint8Array(wasmInstance.memory.buffer);
    const r = outMem[bgOutPtr];
    const g = outMem[bgOutPtr + 1];
    const b = outMem[bgOutPtr + 2];
    const a = outMem[bgOutPtr + 3];

    return { r, g, b, a };
  } finally {
    wasmInstance.dealloc(ptr, size);
    wasmInstance.dealloc(bgOutPtr, 4);
  }
}

/**
 * Fast in-place background keyout in WebAssembly
 */
export function wasmRemoveBackground(
  imageData: ImageData,
  targetBg: RgbColor,
  tolerance: number
): { processedData: ImageData; removedCount: number } {
  if (!wasmInstance) {
    return { processedData: imageData, removedCount: 0 };
  }

  const { width, height, data } = imageData;
  const size = data.length;

  const ptr = wasmInstance.alloc(size);

  try {
    const memory = new Uint8Array(wasmInstance.memory.buffer);
    memory.set(data, ptr);

    const removedCount = wasmInstance.wasm_remove_background(
      ptr,
      size,
      targetBg.r,
      targetBg.g,
      targetBg.b,
      tolerance
    );

    const resultMem = new Uint8ClampedArray(wasmInstance.memory.buffer, ptr, size);
    const processedData = new ImageData(new Uint8ClampedArray(resultMem), width, height);

    return { processedData, removedCount };
  } finally {
    wasmInstance.dealloc(ptr, size);
  }
}

/**
 * Fast separable 2D box blur filter in WebAssembly
 */
export function wasmBoxBlur(imageData: ImageData, radius: number): ImageData {
  if (!wasmInstance || radius <= 0) return imageData;

  const { width, height, data } = imageData;
  const size = data.length;
  const ptr = wasmInstance.alloc(size);

  try {
    const memory = new Uint8Array(wasmInstance.memory.buffer);
    memory.set(data, ptr);

    wasmInstance.wasm_box_blur(ptr, width, height, radius);

    const resultMem = new Uint8ClampedArray(wasmInstance.memory.buffer, ptr, size);
    return new ImageData(new Uint8ClampedArray(resultMem), width, height);
  } finally {
    wasmInstance.dealloc(ptr, size);
  }
}

export interface WasmVectorizeResult {
  svg: string;
  wireframe: string;
  palette: Array<{
    hex: string;
    r: number;
    g: number;
    b: number;
    pathCount: number;
    nodeCount: number;
  }>;
  detectedBg: { r: number; g: number; b: number };
  stats: {
    width: number;
    height: number;
    pathCount: number;
    nodeCount: number;
    colorCount: number;
    svgSize: number;
  };
}

/**
 * End-to-end native Rust WebAssembly Vectorizer
 * Executes background removal, blur, K-Means++ color quantization,
 * contour tracing, and midpoint Bézier smoothing entirely inside WebAssembly.
 */
export function wasmVectorizeFull(
  imageData: ImageData,
  options: {
    numColors?: number;
    blurRadius?: number;
    removeBackground?: boolean;
    bgColor?: RgbColor | null;
    bgTolerance?: number;
    pathOmit?: number;
    strokeMode?: 'fill' | 'stroke' | 'fill-stroke';
    strokeWidth?: number;
    detailLevel?: 'low' | 'medium' | 'high';
    cornerThreshold?: number;
    curveSmoothness?: number;
    spliceThreshold?: number;
  }
): WasmVectorizeResult | null {
  if (!wasmInstance) return null;

  const { width, height, data } = imageData;
  const size = data.length;

  const imgPtr = wasmInstance.alloc(size);
  const outLenPtr = wasmInstance.alloc(4);

  try {
    const memory = new Uint8Array(wasmInstance.memory.buffer);
    memory.set(data, imgPtr);

    const numColors = options.numColors || 12;
    const blurRadius = options.blurRadius || 0;
    const removeBg = options.removeBackground ? 1 : 0;
    const bgR = options.bgColor ? options.bgColor.r : 999;
    const bgG = options.bgColor ? options.bgColor.g : 999;
    const bgB = options.bgColor ? options.bgColor.b : 999;
    const bgTol = options.bgTolerance || 25;
    const minPathLen = options.pathOmit !== undefined ? options.pathOmit : 2;
    const strokeModeVal = options.strokeMode === 'stroke' ? 1 : (options.strokeMode === 'fill-stroke' ? 2 : 0);
    const strokeWidthVal = options.strokeWidth || 1.5;

    const detailLevelVal = options.detailLevel === 'high' ? 2 : (options.detailLevel === 'low' ? 0 : 1);
    const cornerThresholdVal = options.cornerThreshold || 0;
    const curveSmoothnessVal = options.curveSmoothness !== undefined ? options.curveSmoothness : 0.0;
    const spliceThresholdVal = options.spliceThreshold || 0;

    const resultPtr = wasmInstance.wasm_vectorize(
      imgPtr,
      width,
      height,
      numColors,
      blurRadius,
      removeBg,
      bgR,
      bgG,
      bgB,
      bgTol,
      minPathLen,
      strokeModeVal,
      strokeWidthVal,
      detailLevelVal,
      cornerThresholdVal,
      curveSmoothnessVal,
      spliceThresholdVal,
      outLenPtr
    );

    if (resultPtr === 0) return null;

    const lenView = new DataView(wasmInstance.memory.buffer);
    const resultLen = lenView.getUint32(outLenPtr, true);

    const jsonBytes = new Uint8Array(wasmInstance.memory.buffer, resultPtr, resultLen);
    const decoder = new TextDecoder('utf-8');
    const jsonStr = decoder.decode(jsonBytes);

    wasmInstance.wasm_free_string(resultPtr, resultLen);

    return JSON.parse(jsonStr) as WasmVectorizeResult;
  } catch (err) {
    console.error('Wasm full vectorization error:', err);
    return null;
  } finally {
    wasmInstance.dealloc(imgPtr, size);
    wasmInstance.dealloc(outLenPtr, 4);
  }
}

