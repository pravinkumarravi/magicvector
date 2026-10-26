export type PresetKey = 'logo' | 'artwork' | 'photo' | 'pixel' | 'silhouette';

export type ViewMode = 'split' | 'sideBySide' | 'vector' | 'original' | 'wireframe';

export type ToolMode = 'select' | 'pan';

export type BgStyle = 'checker-dark' | 'checker-light' | 'solid-white' | 'solid-dark';

export interface RgbColor {
  r: number;
  g: number;
  b: number;
  a?: number;
}

export interface PaletteItem {
  hex: string;
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
}

export interface TracedataSegment {
  type: 'line' | 'spline';
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  cx?: number;
  cy?: number;
}

export interface TracedataPath {
  segments: TracedataSegment[];
}

export interface TracedataLayer {
  color: RgbColor;
  paths: TracedataPath[];
}

export interface Tracedata {
  layers: TracedataLayer[];
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
