import React, { useState, useEffect, useCallback } from 'react';
import Navbar from './components/Navbar';
import Toolbar from './components/Toolbar';
import Viewport from './components/Viewport';
import Sidebar from './components/Sidebar';
import DropZone from './components/DropZone';
import { vectorizeImage, PRESETS } from './engine/tracer';
import {
  PresetKey,
  VectorOptions,
  PaletteItem,
  VectorStats,
  ViewMode,
  ToolMode,
  BgStyle
} from './types';

export default function App() {
  const [imageSrc, setImageSrc] = useState<string | null>(null);
  const [fileName, setFileName] = useState<string>('vectormagic_output');
  const [imageDimensions, setImageDimensions] = useState<{ width: number; height: number }>({ width: 0, height: 0 });
  const [rawImageData, setRawImageData] = useState<ImageData | null>(null);

  // Vector Result State
  const [svgString, setSvgString] = useState<string>('');
  const [wireframeSvg, setWireframeSvg] = useState<string>('');
  const [palette, setPalette] = useState<PaletteItem[]>([]);
  const [stats, setStats] = useState<VectorStats | null>(null);
  const [isProcessing, setIsProcessing] = useState<boolean>(false);

  // Settings State
  const [preset, setPreset] = useState<PresetKey>('logo');
  const [options, setOptions] = useState<VectorOptions>({
    ...PRESETS.logo.options,
    preset: 'logo',
    strokeMode: 'fill-stroke',
    removeBackground: false,
    bgTolerance: 25
  });

  // Viewport State
  const [viewMode, setViewMode] = useState<ViewMode>('split');
  const [toolMode, setToolMode] = useState<ToolMode>('select');
  const [zoom, setZoom] = useState<number>(1);
  const [pan, setPan] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
  const [bgStyle, setBgStyle] = useState<BgStyle>('checker-dark');
  const [copied, setCopied] = useState<boolean>(false);

  // Process image data through tracing engine
  const executeVectorization = useCallback(async (imgData: ImageData, currentOpts: VectorOptions) => {
    if (!imgData) return;
    setIsProcessing(true);
    try {
      const result = await vectorizeImage(imgData, currentOpts);
      setSvgString(result.svgString);
      setWireframeSvg(result.wireframeSvg);
      setPalette(result.palette);
      setStats(result.stats);
    } catch (err) {
      console.error('Vectorization error:', err);
    } finally {
      setIsProcessing(false);
    }
  }, []);

  // Load an image element or URL into canvas
  const loadImage = useCallback((src: string, name: string = 'vectorized') => {
    const img = new Image();
    
    // Crucial: Only set crossOrigin for remote http(s) URLs.
    // Setting crossOrigin on data: or blob: URLs taints the canvas in Chromium.
    if (src.startsWith('http://') || src.startsWith('https://')) {
      img.crossOrigin = 'anonymous';
    }

    img.onload = () => {
      try {
        // Preserve native resolution up to 4K (4096px) to avoid bilinear resampling blur on fine details
        const maxDim = 4096;
        let w = img.naturalWidth || img.width;
        let h = img.naturalHeight || img.height;

        if (w > maxDim || h > maxDim) {
          if (w > h) {
            h = Math.round((h * maxDim) / w);
            w = maxDim;
          } else {
            w = Math.round((w * maxDim) / h);
            h = maxDim;
          }
        }

        const canvas = document.createElement('canvas');
        canvas.width = w;
        canvas.height = h;
        const ctx = canvas.getContext('2d', { willReadFrequently: true });
        if (!ctx) return;

        ctx.drawImage(img, 0, 0, w, h);
        const imgData = ctx.getImageData(0, 0, w, h);

        setImageSrc(src);
        setFileName(name.replace(/\.[^/.]+$/, ''));
        setImageDimensions({ width: w, height: h });
        setRawImageData(imgData);

        // Always reset view to split mode so vectorization is immediately visible
        setViewMode('split');
        setPan({ x: 0, y: 0 });
        setZoom(1);

        executeVectorization(imgData, options);
      } catch (err) {
        console.error('Failed to process image canvas data:', err);
      }
    };

    img.onerror = (err) => {
      console.error('Failed to load image element:', err);
    };

    img.src = src;
  }, [executeVectorization, options]);

  // Clear current image and return to upload DropZone screen
  const handleClearImage = () => {
    setImageSrc(null);
    setRawImageData(null);
    setSvgString('');
    setWireframeSvg('');
    setPalette([]);
    setStats(null);
  };

  // File Upload Handler
  const handleFileSelect = (file: File) => {
    if (!file) return;
    try {
      const objectUrl = URL.createObjectURL(file);
      loadImage(objectUrl, file.name);
    } catch {
      const reader = new FileReader();
      reader.onload = (e) => {
        if (typeof e.target?.result === 'string') {
          loadImage(e.target.result, file.name);
        }
      };
      reader.readAsDataURL(file);
    }
  };

  // Clipboard Paste Handler (Ctrl + V)
  useEffect(() => {
    const handlePaste = (e: ClipboardEvent) => {
      if (e.clipboardData && e.clipboardData.items) {
        for (let i = 0; i < e.clipboardData.items.length; i++) {
          const item = e.clipboardData.items[i];
          if (item.type.indexOf('image') !== -1) {
            const blob = item.getAsFile();
            if (blob) {
              handleFileSelect(blob);
              break;
            }
          }
        }
      }
    };
    window.addEventListener('paste', handlePaste);
    return () => window.removeEventListener('paste', handlePaste);
  }, []);

  // Zoom Helpers
  const handleZoomIn = () => setZoom(z => Number(Math.min(32, z * 1.25).toFixed(2)));
  const handleZoomOut = () => setZoom(z => Number(Math.max(0.1, z * 0.8).toFixed(2)));
  const handleZoomFit = () => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
  };
  const handleZoom100 = () => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
  };

  // Retrace Button Handler
  const handleRetrace = (customOpts?: VectorOptions) => {
    if (rawImageData) {
      executeVectorization(rawImageData, customOpts || options);
    }
  };

  // Download SVG
  const handleDownloadSvg = () => {
    if (!svgString) return;
    const blob = new Blob([svgString], { type: 'image/svg+xml;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${fileName}.svg`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  // Copy SVG Code
  const handleCopySvg = () => {
    if (!svgString) return;
    navigator.clipboard.writeText(svgString).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2200);
    });
  };

  // Export Scaled PNG
  const handleExportPng = (scaleMultiplier: number = 2) => {
    if (!svgString || !imageDimensions.width) return;
    const { width, height } = imageDimensions;
    const canvas = document.createElement('canvas');
    canvas.width = width * scaleMultiplier;
    canvas.height = height * scaleMultiplier;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const svgBlob = new Blob([svgString], { type: 'image/svg+xml;charset=utf-8' });
    const url = URL.createObjectURL(svgBlob);
    const img = new Image();

    img.onload = () => {
      ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
      URL.revokeObjectURL(url);
      const pngUrl = canvas.toDataURL('image/png');
      const a = document.createElement('a');
      a.href = pngUrl;
      a.download = `${fileName}_${scaleMultiplier}x.png`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
    };
    img.src = url;
  };

  // Reset to default settings
  const handleResetSettings = () => {
    const defaultOpts: VectorOptions = {
      ...PRESETS.logo.options,
      preset: 'logo',
      removeBackground: false,
      bgTolerance: 25
    };
    setPreset('logo');
    setOptions(defaultOpts);
    setZoom(1);
    setPan({ x: 0, y: 0 });
    if (rawImageData) {
      executeVectorization(rawImageData, defaultOpts);
    }
  };

  return (
    <div className="app-container">
      {/* Top Navbar */}
      <Navbar
        imageLoaded={!!imageSrc}
        onFileSelect={handleFileSelect}
        onLoadSample={(sample) => loadImage(`/samples/${sample}`, sample)}
        onDownloadSvg={handleDownloadSvg}
        onCopySvg={handleCopySvg}
        copied={copied}
        stats={stats}
        onClearImage={handleClearImage}
      />

      {/* Sub-toolbar */}
      {imageSrc && (
        <Toolbar
          viewMode={viewMode}
          setViewMode={setViewMode}
          toolMode={toolMode}
          setToolMode={setToolMode}
          zoom={zoom}
          onZoomIn={handleZoomIn}
          onZoomOut={handleZoomOut}
          onZoomFit={handleZoomFit}
          onZoom100={handleZoom100}
          bgStyle={bgStyle}
          setBgStyle={setBgStyle}
          onResetSettings={handleResetSettings}
        />
      )}

      {/* Main Workspace Layout */}
      <div className="app-main">
        {imageSrc ? (
          <>
            <Viewport
              imageSrc={imageSrc}
              imageDimensions={imageDimensions}
              svgString={svgString}
              wireframeSvg={wireframeSvg}
              viewMode={viewMode}
              setViewMode={setViewMode}
              toolMode={toolMode}
              zoom={zoom}
              setZoom={setZoom}
              pan={pan}
              setPan={setPan}
              bgStyle={bgStyle}
              isProcessing={isProcessing}
              stats={stats}
            />

            <Sidebar
              preset={preset}
              setPreset={setPreset}
              options={options}
              setOptions={setOptions}
              palette={palette}
              stats={stats}
              isProcessing={isProcessing}
              onRetrace={handleRetrace}
              onDownloadSvg={handleDownloadSvg}
              onCopySvg={handleCopySvg}
              onExportPng={handleExportPng}
              copied={copied}
            />
          </>
        ) : (
          <DropZone
            onFileSelect={handleFileSelect}
            onLoadSample={(sample) => loadImage(`/samples/${sample}`, sample)}
          />
        )}
      </div>
    </div>
  );
}
