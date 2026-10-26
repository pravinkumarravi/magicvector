<div align="center">

<img src="./public/logo.svg" width="128" height="128" alt="Vector Magic Logo" />

# Vector Magic — Precision Image to SVG Vectorizer

**A high-performance in-browser Bitmap to SVG conversion studio built with React 19 and TypeScript.**  
*Reverse engineered from Vector Magic's editor workflow, featuring real-time split-view comparison, Bézier curve fitting, and Hurricane wireframe inspection.*

[![TypeScript](https://img.shields.io/badge/TypeScript-5.x-blue.svg?style=for-the-badge&logo=typescript)](https://www.typescriptlang.org/)
[![React](https://img.shields.io/badge/React-19.x-61DAFB.svg?style=for-the-badge&logo=react)](https://react.dev/)
[![Vite](https://img.shields.io/badge/Vite-8.x-646CFF.svg?style=for-the-badge&logo=vite)](https://vitejs.dev/)
[![License](https://img.shields.io/badge/License-MIT-green.svg?style=for-the-badge)](LICENSE)

</div>

---

## 🌟 Key Features

- ⚡ **100% Client-Side Tracing**: Converts any raster image (`PNG`, `JPG`, `WEBP`, `BMP`, `GIF`) into crisp, resolution-independent vector SVGs entirely inside the browser. No uploads to external servers.
- 🪟 **Interactive Draggable Split Slider**: Real-time comparison slider with synchronized zoom and pan, showing the original bitmap on the left and the vector SVG on the right.
- 👥 **Side-by-Side Dual View**: Dual synchronized canvas panes for comparative inspection.
- 🌀 **Hurricane Wireframe Mode**: Reverse engineered from Vector Magic's outline inspector — visualizes every Bézier anchor node, tangent handle, and path boundary.
- 🎛️ **Precision Image Presets**:
  - **Logo / Graphic**: Clean vectors, sharp corners, and minimal path noise.
  - **Anti-aliased Artwork**: Smooth Bézier curves for illustrations, drawings, and scans.
  - **Photo / Complex**: Multi-palette quantization for rich color imagery.
  - **Pixel Art / Crisp**: Strict pixel grid preservation without curve deformation.
  - **Monochrome / Silhouette**: 2-color stencil, stamp, and silhouette tracing.
- 🎨 **Palette Extraction & Segmentation**: Interactive swatches with hex codes, path counts, and node metrics.
- 🪄 **Background Keyout**: Automatically detects dominant background color and keyouts background to produce clean transparent SVGs.
- 💾 **Multi-Format Export**:
  - Clean, optimized `.svg` file download
  - 1-click SVG markup copying to clipboard
  - High-resolution raster export (1×, 2× HD, 4× Print)

---

## 🛠️ Reverse Engineering Architecture

The application layout, controls, and workflows were reverse engineered from Vector Magic's web interface bundles (`Main.min_*.js`, `gs_*.js`, and `all_*.js`):

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Top Navbar (Logo & Actions)                     │
├────────────────────────────────────────────────────────────────────────┤
│          Toolbar (View Modes: Split, Dual, Vector, Wireframe)          │
├─────────────────────────────────────────┬──────────────────────────────┤
│                                         │   Vector Settings Sidebar    │
│                                         │                              │
│       Interactive Viewport              │  • Image Presets (Logo/Art)  │
│                                         │  • Detail & Noise Filters    │
│  [Original Bitmap] ║ [Vector SVG]       │  • Bézier Spline Resolution  │
│                    ║                    │  • Palette Swatches Grid     │
│             Draggable Split             │  • Background Removal        │
│                                         │  • Export Center (SVG/PNG)   │
│                                         │  • Real-time Performance     │
└─────────────────────────────────────────┴──────────────────────────────┘
```

---

## 🚀 Quick Start

### Prerequisites
- [Node.js](https://nodejs.org/) (v18 or higher recommended)
- [npm](https://www.npmjs.com/) or [pnpm](https://pnpm.io/)

### Installation

```bash
# Clone repository
git clone https://github.com/pravinkumarravi/vectormagic.git

# Navigate to project directory
cd vectormagic

# Install dependencies
npm install

# Start development server
npm run dev
```

Open your browser at `http://127.0.0.1:5173/` to start vectorizing.

### Production Build

```bash
npm run build
npm run preview
```

---

## 📦 Tech Stack

- **Framework**: [React 19](https://react.dev/)
- **Language**: [TypeScript](https://www.typescriptlang.org/)
- **Build Tool**: [Vite 8](https://vitejs.dev/)
- **Icons**: [Lucide React](https://lucide.dev/)
- **Effects**: [Canvas Confetti](https://github.com/catdad/canvas-confetti)
- **Tracing Engine**: Custom wrapper over [ImageTracerJS](https://github.com/jankovicsandras/imagetracerjs) with customized presets and Bézier node extraction.

---

## 📄 License

This project is open-source and available under the [MIT License](LICENSE).
