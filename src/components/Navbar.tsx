import React, { useRef } from 'react';
import { Upload, Sparkles, Download, Copy, Check } from 'lucide-react';
import confetti from 'canvas-confetti';
import { VectorStats } from '../types';

interface NavbarProps {
  imageLoaded: boolean;
  onFileSelect: (file: File) => void;
  onLoadSample: (sample: string) => void;
  onDownloadSvg: () => void;
  onCopySvg: () => void;
  copied: boolean;
  stats: VectorStats | null;
}

export default function Navbar({
  imageLoaded,
  onFileSelect,
  onLoadSample,
  onDownloadSvg,
  onCopySvg,
  copied,
  stats
}: NavbarProps) {
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleDownloadClick = () => {
    confetti({
      particleCount: 50,
      spread: 60,
      origin: { y: 0.1 }
    });
    onDownloadSvg();
  };

  return (
    <header className="navbar">
      <div className="brand-section">
        <div className="vm-logo-badge">
          <svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
            <g style={{ strokeWidth: '1.4px', strokeLinecap: 'round' }}>
              <rect x="0.75" y="0.75" width="14.5" height="14.5" rx="3" style={{ fill: '#eab819', stroke: '#718fa4' }} />
              <circle cx="8" cy="8" r="4.5" style={{ fill: 'none', stroke: 'white' }} />
              <circle cx="8" cy="8" r="1.5" style={{ fill: 'white', stroke: 'white' }} />
              <rect x="8" y="8" width="7.25" height="7.25" style={{ fill: 'white', stroke: '#718fa4' }} />
              <line x1="8" y1="15.25" x2="15.25" y2="8" style={{ stroke: '#718fa4' }} />
            </g>
          </svg>
        </div>
        <div className="brand-title">
          <span className="brand-name">Vector Magic</span>
          <span className="brand-tag">Precision Image to SVG</span>
        </div>
      </div>

      <div className="navbar-actions">
        <input
          ref={fileInputRef}
          type="file"
          accept="image/*"
          style={{ display: 'none' }}
          onChange={(e) => {
            if (e.target.files && e.target.files[0]) {
              onFileSelect(e.target.files[0]);
              e.target.value = '';
            }
          }}
        />

        <button
          className="btn"
          onClick={() => fileInputRef.current?.click()}
          title="Upload image from computer (PNG, JPG, WEBP, BMP, GIF)"
        >
          <Upload size={14} />
          <span>Open Image</span>
        </button>

        <div className="toolbar-group">
          <button
            className="btn"
            onClick={() => onLoadSample('falcon_logo.jpg')}
            title="Load Rocket Falcon Logo sample"
          >
            <Sparkles size={14} color="#f1c40f" />
            <span>Sample Logo</span>
          </button>
          <button
            className="btn"
            onClick={() => onLoadSample('chameleon.jpg')}
            title="Load Chameleon Mascot sample"
          >
            <Sparkles size={14} color="#10b981" />
            <span>Sample Mascot</span>
          </button>
        </div>

        <div className="toolbar-divider" />

        {imageLoaded && (
          <>
            <button
              className="btn"
              onClick={onCopySvg}
              title="Copy SVG markup code to clipboard"
            >
              {copied ? <Check size={14} color="#10b981" /> : <Copy size={14} />}
              <span>{copied ? 'Copied XML' : 'Copy SVG'}</span>
            </button>

            <button
              className="btn btn-primary"
              onClick={handleDownloadClick}
              title="Download optimized SVG vector file"
            >
              <Download size={14} />
              <span>Download SVG</span>
            </button>
          </>
        )}
      </div>
    </header>
  );
}
