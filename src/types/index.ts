export type PresetKey = 'logo' | 'artwork' | 'photo' | 'pixel' | 'silhouette';

export type ViewMode = 'split' | 'sideBySide' | 'vector' | 'original' | 'wireframe';

export type ToolMode = 'select' | 'pan';

export type BgStyle = 'checker-dark' | 'checker-light' | 'solid-white' | 'solid-dark';

export type Theme = 'light' | 'dark';

export interface RgbColor {
  r: number;
  g: number;
  b: number;
  a?: number;
}

export interface PaletteItem {
  hex: string;
  originalHex?: string;
  rgb: RgbColor;
  pathCount: number;
  nodeCount: number;
}

export interface VectorOptions {
  numberofcolors?: number;
  pathomit?: number;
  ltres?: number;
  qtres?: number;
  rightangleenhance?: boolean;
  linefilter?: boolean;
  blurradius?: number;
  blurdelta?: number;
  roundcoords?: number;
  layering?: number;
  scale?: number;
  viewbox?: boolean;
  desc?: boolean;
  preset?: PresetKey;
  removeBackground?: boolean;
  customBgColor?: string;
  bgTolerance?: number;
  customPalette?: RgbColor[];
  pal?: RgbColor[];
  strokeMode?: 'fill' | 'stroke' | 'fill-stroke';
  strokeWidth?: number;
  detailLevel?: 'low' | 'medium' | 'high';
  cornerThreshold?: number;
  curveSmoothness?: number;
  spliceThreshold?: number;
  colorMode?: 'auto' | 'limited' | 'custom';
  [key: string]: any;
}

export interface PresetConfig {
  name: string;
  desc: string;
  icon: string;
  options: VectorOptions;
}

export interface VectorStats {
  width: number;
  height: number;
  pathCount: number;
  nodeCount: number;
  colorCount: number;
  durationMs: number;
  svgSize: number;
  wasmAccelerated?: boolean;
}

export interface TracedataSegment {
  type: 'L' | 'Q' | string;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  cx?: number;
  cy?: number;
}

export interface TracedataPath {
  segments: TracedataSegment[];
  boundingbox?: number[];
  holechildren?: any[];
  isholepath?: boolean;
}

export interface Tracedata {
  layers: TracedataPath[][];
  palette: RgbColor[];
  width: number;
  height: number;
}

export interface VectorizeResult {
  svgString: string;
  wireframeSvg: string;
  tracedata: Tracedata;
  palette: PaletteItem[];
  detectedBg: RgbColor;
  stats: VectorStats;
}
