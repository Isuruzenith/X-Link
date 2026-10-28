import React, { useEffect, useRef, useState } from 'react';

interface SpeedDataPoint {
  up: number; // in bytes/sec
  down: number; // in bytes/sec
}

interface TrafficChartProps {
  history: SpeedDataPoint[];
}

interface ResolvedChartColors {
  textLow: string;
  borderDefault: string;
  borderStrong: string;
  chartDownStroke: string;
  chartDownGlow: string;
  chartDownFill0: string;
  chartDownFill1: string;
  chartUpStroke: string;
  chartUpGlow: string;
  chartUpFill0: string;
  chartUpFill1: string;
}

function resolveBrowserColor(cssValue: string, fallback: string): string {
  if (!cssValue) return fallback;
  const trimmed = cssValue.trim();
  if (!trimmed) return fallback;

  if (typeof document !== 'undefined') {
    try {
      const probe = document.createElement('div');
      probe.style.display = 'none';
      probe.style.color = trimmed;
      document.documentElement.appendChild(probe);
      const computed = getComputedStyle(probe).color;
      document.documentElement.removeChild(probe);
      if (computed && computed !== '' && !computed.includes('var(')) {
        return computed;
      }
    } catch {
      // fallback to safe value
    }
  }

  return trimmed.includes('var(') ? fallback : trimmed;
}

function readThemeColors(): ResolvedChartColors {
  if (typeof document === 'undefined') {
    return {
      textLow: '#71717a',
      borderDefault: 'rgba(255, 255, 255, 0.08)',
      borderStrong: 'rgba(255, 255, 255, 0.15)',
      chartDownStroke: '#ffffff',
      chartDownGlow: 'rgba(255, 255, 255, 0.35)',
      chartDownFill0: 'rgba(255, 255, 255, 0.15)',
      chartDownFill1: 'rgba(255, 255, 255, 0)',
      chartUpStroke: '#a1a1aa',
      chartUpGlow: 'rgba(161, 161, 170, 0.2)',
      chartUpFill0: 'rgba(161, 161, 170, 0.08)',
      chartUpFill1: 'rgba(161, 161, 170, 0)',
    };
  }

  const isDark = document.documentElement.classList.contains('dark') ||
    document.documentElement.getAttribute('data-theme') === 'dark';

  const defaultBorder = isDark ? 'rgba(255, 255, 255, 0.08)' : 'rgba(0, 0, 0, 0.08)';
  const defaultBorderStrong = isDark ? 'rgba(255, 255, 255, 0.15)' : 'rgba(0, 0, 0, 0.15)';
  const defaultTextLow = isDark ? '#71717a' : '#71717a';

  const defaultDownStroke = isDark ? '#ffffff' : '#18181b';
  const defaultDownGlow = isDark ? 'rgba(255, 255, 255, 0.35)' : 'rgba(0, 0, 0, 0.2)';
  const defaultDownFill0 = isDark ? 'rgba(255, 255, 255, 0.15)' : 'rgba(0, 0, 0, 0.08)';
  const defaultDownFill1 = 'rgba(0, 0, 0, 0)';

  const defaultUpStroke = isDark ? '#a1a1aa' : '#71717a';
  const defaultUpGlow = isDark ? 'rgba(161, 161, 170, 0.2)' : 'rgba(0, 0, 0, 0.12)';
  const defaultUpFill0 = isDark ? 'rgba(161, 161, 170, 0.08)' : 'rgba(0, 0, 0, 0.04)';
  const defaultUpFill1 = 'rgba(0, 0, 0, 0)';

  const rootStyle = getComputedStyle(document.documentElement);

  return {
    textLow: resolveBrowserColor(rootStyle.getPropertyValue('--text-low'), defaultTextLow),
    borderDefault: resolveBrowserColor(rootStyle.getPropertyValue('--border-default') || rootStyle.getPropertyValue('--border'), defaultBorder),
    borderStrong: resolveBrowserColor(rootStyle.getPropertyValue('--border-strong'), defaultBorderStrong),
    chartDownStroke: resolveBrowserColor(rootStyle.getPropertyValue('--chart-down-stroke') || rootStyle.getPropertyValue('--chart-download'), defaultDownStroke),
    chartDownGlow: resolveBrowserColor(rootStyle.getPropertyValue('--chart-down-glow'), defaultDownGlow),
    chartDownFill0: resolveBrowserColor(rootStyle.getPropertyValue('--chart-down-fill-0'), defaultDownFill0),
    chartDownFill1: resolveBrowserColor(rootStyle.getPropertyValue('--chart-down-fill-1'), defaultDownFill1),
    chartUpStroke: resolveBrowserColor(rootStyle.getPropertyValue('--chart-up-stroke') || rootStyle.getPropertyValue('--chart-upload'), defaultUpStroke),
    chartUpGlow: resolveBrowserColor(rootStyle.getPropertyValue('--chart-up-glow'), defaultUpGlow),
    chartUpFill0: resolveBrowserColor(rootStyle.getPropertyValue('--chart-up-fill-0'), defaultUpFill0),
    chartUpFill1: resolveBrowserColor(rootStyle.getPropertyValue('--chart-up-fill-1'), defaultUpFill1),
  };
}

