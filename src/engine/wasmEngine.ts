import { RgbColor } from '../types';

interface WasmExports {
  memory: WebAssembly.Memory;
  alloc: (size: number) => number;
  dealloc: (ptr: number, size: number) => void;
  wasm_detect_background: (ptr: number, width: number, height: number, bgOutPtr: number) => void;
  wasm_remove_background: (ptr: number, len: number, tr: number, tg: number, tb: number, tol: number) => number;
  wasm_box_blur: (ptr: number, width: number, height: number, radius: number) => void;
  wasm_quantize_kmeans: (ptr: number, width: number, height: number, k: number, maxIters: number, palOutPtr: number) => number;
  wasm_despeckle: (ptr: number, width: number, height: number, threshold: number) => void;
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

/**
 * Heavy-duty K-Means color quantization in WebAssembly
 */
export function wasmQuantizeKMeans(
  imageData: ImageData,
  k: number,
  maxIters: number = 8
): { quantizedData: ImageData; palette: RgbColor[] } {
  if (!wasmInstance) {
    return { quantizedData: imageData, palette: [] };
  }

  const { width, height, data } = imageData;
  const size = data.length;
  const clampedK = Math.min(64, Math.max(2, k));

  const ptr = wasmInstance.alloc(size);
  const palOutPtr = wasmInstance.alloc(clampedK * 4);

  try {
    const memory = new Uint8Array(wasmInstance.memory.buffer);
    memory.set(data, ptr);

    const actualK = wasmInstance.wasm_quantize_kmeans(ptr, width, height, clampedK, maxIters, palOutPtr);

    const resultMem = new Uint8ClampedArray(wasmInstance.memory.buffer, ptr, size);
    const quantizedData = new ImageData(new Uint8ClampedArray(resultMem), width, height);

    const palMem = new Uint8Array(wasmInstance.memory.buffer);
    const palette: RgbColor[] = [];

    for (let i = 0; i < actualK; i++) {
      const offset = palOutPtr + i * 4;
      palette.push({
        r: palMem[offset],
        g: palMem[offset + 1],
        b: palMem[offset + 2],
        a: palMem[offset + 3]
      });
    }

    return { quantizedData, palette };
  } finally {
    wasmInstance.dealloc(ptr, size);
    wasmInstance.dealloc(palOutPtr, clampedK * 4);
  }
}

/**
 * Noise despeckle filter in WebAssembly
 */
export function wasmDespeckle(imageData: ImageData, threshold: number = 2): ImageData {
  if (!wasmInstance) return imageData;

  const { width, height, data } = imageData;
  const size = data.length;
  const ptr = wasmInstance.alloc(size);

  try {
    const memory = new Uint8Array(wasmInstance.memory.buffer);
    memory.set(data, ptr);

    wasmInstance.wasm_despeckle(ptr, width, height, threshold);

    const resultMem = new Uint8ClampedArray(wasmInstance.memory.buffer, ptr, size);
    return new ImageData(new Uint8ClampedArray(resultMem), width, height);
  } finally {
    wasmInstance.dealloc(ptr, size);
  }
}
