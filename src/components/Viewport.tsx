import React, { useState, useRef, useEffect, MouseEvent, WheelEvent } from 'react';
import { ViewMode, ToolMode, BgStyle, VectorStats } from '../types';
import { SplitSquareVertical, Layers, Image as ImageIcon, Network, CheckCircle } from 'lucide-react';

interface ViewportProps {
  imageSrc: string | null;
  imageDimensions: { width: number; height: number };
  svgString: string;
  wireframeSvg: string;
  viewMode: ViewMode;
  setViewMode: (mode: ViewMode) => void;
  toolMode: ToolMode;
  zoom: number;
  setZoom: React.Dispatch<React.SetStateAction<number>>;
  pan: { x: number; y: number };
  setPan: React.Dispatch<React.SetStateAction<{ x: number; y: number }>>;
  bgStyle: BgStyle;
  isProcessing: boolean;
  stats?: VectorStats | null;
}

export default function Viewport({
  imageSrc,
  imageDimensions,
  svgString,
  wireframeSvg,
  viewMode,
  setViewMode,
  toolMode,
  zoom,
  setZoom,
  pan,
  setPan,
  bgStyle,
  isProcessing,
  stats
}: ViewportProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [splitPos, setSplitPos] = useState<number>(50);
  const [isDraggingSplit, setIsDraggingSplit] = useState<boolean>(false);
  const [isPanning, setIsPanning] = useState<boolean>(false);
  const [panStart, setPanStart] = useState<{ x: number; y: number }>({ x: 0, y: 0 });

  // Handle Wheel Zoom
  const handleWheel = (e: WheelEvent<HTMLDivElement>) => {
    e.preventDefault();
    const zoomFactor = e.deltaY < 0 ? 1.15 : 0.85;
    setZoom((prevZoom) => {
      const nextZoom = Math.min(32, Math.max(0.1, prevZoom * zoomFactor));
      return Number(nextZoom.toFixed(2));
    });
  };

  // Handle Pan Start
  const handleMouseDown = (e: MouseEvent<HTMLDivElement>) => {
    const target = e.target as HTMLElement;
    if (target.closest('.split-handle') || target.closest('.split-pill') || target.closest('.top-floating-bar')) return;

    if (e.button === 0 || toolMode === 'pan') {
      setIsPanning(true);
      setPanStart({ x: e.clientX - pan.x, y: e.clientY - pan.y });
    }
  };

  // Handle Mouse Move
  const handleMouseMove = (e: MouseEvent<HTMLDivElement>) => {
    if (isDraggingSplit && containerRef.current) {
      const rect = containerRef.current.getBoundingClientRect();
      const relativeX = e.clientX - rect.left;
      const percentage = Math.max(2, Math.min(98, (relativeX / rect.width) * 100));
      setSplitPos(percentage);
    } else if (isPanning) {
      setPan({
        x: e.clientX - panStart.x,
        y: e.clientY - panStart.y
      });
    }
  };

  // Handle Mouse Up
  const handleMouseUp = () => {
    setIsDraggingSplit(false);
    setIsPanning(false);
  };

  useEffect(() => {
    window.addEventListener('mouseup', handleMouseUp);
    return () => window.removeEventListener('mouseup', handleMouseUp);
  }, []);

  const getBgClass = () => {
    switch (bgStyle) {
      case 'checker-light': return 'checkerboard-light';
      case 'solid-white': return 'bg-solid-white';
      case 'solid-dark': return 'bg-solid-dark';
      case 'checker-dark':
      default: return 'checkerboard-pattern';
    }
  };

  if (!imageSrc) return null;

  const contentWidth = imageDimensions.width;
  const contentHeight = imageDimensions.height;

  // Base transform style for zoom and pan
  const transformStyle: React.CSSProperties = {
    transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
    transformOrigin: 'center center',
    width: `${contentWidth}px`,
    height: `${contentHeight}px`,
    transition: isPanning ? 'none' : 'transform 0.05s ease-out',
    position: 'relative'
  };

  const cursorClass = isPanning ? 'cursor-grabbing' : toolMode === 'pan' ? 'cursor-grab' : 'cursor-default';

  return (
    <div
      ref={containerRef}
      className={`viewport-container ${getBgClass()} ${cursorClass}`}
      onWheel={handleWheel}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      style={{ userSelect: 'none', cursor: isPanning ? 'grabbing' : toolMode === 'pan' ? 'grab' : 'default' }}
    >
      {/* Top Floating View Mode Badges */}
      <div className="top-floating-bar" style={{
        position: 'absolute',
        top: '1rem',
        left: '1rem',
        display: 'flex',
        alignItems: 'center',
        gap: '0.4rem',
        background: 'rgba(15, 23, 42, 0.85)',
        backdropFilter: 'blur(12px)',
        border: '1px solid var(--border-subtle)',
        borderRadius: '30px',
        padding: '0.3rem 0.5rem',
        zIndex: 40,
        boxShadow: 'var(--shadow-md)'
      }}>
        <button
          className={`btn ${viewMode === 'split' ? 'btn-active' : ''}`}
          style={{ padding: '0.3rem 0.7rem', borderRadius: '20px', fontSize: '0.75rem' }}
          onClick={() => setViewMode('split')}
        >
          <SplitSquareVertical size={13} />
          <span>Split (50/50)</span>
        </button>

        <button
          className={`btn ${viewMode === 'vector' ? 'btn-active' : ''}`}
          style={{ padding: '0.3rem 0.7rem', borderRadius: '20px', fontSize: '0.75rem' }}
          onClick={() => setViewMode('vector')}
        >
          <Layers size={13} />
          <span>Vector Only</span>
        </button>

        <button
          className={`btn ${viewMode === 'original' ? 'btn-active' : ''}`}
          style={{ padding: '0.3rem 0.7rem', borderRadius: '20px', fontSize: '0.75rem' }}
          onClick={() => setViewMode('original')}
        >
          <ImageIcon size={13} />
          <span>Original Only</span>
        </button>

        <button
          className={`btn ${viewMode === 'wireframe' ? 'btn-active' : ''}`}
          style={{ padding: '0.3rem 0.7rem', borderRadius: '20px', fontSize: '0.75rem' }}
          onClick={() => setViewMode('wireframe')}
        >
          <Network size={13} />
          <span>Wireframe</span>
        </button>
      </div>

      {/* Top-Right Status Badge */}
      <div style={{
        position: 'absolute',
        top: '1rem',
        right: '1rem',
        background: 'rgba(15, 23, 42, 0.85)',
        backdropFilter: 'blur(10px)',
        border: isProcessing ? '1px solid var(--accent-gold)' : '1px solid var(--border-subtle)',
        borderRadius: '8px',
        padding: '0.45rem 0.85rem',
        display: 'flex',
        alignItems: 'center',
        gap: '0.5rem',
        zIndex: 40,
        boxShadow: isProcessing ? 'var(--shadow-gold)' : 'var(--shadow-sm)'
      }}>
        {isProcessing ? (
          <>
            <div className="spinner" style={{ color: 'var(--accent-gold)' }} />
            <span style={{ fontSize: '0.75rem', color: '#f8fafc', fontWeight: 500 }}>
              Vectorizing Image...
            </span>
          </>
        ) : stats ? (
          <>
            <CheckCircle size={14} color="#10b981" />
            <span style={{ fontSize: '0.75rem', color: '#f8fafc', fontWeight: 500 }}>
              SVG Ready ({stats.pathCount} paths • {stats.colorCount} colors)
            </span>
          </>
        ) : null}
      </div>

      {/* Render Mode 1: SPLIT VIEW (Draggable Divider) */}
      {viewMode === 'split' && (
        <div style={transformStyle}>
          {/* Layer 1: Original Bitmap (Left side clipped) */}
          <div
            style={{
              position: 'absolute',
              inset: 0,
              clipPath: `polygon(0 0, ${splitPos}% 0, ${splitPos}% 100%, 0 100%)`,
              zIndex: 10
            }}
          >
            <img
              src={imageSrc}
              alt="Original Raster"
              style={{ width: '100%', height: '100%', objectFit: 'contain', display: 'block' }}
              draggable={false}
            />
            {/* Split Tag Original */}
            <div style={{
              position: 'absolute',
              top: '12px',
              left: '12px',
              background: 'rgba(0, 0, 0, 0.8)',
              color: '#fff',
              fontSize: '11px',
              padding: '4px 9px',
              borderRadius: '4px',
              fontWeight: 600,
              letterSpacing: '0.5px',
              border: '1px solid rgba(255, 255, 255, 0.2)'
            }}>
              ORIGINAL BITMAP
            </div>
          </div>

          {/* Layer 2: Vector SVG (Right side clipped) */}
          <div
            style={{
              position: 'absolute',
              inset: 0,
              clipPath: `polygon(${splitPos}% 0, 100% 0, 100% 100%, ${splitPos}% 100%)`,
              zIndex: 15
            }}
          >
            {svgString ? (
              <div
                dangerouslySetInnerHTML={{ __html: svgString }}
                style={{ width: '100%', height: '100%', display: 'flex' }}
              />
            ) : (
              <div style={{
                width: '100%',
                height: '100%',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color: 'var(--text-muted)',
                fontSize: '0.875rem'
              }}>
                Rendering Vector...
              </div>
            )}
            {/* Split Tag Vector */}
            <div style={{
              position: 'absolute',
              top: '12px',
              right: '12px',
              background: 'rgba(241, 196, 15, 0.95)',
              color: '#0b0f19',
              fontSize: '11px',
              padding: '4px 9px',
              borderRadius: '4px',
              fontWeight: 700,
              letterSpacing: '0.5px',
              boxShadow: '0 2px 8px rgba(0,0,0,0.4)'
            }}>
              VECTOR MAGIC SVG
            </div>
          </div>

          {/* Draggable Divider Handle */}
          <div
            className="split-handle"
            style={{ left: `${splitPos}%` }}
            onMouseDown={(e) => {
              e.stopPropagation();
              setIsDraggingSplit(true);
            }}
          >
            <div className="split-pill">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <polyline points="15 18 9 12 15 6" />
                <polyline points="9 18 3 12 9 6" />
                <polyline points="21 18 15 12 21 6" />
              </svg>
            </div>
          </div>
        </div>
      )}

      {/* Render Mode 2: SIDE BY SIDE (Synchronized Dual Panes) */}
      {viewMode === 'sideBySide' && (
        <div style={{
          display: 'flex',
          width: '100%',
          height: '100%',
          position: 'relative'
        }}>
          {/* Left Pane: Original */}
          <div style={{
            flex: 1,
            position: 'relative',
            overflow: 'hidden',
            borderRight: '1px solid var(--border-subtle)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center'
          }}>
            <div style={transformStyle}>
              <img
                src={imageSrc}
                alt="Original"
                style={{ width: '100%', height: '100%', objectFit: 'contain' }}
                draggable={false}
              />
            </div>
            <div style={{
              position: 'absolute',
              top: '12px',
              left: '12px',
              background: 'rgba(0, 0, 0, 0.75)',
              color: '#fff',
              fontSize: '11px',
              padding: '4px 10px',
              borderRadius: '6px',
              fontWeight: 600,
              zIndex: 30
            }}>
              ORIGINAL BITMAP
            </div>
          </div>

          {/* Right Pane: Vector SVG */}
          <div style={{
            flex: 1,
            position: 'relative',
            overflow: 'hidden',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center'
          }}>
            <div style={transformStyle}>
              {svgString && (
                <div
                  dangerouslySetInnerHTML={{ __html: svgString }}
                  style={{ width: '100%', height: '100%' }}
                />
              )}
            </div>
            <div style={{
              position: 'absolute',
              top: '12px',
              left: '12px',
              background: 'rgba(241, 196, 15, 0.9)',
              color: '#0b0f19',
              fontSize: '11px',
              padding: '4px 10px',
              borderRadius: '6px',
              fontWeight: 700,
              zIndex: 30
            }}>
              VECTOR SVG RESULT
            </div>
          </div>
        </div>
      )}

      {/* Render Mode 3: VECTOR ONLY */}
      {viewMode === 'vector' && (
        <div style={transformStyle}>
          {svgString ? (
            <div
              dangerouslySetInnerHTML={{ __html: svgString }}
              style={{ width: '100%', height: '100%' }}
            />
          ) : (
            <div style={{ color: 'var(--text-muted)', fontSize: '1rem' }}>
              Vector output calculating...
            </div>
          )}
          <div style={{
            position: 'absolute',
            bottom: '12px',
            right: '12px',
            background: 'rgba(241, 196, 15, 0.95)',
            color: '#0b0f19',
            fontSize: '11px',
            padding: '4px 10px',
            borderRadius: '6px',
            fontWeight: 700
          }}>
            PURE VECTOR SVG
          </div>
        </div>
      )}

      {/* Render Mode 4: ORIGINAL ONLY */}
      {viewMode === 'original' && (
        <div style={transformStyle}>
          <img
            src={imageSrc}
            alt="Original"
            style={{ width: '100%', height: '100%', objectFit: 'contain' }}
            draggable={false}
          />
          <div style={{
            position: 'absolute',
            bottom: '12px',
            right: '12px',
            background: 'rgba(0, 0, 0, 0.8)',
            color: '#fff',
            fontSize: '11px',
            padding: '4px 10px',
            borderRadius: '6px',
            fontWeight: 600
          }}>
            ORIGINAL RASTER BITMAP
          </div>
        </div>
      )}

      {/* Render Mode 5: WIREFRAME / HURRICANE MODE */}
      {viewMode === 'wireframe' && (
        <div style={transformStyle}>
          {wireframeSvg ? (
            <div
              dangerouslySetInnerHTML={{ __html: wireframeSvg }}
              style={{ width: '100%', height: '100%' }}
            />
          ) : (
            svgString && (
              <div
                dangerouslySetInnerHTML={{ __html: svgString }}
                style={{ width: '100%', height: '100%' }}
              />
            )
          )}
          <div style={{
            position: 'absolute',
            bottom: '12px',
            right: '12px',
            background: 'rgba(15, 23, 42, 0.9)',
            color: '#38bdf8',
            fontSize: '11px',
            padding: '4px 10px',
            borderRadius: '6px',
            border: '1px solid rgba(56, 189, 248, 0.4)',
            fontWeight: 600
          }}>
            HURRICANE BÉZIER NODES & TANGENTS
          </div>
        </div>
      )}

      {/* Floating HUD Pill */}
      <div className="viewport-hud">
        <span className="hud-info">
          {Math.round(zoom * 100)}%
        </span>
        <div style={{ width: 1, height: 14, background: 'var(--border-subtle)' }} />
        <span className="hud-info">
          {contentWidth} × {contentHeight} px
        </span>
        <div style={{ width: 1, height: 14, background: 'var(--border-subtle)' }} />
        <button
          className="btn"
          style={{ padding: '0.2rem 0.5rem', fontSize: '0.6875rem' }}
          onClick={() => {
            setZoom(1);
            setPan({ x: 0, y: 0 });
          }}
        >
          Reset View
        </button>
      </div>
    </div>
  );
}
