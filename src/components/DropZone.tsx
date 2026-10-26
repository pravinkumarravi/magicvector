import React, { useState, DragEvent } from 'react';
import { Upload } from 'lucide-react';

interface DropZoneProps {
  onFileSelect: (file: File) => void;
  onLoadSample: (sample: string) => void;
}

export default function DropZone({ onFileSelect, onLoadSample: _onLoadSample }: DropZoneProps) {
  const [isDragging, setIsDragging] = useState<boolean>(false);

  const handleDragOver = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    setIsDragging(true);
  };

  const handleDragLeave = () => {
    setIsDragging(false);
  };

  const handleDrop = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    setIsDragging(false);
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      onFileSelect(e.dataTransfer.files[0]);
    }
  };

  return (
    <div
      className="dropzone-overlay"
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      <div className={`dropzone-card ${isDragging ? 'dragging' : ''}`}>
        {/* Brand Icon */}
        <div style={{
          width: 64,
          height: 64,
          borderRadius: 16,
          background: 'rgba(241, 196, 15, 0.1)',
          border: '1px solid rgba(241, 196, 15, 0.3)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          boxShadow: 'var(--shadow-gold)'
        }}>
          <svg viewBox="0 0 16 16" width="38" height="38" xmlns="http://www.w3.org/2000/svg">
            <g style={{ strokeWidth: '1.4px', strokeLinecap: 'round' }}>
              <rect x="0.75" y="0.75" width="14.5" height="14.5" rx="3" style={{ fill: '#eab819', stroke: '#718fa4' }} />
              <circle cx="8" cy="8" r="4.5" style={{ fill: 'none', stroke: 'white' }} />
              <circle cx="8" cy="8" r="1.5" style={{ fill: 'white', stroke: 'white' }} />
              <rect x="8" y="8" width="7.25" height="7.25" style={{ fill: 'white', stroke: '#718fa4' }} />
              <line x1="8" y1="15.25" x2="15.25" y2="8" style={{ stroke: '#718fa4' }} />
            </g>
          </svg>
        </div>

        <div>
          <h2 style={{ fontSize: '1.5rem', fontWeight: 700, marginBottom: '0.4rem', color: '#f8fafc' }}>
            Magic Vector Studio
          </h2>
          <p style={{ color: 'var(--text-secondary)', fontSize: '0.875rem', maxWidth: '380px', lineHeight: 1.5 }}>
            Drag and drop any bitmap logo, drawing or photo to vectorize it automatically into silky smooth SVG curves.
          </p>
        </div>

        <label className="btn btn-primary" style={{ padding: '0.75rem 1.75rem', fontSize: '0.9375rem', cursor: 'pointer' }}>
          <Upload size={16} />
          <span>Upload Image File</span>
          <input
            type="file"
            accept="image/*"
            style={{ display: 'none' }}
            onChange={(e) => {
              if (e.target.files && e.target.files[0]) {
                onFileSelect(e.target.files[0]);
              }
            }}
          />
        </label>

        <div style={{ color: 'var(--text-muted)', fontSize: '0.75rem' }}>
          Tip: You can also press <kbd style={{ padding: '2px 5px', background: 'rgba(255,255,255,0.1)', borderRadius: 4 }}>Ctrl</kbd> + <kbd style={{ padding: '2px 5px', background: 'rgba(255,255,255,0.1)', borderRadius: 4 }}>V</kbd> to paste from clipboard
        </div>
      </div>
    </div>
  );
}
