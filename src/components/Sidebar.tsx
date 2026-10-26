import React, { useState, useEffect, useRef } from 'react';
import {
  Shield,
  Palette,
  Image as ImageIcon,
  Grid,
  Moon,
  Sliders,
  Sparkles,
  Download,
  Copy,
  Check,
  RefreshCw
} from 'lucide-react';
import { PRESETS } from '../engine/tracer';
import { PresetKey, VectorOptions, PaletteItem, VectorStats } from '../types';

interface SidebarProps {
  preset: PresetKey;
  setPreset: (preset: PresetKey) => void;
  options: VectorOptions;
  setOptions: React.Dispatch<React.SetStateAction<VectorOptions>>;
  palette: PaletteItem[];
  selectedColors?: string[];
  onToggleColor?: (hex: string) => void;
  onChangeColor?: (oldHex: string, newHex: string) => void;
  stats: VectorStats | null;
  isProcessing: boolean;
  onRetrace: (customOpts?: VectorOptions) => void;
  onDownloadSvg: () => void;
  onCopySvg: () => void;
  onExportPng: (scale: number) => void;
  copied: boolean;
  onColorClick?: (color: PaletteItem) => void;
}

export default function Sidebar({
  preset,
  setPreset,
  options,
  setOptions,
  palette,
  selectedColors,
  onToggleColor,
  onChangeColor,
  stats,
  isProcessing,
  onRetrace,
  onDownloadSvg,
  onCopySvg,
  onExportPng,
  copied,
  onColorClick: _onColorClick
}: SidebarProps) {
  const [activeTab, setActiveTab] = useState<'settings' | 'palette' | 'export'>('settings');
  const onRetraceRef = useRef(onRetrace);
  useEffect(() => { onRetraceRef.current = onRetrace; }, [onRetrace]);
  const isFirstRender = useRef(true);
  useEffect(() => {
    if (isFirstRender.current) {
      isFirstRender.current = false;
      return;
    }
    const timer = setTimeout(() => onRetraceRef.current(options), 300);
    return () => clearTimeout(timer);
  }, [options]);

  const handlePresetSelect = (key: PresetKey) => {
    setPreset(key);
    const p = PRESETS[key];
    if (p) {
      const updatedOpts: VectorOptions = {
        ...options,
        ...p.options,
        preset: key
      };
      setOptions(updatedOpts);
    }
  };

  const getPresetIcon = (key: PresetKey) => {
    switch (key) {
      case 'logo': return <Shield size={14} />;
      case 'artwork': return <Palette size={14} />;
      case 'photo': return <ImageIcon size={14} />;
      case 'pixel': return <Grid size={14} />;
      case 'silhouette': return <Moon size={14} />;
      default: return <Sliders size={14} />;
    }
  };

  return (
    <aside className="sidebar">
      {/* Sidebar Header */}
      <div className="sidebar-header">
        <div className="sidebar-title">
          <Sliders size={16} color="#f1c40f" />
          <span>Vector Settings</span>
        </div>
        <button
          className="btn btn-primary"
          style={{ padding: '0.35rem 0.75rem', fontSize: '0.75rem' }}
          onClick={onRetrace}
          disabled={isProcessing}
        >
          <RefreshCw size={12} className={isProcessing ? 'spinner' : ''} />
          <span>{isProcessing ? 'Tracing...' : 'Update Vector'}</span>
        </button>
      </div>

      {/* Tabs */}
      <div style={{
        display: 'flex',
        borderBottom: '1px solid var(--border-subtle)',
        background: 'rgba(0, 0, 0, 0.2)'
      }}>
        <button
          className={`btn ${activeTab === 'settings' ? 'btn-active' : ''}`}
          style={{ flex: 1, borderRadius: 0, border: 'none', borderBottom: activeTab === 'settings' ? '2px solid var(--accent-gold)' : 'none', justifyContent: 'center' }}
          onClick={() => setActiveTab('settings')}
        >
          <Sliders size={13} />
          <span>Tracing</span>
        </button>
        <button
          className={`btn ${activeTab === 'palette' ? 'btn-active' : ''}`}
          style={{ flex: 1, borderRadius: 0, border: 'none', borderBottom: activeTab === 'palette' ? '2px solid var(--accent-gold)' : 'none', justifyContent: 'center' }}
          onClick={() => setActiveTab('palette')}
        >
          <Palette size={13} />
          <span>Colors ({palette ? palette.length : 0})</span>
        </button>
        <button
          className={`btn ${activeTab === 'export' ? 'btn-active' : ''}`}
          style={{ flex: 1, borderRadius: 0, border: 'none', borderBottom: activeTab === 'export' ? '2px solid var(--accent-gold)' : 'none', justifyContent: 'center' }}
          onClick={() => setActiveTab('export')}
        >
          <Download size={13} />
          <span>Export</span>
        </button>
      </div>

      {/* TAB 1: SETTINGS */}
      {activeTab === 'settings' && (
        <>
          {/* Preset Selector */}
          <div className="sidebar-section">
            <div className="section-label">
              <span>Image Type & Presets</span>
              <Sparkles size={13} color="#f1c40f" />
            </div>
            <div className="preset-grid">
              {(Object.keys(PRESETS) as PresetKey[]).map((key) => {
                const item = PRESETS[key];
                return (
                  <div
                    key={key}
                    className={`preset-card ${preset === key ? 'active' : ''}`}
                    onClick={() => handlePresetSelect(key)}
                  >
                    <div className="preset-card-title">
                      {getPresetIcon(key)}
                      <span>{item.name}</span>
                    </div>
                    <div className="preset-card-desc">{item.desc}</div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Precision & Detail Sliders */}
          <div className="sidebar-section">
            <div className="section-label">Detail Level (Vector Magic Quality)</div>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: '0.35rem', marginBottom: '0.85rem' }}>
              {(['low', 'medium', 'high'] as const).map((lvl) => (
                <button
                  key={lvl}
                  className={`btn ${(options.detailLevel || 'high') === lvl ? 'btn-primary' : ''}`}
                  style={{
                    justifyContent: 'center',
                    fontSize: '0.75rem',
                    padding: '0.45rem 0.5rem',
                    textTransform: 'capitalize'
                  }}
                  onClick={() => {
                    const newOpts: VectorOptions = {
                      ...options,
                      detailLevel: lvl,
                      pathomit: lvl === 'high' ? 2 : (lvl === 'medium' ? 4 : 8),
                      curveSmoothness: lvl === 'high' ? 0.5 : (lvl === 'medium' ? 1.0 : 1.8),
                      cornerThreshold: lvl === 'high' ? 50 : (lvl === 'medium' ? 60 : 75)
                    };
                    setOptions(newOpts);
                  }}
                >
                  {lvl}
                </button>
              ))}
            </div>

            {/* Color Count Slider */}
            <div className="control-row">
              <div className="control-header">
                <span className="control-name">Palette Color Count</span>
                <span className="control-value">{options.numberofcolors || 16} colors</span>
              </div>
              <input
                type="range"
                min="2"
                max="64"
                step="1"
                value={options.numberofcolors || 16}
                onChange={(e) => setOptions(prev => ({ ...prev, numberofcolors: parseInt(e.target.value) }))}
              />
            </div>

            {/* Small Detail & Noise Filter */}
            <div className="control-row">
              <div className="control-header">
                <span className="control-name">Speckle / Detail Suppression</span>
                <span className="control-value">{options.pathomit !== undefined ? options.pathomit : 2} px</span>
              </div>
              <input
                type="range"
                min="0"
                max="16"
                step="1"
                value={options.pathomit !== undefined ? options.pathomit : 2}
                onChange={(e) => setOptions(prev => ({ ...prev, pathomit: parseInt(e.target.value) }))}
              />
            </div>

            {/* Curve Smoothness (Paper.js / vtracer simplify) */}
            <div className="control-row">
              <div className="control-header">
                <span className="control-name">Curve Smoothness</span>
                <span className="control-value">{(options.curveSmoothness !== undefined ? options.curveSmoothness : 0.5).toFixed(1)}</span>
              </div>
              <input
                type="range"
                min="0.1"
                max="3.0"
                step="0.1"
                value={options.curveSmoothness !== undefined ? options.curveSmoothness : 0.5}
                onChange={(e) => setOptions(prev => ({ ...prev, curveSmoothness: parseFloat(e.target.value) }))}
              />
            </div>

            {/* Corner Angle Threshold */}
            <div className="control-row">
              <div className="control-header">
                <span className="control-name">Corner Sharpness Angle</span>
                <span className="control-value">{options.cornerThreshold || 50}°</span>
              </div>
              <input
                type="range"
                min="30"
                max="90"
                step="5"
                value={options.cornerThreshold || 50}
                onChange={(e) => setOptions(prev => ({ ...prev, cornerThreshold: parseInt(e.target.value) }))}
              />
            </div>

            {/* Curve Sharpness */}
            <div className="control-row">
              <div className="control-header">
                <span className="control-name">Bézier Spline Resolution</span>
                <span className="control-value">{options.qtres || 1}</span>
              </div>
              <input
                type="range"
                min="0.01"
                max="5"
                step="0.05"
                value={options.qtres || 1}
                onChange={(e) => setOptions(prev => ({ ...prev, qtres: parseFloat(e.target.value) }))}
              />
            </div>

            {/* Anti-Alias Smoothing Blur */}
            <div className="control-row">
              <div className="control-header">
                <span className="control-name">Pre-filtering Blur</span>
                <span className="control-value">{options.blurradius || 0} px</span>
              </div>
              <input
                type="range"
                min="0"
                max="8"
                step="1"
                value={options.blurradius || 0}
                onChange={(e) => setOptions(prev => ({ ...prev, blurradius: parseInt(e.target.value) }))}
              />
            </div>

            {/* Corner Enhance Toggle */}
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginTop: '0.75rem' }}>
              <span style={{ fontSize: '0.8125rem', color: 'var(--text-secondary)' }}>
                Right-Angle Corner Enhance
              </span>
              <label className="switch">
                <input
                  type="checkbox"
                  checked={!!options.rightangleenhance}
                  onChange={(e) => setOptions(prev => ({ ...prev, rightangleenhance: e.target.checked }))}
                />
                <span className="switch-slider" />
              </label>
            </div>
          </div>

          {/* Path & Stroke Mode */}
          <div className="sidebar-section">
            <div className="section-label">Path & Stroke Style</div>
            
            <div style={{ display: 'grid', gridTemplateColumns: '1.2fr 0.9fr 0.9fr', gap: '0.35rem', marginBottom: '0.75rem' }}>
              <button
                className={`btn ${(options.strokeMode || 'fill-stroke') === 'fill-stroke' ? 'btn-primary' : ''}`}
                style={{ justifyContent: 'center', fontSize: '0.72rem', padding: '0.45rem 0.3rem', textAlign: 'center' }}
                onClick={() => {
                  const newOpts = { ...options, strokeMode: 'fill-stroke' as const };
                  setOptions(newOpts);
                }}
                title="Vector Magic Standard: Solid fill with 1px anti-seam stroke for seamless, continuous line art"
              >
                Fill + Stroke (VM)
              </button>
              <button
                className={`btn ${options.strokeMode === 'fill' ? 'btn-primary' : ''}`}
                style={{ justifyContent: 'center', fontSize: '0.72rem', padding: '0.45rem 0.3rem', textAlign: 'center' }}
                onClick={() => {
                  const newOpts = { ...options, strokeMode: 'fill' as const };
                  setOptions(newOpts);
                }}
                title="Pure solid filled vector paths without stroke outlines"
              >
                Pure Fill
              </button>
              <button
                className={`btn ${options.strokeMode === 'stroke' ? 'btn-primary' : ''}`}
                style={{ justifyContent: 'center', fontSize: '0.72rem', padding: '0.45rem 0.3rem', textAlign: 'center' }}
                onClick={() => {
                  const newOpts = { ...options, strokeMode: 'stroke' as const };
                  setOptions(newOpts);
                }}
                title="Single stroke line art without solid fill (ideal for plotters & laser cutting)"
              >
                Wireframe
              </button>
            </div>

            {options.strokeMode === 'stroke' && (
              <div className="control-row">
                <div className="control-header">
                  <span className="control-name">Stroke Line Width</span>
                  <span className="control-value">{(options.strokeWidth || 1.5).toFixed(1)} px</span>
                </div>
                <input
                  type="range"
                  min="0.5"
                  max="6"
                  step="0.25"
                  value={options.strokeWidth || 1.5}
                  onChange={(e) => setOptions(prev => ({ ...prev, strokeWidth: parseFloat(e.target.value) }))}
                />
              </div>
            )}
          </div>

          {/* Background Removal */}
          <div className="sidebar-section">
            <div className="section-label">Background & Transparency</div>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '0.75rem' }}>
              <span style={{ fontSize: '0.8125rem', color: 'var(--text-secondary)' }}>
                Remove Background (Keyout)
              </span>
              <label className="switch">
              <input
                type="checkbox"
                checked={!!options.removeBackground}
                onChange={(e) => {
                  const newOpts = { ...options, removeBackground: e.target.checked };
                  setOptions(newOpts);
                }}
              />
                <span className="switch-slider" />
              </label>
            </div>

            {options.removeBackground && (
              <div className="control-row" style={{ marginTop: '0.75rem' }}>
                <div className="control-header">
                  <span className="control-name">Background Color Tolerance</span>
                  <span className="control-value">±{options.bgTolerance || 25}</span>
                </div>
                <input
                  type="range"
                  min="5"
                  max="100"
                  step="1"
                  value={options.bgTolerance || 25}
                  onChange={(e) => setOptions(prev => ({ ...prev, bgTolerance: parseInt(e.target.value) }))}
                />
              </div>
            )}
          </div>
        </>
      )}

      {/* TAB 2: PALETTE & SEGMENTATION */}
      {activeTab === 'palette' && (
        <div className="sidebar-section">
          <div className="section-label">
            <span>Color Palette & Segments</span>
            <span style={{ color: 'var(--accent-gold)' }}>{palette ? palette.length : 0} Colors</span>
          </div>

          <p style={{ fontSize: '0.75rem', color: 'var(--text-muted)', marginBottom: '0.75rem' }}>
            Click on any color swatch to view detailed metrics or inspect segments.
          </p>

          <div className="swatches-grid">
            {palette && palette.map((color, idx) => (
              <div
                key={idx}
                className="color-swatch"
                style={{ backgroundColor: color.hex, opacity: (selectedColors || []).includes(color.hex) ? 1 : 0.35 }}
                onClick={() => onToggleColor && onToggleColor(color.hex)}
                title={`Color: ${color.hex}\nPaths: ${color.pathCount}\nNodes: ${color.nodeCount}`}
              />
            ))}
          </div>

          <div style={{ marginTop: '1.25rem', display: 'flex', flexDirection: 'column', gap: '0.5rem', maxHeight: '220px', overflowY: 'auto', paddingRight: '0.25rem' }}>
            {palette && palette.slice(0, 8).map((color, idx) => (
              <div
                key={idx}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  padding: '0.45rem 0.75rem',
                  background: 'rgba(255, 255, 255, 0.03)',
                  border: '1px solid var(--border-subtle)',
                  borderRadius: '6px',
                  fontSize: '0.75rem'
                }}
              >
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.6rem' }}>
                <input
                  type="checkbox"
                  checked={(selectedColors || []).includes(color.originalHex || color.hex)}
                  onChange={() => onToggleColor && onToggleColor(color.originalHex || color.hex)}
                  style={{ cursor: 'pointer' }}
                />
                <input
                  type="color"
                  value={color.hex}
                  onChange={(e) => onChangeColor && onChangeColor(color.originalHex || color.hex, e.target.value)}
                  style={{ width: 22, height: 22, border: 'none', background: 'none', cursor: 'pointer', padding: 0 }}
                  title="Change color"
                />
                <div style={{ width: 14, height: 14, borderRadius: 3, backgroundColor: color.hex, border: '1px solid rgba(255, 255, 255, 0.2)' }} />
                <span style={{ fontFamily: 'var(--font-mono)', fontWeight: 600 }}>{color.hex.toUpperCase()}</span>
              </div>
                <div style={{ color: 'var(--text-muted)' }}>
                  {color.pathCount} paths ({color.nodeCount} nodes)
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* TAB 3: EXPORT CENTER */}
      {activeTab === 'export' && (
        <div className="sidebar-section">
          <div className="section-label">Export & Formats</div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
            <button
              className="btn btn-primary"
              style={{ padding: '0.75rem 1rem', fontSize: '0.875rem', justifyContent: 'center' }}
              onClick={onDownloadSvg}
            >
              <Download size={16} />
              <span>Download Vector SVG (.svg)</span>
            </button>

            <button
              className="btn"
              style={{ padding: '0.65rem 1rem', justifyContent: 'center' }}
              onClick={onCopySvg}
            >
              {copied ? <Check size={16} color="#10b981" /> : <Copy size={16} />}
              <span>{copied ? 'Copied XML to Clipboard' : 'Copy SVG Markup'}</span>
            </button>

            <div className="toolbar-divider" style={{ width: '100%', height: 1, margin: '0.5rem 0' }} />

            <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', marginBottom: '0.25rem' }}>
              High-Resolution Rasterization:
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: '0.4rem' }}>
              <button className="btn" style={{ justifyContent: 'center' }} onClick={() => onExportPng(1)}>
                PNG 1×
              </button>
              <button className="btn" style={{ justifyContent: 'center' }} onClick={() => onExportPng(2)}>
                PNG 2× (HD)
              </button>
              <button className="btn" style={{ justifyContent: 'center' }} onClick={() => onExportPng(4)}>
                PNG 4× (Print)
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Metrics & Performance Box */}
      {stats && (
        <div className="sidebar-section" style={{ marginTop: 'auto' }}>
          <div className="section-label">
            <span>Vector Metrics</span>
            <span style={{ color: '#10b981' }}>{stats.durationMs} ms</span>
          </div>

          <div className="stats-grid">
            <div className="stat-box">
              <div className="stat-label">Vector Paths</div>
              <div className="stat-val">{stats.pathCount.toLocaleString()}</div>
            </div>
            <div className="stat-box">
              <div className="stat-label">Bézier Nodes</div>
              <div className="stat-val">{stats.nodeCount.toLocaleString()}</div>
            </div>
            <div className="stat-box">
              <div className="stat-label">Palette Size</div>
              <div className="stat-val">{stats.colorCount} colors</div>
            </div>
            <div className="stat-box">
              <div className="stat-label">SVG File Size</div>
              <div className="stat-val">{(stats.svgSize / 1024).toFixed(1)} KB</div>
            </div>
          </div>

        </div>
      )}
    </aside>
  );
}
