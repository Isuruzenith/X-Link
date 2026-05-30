import React, { useEffect, useRef } from 'react';

interface SpeedDataPoint {
  up: number; // in bytes/sec
  down: number; // in bytes/sec
}

interface TrafficChartProps {
  history: SpeedDataPoint[];
}

export const TrafficChart: React.FC<TrafficChartProps> = ({ history }) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  // Helper to format speed to human readable format (e.g. 1.2 MB/s, 450 KB/s)
  const formatSpeed = (bytesPerSec: number): string => {
    if (bytesPerSec === 0) return '0 B/s';
    const k = 1024;
    const sizes = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
    const i = Math.floor(Math.log(bytesPerSec) / Math.log(k));
    return parseFloat((bytesPerSec / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  };

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Handle high DPI screens (retina)
    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    ctx.scale(dpr, dpr);

    const width = rect.width;
    const height = rect.height;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Padding
    const padding = { top: 20, right: 15, bottom: 25, left: 55 };
    const chartWidth = width - padding.left - padding.right;
    const chartHeight = height - padding.top - padding.bottom;

    // Draw background grid lines
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.05)';
    ctx.lineWidth = 1;
    const gridCount = 4;
    for (let i = 0; i <= gridCount; i++) {
      const y = padding.top + (chartHeight / gridCount) * i;
      ctx.beginPath();
      ctx.moveTo(padding.left, y);
      ctx.lineTo(width - padding.right, y);
      ctx.stroke();
    }

    // If there is no history, draw empty state
    if (history.length === 0) {
      ctx.fillStyle = 'rgba(255, 255, 255, 0.3)';
      ctx.font = '13px "Outfit", "Inter", sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText('No real-time traffic data', width / 2, height / 2);
      return;
    }

    // Calculate maximum speed in the history to scale the Y-axis
    let maxSpeed = 1024 * 100; // Minimum scale of 100 KB/s
    history.forEach((pt) => {
      if (pt.up > maxSpeed) maxSpeed = pt.up;
      if (pt.down > maxSpeed) maxSpeed = pt.down;
    });

    // Add a 15% buffer at the top
    maxSpeed = maxSpeed * 1.15;

    // Draw Y axis labels
    ctx.fillStyle = 'rgba(255, 255, 255, 0.4)';
    ctx.font = '10px "Inter", sans-serif';
    ctx.textAlign = 'right';
    ctx.textBaseline = 'middle';
    for (let i = 0; i <= gridCount; i++) {
      const val = maxSpeed * (1 - i / gridCount);
      const y = padding.top + (chartHeight / gridCount) * i;
      ctx.fillText(formatSpeed(val), padding.left - 8, y);
    }

    const pointsCount = 30; // Display last 30 points max
    const displayHistory = history.slice(-pointsCount);

    const getCoordinates = (index: number, value: number) => {
      const x = padding.left + (chartWidth / (pointsCount - 1)) * index;
      const y = padding.top + chartHeight - (chartHeight * (value / maxSpeed));
      return { x, y };
    };

    // Helper to draw bezier path
    const drawCurve = (
      data: number[],
      strokeColor: string,
      fillGradient: CanvasGradient,
      glowColor: string
    ) => {
      if (data.length < 2) return;

      ctx.save();

      // 1. Draw area gradient fill under curve
      ctx.beginPath();
      let firstPt = getCoordinates(0, data[0]);
      ctx.moveTo(firstPt.x, firstPt.y);

      for (let i = 0; i < data.length - 1; i++) {
        const p0 = getCoordinates(i, data[i]);
        const p1 = getCoordinates(i + 1, data[i + 1]);
        const cpX1 = p0.x + (p1.x - p0.x) / 2;
        const cpY1 = p0.y;
        const cpX2 = p0.x + (p1.x - p0.x) / 2;
        const cpY2 = p1.y;
        ctx.bezierCurveTo(cpX1, cpY1, cpX2, cpY2, p1.x, p1.y);
      }

      // Complete the fill path back to the bottom of the chart
      const lastPt = getCoordinates(data.length - 1, data[data.length - 1]);
      ctx.lineTo(lastPt.x, padding.top + chartHeight);
      ctx.lineTo(padding.left, padding.top + chartHeight);
      ctx.closePath();
      ctx.fillStyle = fillGradient;
      ctx.fill();

      // 2. Draw glowing curve stroke
      ctx.shadowColor = glowColor;
      ctx.shadowBlur = 8;
      ctx.strokeStyle = strokeColor;
      ctx.lineWidth = 2.5;
      ctx.lineCap = 'round';
      ctx.lineJoin = 'round';

      ctx.beginPath();
      ctx.moveTo(firstPt.x, firstPt.y);

      for (let i = 0; i < data.length - 1; i++) {
        const p0 = getCoordinates(i, data[i]);
        const p1 = getCoordinates(i + 1, data[i + 1]);
        const cpX1 = p0.x + (p1.x - p0.x) / 2;
        const cpY1 = p0.y;
        const cpX2 = p0.x + (p1.x - p0.x) / 2;
        const cpY2 = p1.y;
        ctx.bezierCurveTo(cpX1, cpY1, cpX2, cpY2, p1.x, p1.y);
      }
      ctx.stroke();
      ctx.restore();
    };

    // Pad the data if we have fewer than pointsCount
    const paddedDown = new Array(pointsCount - displayHistory.length).fill(0).concat(
      displayHistory.map((h) => h.down)
    );
    const paddedUp = new Array(pointsCount - displayHistory.length).fill(0).concat(
      displayHistory.map((h) => h.up)
    );

    // Create Gradients
    // Download (Teal glow)
    const downGrad = ctx.createLinearGradient(0, padding.top, 0, padding.top + chartHeight);
    downGrad.addColorStop(0, 'rgba(102, 252, 241, 0.25)');
    downGrad.addColorStop(1, 'rgba(102, 252, 241, 0.00)');

    // Upload (Purple/Violet glow)
    const upGrad = ctx.createLinearGradient(0, padding.top, 0, padding.top + chartHeight);
    upGrad.addColorStop(0, 'rgba(168, 85, 247, 0.25)');
    upGrad.addColorStop(1, 'rgba(168, 85, 247, 0.00)');

    // Draw Download Curve first (background)
    drawCurve(paddedDown, '#66FCF1', downGrad, 'rgba(102, 252, 241, 0.5)');

    // Draw Upload Curve second (foreground)
    drawCurve(paddedUp, '#A855F7', upGrad, 'rgba(168, 85, 247, 0.5)');

    // Draw X-axis line (subtle)
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.15)';
    ctx.beginPath();
    ctx.moveTo(padding.left, padding.top + chartHeight);
    ctx.lineTo(width - padding.right, padding.top + chartHeight);
    ctx.stroke();

    // Draw timeline markers (e.g. -30s, -15s, Now)
    ctx.fillStyle = 'rgba(255, 255, 255, 0.35)';
    ctx.font = '9px "Inter", sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'top';

    ctx.fillText('30s ago', padding.left, padding.top + chartHeight + 6);
    ctx.fillText('15s ago', padding.left + chartWidth / 2, padding.top + chartHeight + 6);
    ctx.textAlign = 'right';
    ctx.fillText('Live', width - padding.right, padding.top + chartHeight + 6);

  }, [history]);

  return (
    <div style={{ width: '100%', height: '100%', position: 'relative' }}>
      <canvas
        ref={canvasRef}
        style={{
          width: '100%',
          height: '100%',
          display: 'block',
        }}
      />
    </div>
  );
};
