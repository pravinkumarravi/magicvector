import React from 'react';
import {
  Columns,
  SplitSquareVertical,
  Layers,
  Image as ImageIcon,
  Network,
  ZoomIn,
  ZoomOut,
  Maximize2,
  Minimize2,
  Hand,
  RotateCcw,
  Grid
} from 'lucide-react';
import { ViewMode, ToolMode, BgStyle } from '../types';

interface ToolbarProps {
  viewMode: ViewMode;
  setViewMode: (mode: ViewMode) => void;
  toolMode: ToolMode;
  setToolMode: (mode: ToolMode) => void;
  zoom: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onZoomFit: () => void;
  onZoom100: () => void;
  bgStyle: BgStyle;
  setBgStyle: (style: BgStyle) => void;
  onResetSettings: () => void;
}

export default function Toolbar({
  viewMode,
  setViewMode,
  toolMode,
  setToolMode,
  zoom: _zoom,
  onZoomIn,
  onZoomOut,
  onZoomFit,
  onZoom100,
  bgStyle,
  setBgStyle,
  onResetSettings
}: ToolbarProps) {
  const toggleBgStyle = () => {
    switch (bgStyle) {
      case 'checker-dark': setBgStyle('checker-light'); break;
      case 'checker-light': setBgStyle('solid-white'); break;
      case 'solid-white': setBgStyle('solid-dark'); break;
      case 'solid-dark': setBgStyle('checker-dark'); break;
      default: setBgStyle('checker-dark');
    }
  };

  return (
    <div className="sub-toolbar">
      {/* View Mode Switcher */}
      <div className="toolbar-group">
        <button
          className={`btn ${viewMode === 'split' ? 'btn-active' : ''}`}
          onClick={() => setViewMode('split')}
          title="Split View: Compare Original & Vector with draggable divider"
        >
          <SplitSquareVertical size={14} />
          <span>Split View</span>
        </button>

        <button
          className={`btn ${viewMode === 'sideBySide' ? 'btn-active' : ''}`}
          onClick={() => setViewMode('sideBySide')}
          title="Side-by-Side: Dual synchronous viewing panes"
        >
          <Columns size={14} />
          <span>Side by Side</span>
        </button>

        <button
          className={`btn ${viewMode === 'vector' ? 'btn-active' : ''}`}
          onClick={() => setViewMode('vector')}
          title="Vector Result Only"
        >
          <Layers size={14} />
          <span>Vector Only</span>
        </button>

        <button
          className={`btn ${viewMode === 'original' ? 'btn-active' : ''}`}
          onClick={() => setViewMode('original')}
          title="Original Bitmap Only"
        >
          <ImageIcon size={14} />
          <span>Original</span>
        </button>

        <button
          className={`btn ${viewMode === 'wireframe' ? 'btn-active' : ''}`}
          onClick={() => setViewMode('wireframe')}
          title="Wireframe / Hurricane Mode: Inspect Bézier curves and nodes"
        >
          <Network size={14} />
          <span>Wireframe Paths</span>
        </button>
      </div>

      <div className="toolbar-divider" />

      {/* Navigation Tools */}
      <div className="toolbar-group">
        <button
          className={`btn btn-icon-only ${toolMode === 'pan' ? 'btn-active' : ''}`}
          onClick={() => setToolMode(toolMode === 'pan' ? 'select' : 'pan')}
          title="Hand / Pan Tool (Drag canvas to move)"
        >
          <Hand size={14} />
        </button>

        <button
          className="btn btn-icon-only"
          onClick={onZoomIn}
          title="Zoom In (+)"
        >
          <ZoomIn size={14} />
        </button>

        <button
          className="btn btn-icon-only"
          onClick={onZoomOut}
          title="Zoom Out (-)"
        >
          <ZoomOut size={14} />
        </button>

        <button
          className="btn"
          onClick={onZoomFit}
          title="Zoom to Fit Viewport"
        >
          <Maximize2 size={13} />
          <span>Fit</span>
        </button>

        <button
          className="btn"
          onClick={onZoom100}
          title="Actual Size 1:1 (100%)"
        >
          <Minimize2 size={13} />
          <span>100%</span>
        </button>

        <div className="toolbar-divider" />

        {/* Background style toggle */}
        <button
          className={`btn btn-icon-only ${bgStyle === 'checker-dark' ? 'btn-active' : ''}`}
          onClick={toggleBgStyle}
          title={`Canvas Backdrop: ${bgStyle} (Click to toggle)`}
        >
          <Grid size={14} />
        </button>

        <button
          className="btn btn-icon-only"
          onClick={onResetSettings}
          title="Reset Parameters & View"
        >
          <RotateCcw size={14} />
        </button>
      </div>
    </div>
  );
}