export const TrafficChart: React.FC<TrafficChartProps> = ({ history }) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [dimensions, setDimensions] = useState({ width: 0, height: 0 });
  const [colors, setColors] = useState<ResolvedChartColors>(readThemeColors);

  // Helper to format speed to human readable format (e.g. 1.2 MB/s, 450 KB/s)
  const formatSpeed = (bytesPerSec: number): string => {
    if (bytesPerSec <= 0) return '0 B/s';
    const k = 1024;
    const sizes = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
    const i = Math.floor(Math.log(bytesPerSec) / Math.log(k));
    return parseFloat((bytesPerSec / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  };

  // Observe container size
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const resizeObserver = new ResizeObserver((entries) => {
      if (!entries || entries.length === 0) return;
      const { width, height } = entries[0].contentRect;
      setDimensions({ width, height });
    });

    resizeObserver.observe(container);
    return () => {
      resizeObserver.disconnect();
    };
  }, []);

  // Update theme colors when theme changes without re-reading per traffic sample
  useEffect(() => {
    const updateColors = () => {
      setColors(readThemeColors());
    };

    updateColors();

    const observer = new MutationObserver((mutations) => {
      for (const m of mutations) {
        if (m.type === 'attributes' && (m.attributeName === 'class' || m.attributeName === 'data-theme')) {
          updateColors();
          break;
        }
      }
    });

    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class', 'data-theme'],
    });

    const mql = window.matchMedia('(prefers-color-scheme: dark)');
    mql.addEventListener('change', updateColors);

    return () => {
      observer.disconnect();
      mql.removeEventListener('change', updateColors);
    };
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || dimensions.width === 0 || dimensions.height === 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Handle high DPI screens (retina)
    const dpr = window.devicePixelRatio || 1;
    canvas.width = dimensions.width * dpr;
    canvas.height = dimensions.height * dpr;
    ctx.scale(dpr, dpr);

    const width = dimensions.width;
    const height = dimensions.height;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    const {
      textLow,
      borderDefault,
      borderStrong,
      chartDownStroke,
      chartDownGlow,
      chartDownFill0,
      chartDownFill1,
      chartUpStroke,
      chartUpGlow,
      chartUpFill0,
      chartUpFill1,
    } = colors;

    // Calculate maximum speed in the history to scale the Y-axis
    let maxSpeed = 1024 * 100; // Minimum scale of 100 KB/s
    if (history.length > 0) {
      history.forEach((pt) => {
        if (pt.up > maxSpeed) maxSpeed = pt.up;
        if (pt.down > maxSpeed) maxSpeed = pt.down;
      });
    }
 
    // Add a 15% buffer at the top
    maxSpeed = maxSpeed * 1.15;
 
    // Measure Y-axis text width to dynamically scale padding.left
    ctx.font = '9px "Inter", sans-serif';
    const longestLabel = formatSpeed(maxSpeed);
    const labelWidth = ctx.measureText(longestLabel).width;
    const paddingLeft = Math.max(48, Math.ceil(labelWidth + 12));
 
    // Padding
    const padding = { top: 20, right: 15, bottom: 25, left: paddingLeft };
    const chartWidth = width - padding.left - padding.right;
    const chartHeight = height - padding.top - padding.bottom;
 
    // Draw background grid lines
    ctx.strokeStyle = borderDefault;
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
      ctx.fillStyle = textLow;
      ctx.font = '12px "Inter", sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText('No real-time traffic data', width / 2, height / 2);
      return;
    }
 
    // Draw Y axis labels
    ctx.fillStyle = textLow;
    ctx.font = '9px "Inter", sans-serif';
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
      glowColor: string,
      isDashed = false
    ) => {
      if (data.length < 2) return;

      ctx.save();

      // 1. Draw area gradient fill under curve
      ctx.beginPath();
      const firstPt = getCoordinates(0, data[0]);
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
      ctx.lineWidth = 2;
      ctx.lineCap = 'round';
      ctx.lineJoin = 'round';
      if (isDashed) {
        ctx.setLineDash([3, 3]);
      } else {
        ctx.setLineDash([]);
      }

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
    // Download
    const downGrad = ctx.createLinearGradient(0, padding.top, 0, padding.top + chartHeight);
    downGrad.addColorStop(0, chartDownFill0);
    downGrad.addColorStop(1, chartDownFill1);

    // Upload
    const upGrad = ctx.createLinearGradient(0, padding.top, 0, padding.top + chartHeight);
    upGrad.addColorStop(0, chartUpFill0);
    upGrad.addColorStop(1, chartUpFill1);

    // Draw Download Curve first (background)
    drawCurve(paddedDown, chartDownStroke, downGrad, chartDownGlow, false);

    // Draw Upload Curve second (foreground)
    drawCurve(paddedUp, chartUpStroke, upGrad, chartUpGlow, true);

    // Draw X-axis line (subtle)
    ctx.strokeStyle = borderStrong;
    ctx.beginPath();
    ctx.moveTo(padding.left, padding.top + chartHeight);
    ctx.lineTo(width - padding.right, padding.top + chartHeight);
    ctx.stroke();

    // Draw timeline markers (e.g. -30s, -15s, Now)
    ctx.fillStyle = textLow;
    ctx.font = '9px "Inter", sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'top';

    ctx.fillText('30s ago', padding.left, padding.top + chartHeight + 6);
    ctx.fillText('15s ago', padding.left + chartWidth / 2, padding.top + chartHeight + 6);
    ctx.textAlign = 'right';
    ctx.fillText('Live', width - padding.right, padding.top + chartHeight + 6);

  }, [history, dimensions, colors]);

  return (
    <div ref={containerRef} style={{ width: '100%', height: '100%', position: 'relative' }}>
      <canvas
        ref={canvasRef}
        role="img"
        aria-label="Real-time network traffic chart showing upload and download speeds"
        style={{
          width: '100%',
          height: '100%',
          display: 'block',
        }}
      />
    </div>
  );
};
