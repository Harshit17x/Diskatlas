import React, { useEffect, useRef, useState } from 'react';
import * as d3 from 'd3';
import type { HierarchicalNode } from '../../types/bindings';
import { formatBytes } from '../../utils/formatters';
import { Button } from '../common/Button';
import {
  RotateCcw,
  ArrowUp,
  ChevronRight,
  FolderTree,
} from 'lucide-react';

interface TreemapProps {
  data: HierarchicalNode;
  onDrillDown: (path: string) => void;
  onNavigateUp: () => void;
  onReset: () => void;
  breadcrumbs: string[];
  maxDepth: number;
  onSetMaxDepth: (depth: number) => void;
}

const CATEGORY_COLORS: Record<string, string> = {
  developer: '#E5A54B',
  os: '#5687E8',
  applications: '#8B5CF6',
  videos: '#EF4444',
  images: '#06B6D4',
  audio: '#10B981',
  documents: '#3B82F6',
  archives: '#F59E0B',
  temporary: '#DC2626',
  gaming: '#EC4899',
  vm_emulators: '#6366F1',
  drive_root: '#1F242C',
  unknown: '#323842',
};

export const TreemapChart: React.FC<TreemapProps> = ({
  data,
  onDrillDown,
  onNavigateUp,
  onReset,
  breadcrumbs,
  maxDepth,
  onSetMaxDepth,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  const [tooltip, setTooltip] = useState<{
    visible: boolean;
    x: number;
    y: number;
    name: string;
    path: string;
    size: number;
    category: string;
    percentage: number;
    isDirectory: boolean;
  } | null>(null);

  useEffect(() => {
    if (!svgRef.current || !containerRef.current || !data) return;

    const width = containerRef.current.clientWidth || 800;
    const height = Math.max(containerRef.current.clientHeight, 480);

    const svg = d3.select(svgRef.current);
    svg.selectAll('*').remove();

    svg.attr('viewBox', `0 0 ${width} ${height}`).attr('width', width).attr('height', height);

    // Create D3 Hierarchy
    const root = d3
      .hierarchy<HierarchicalNode>(data)
      .sum((d) => (d.children && d.children.length > 0 ? 0 : d.value))
      .sort((a, b) => (b.value || 0) - (a.value || 0));

    // Construct Treemap Layout with squarified padding
    const treemapLayout = d3
      .treemap<HierarchicalNode>()
      .size([width, height])
      .paddingOuter(4)
      .paddingTop(20)
      .paddingInner(2)
      .round(true);

    treemapLayout(root);

    const totalRootValue = root.value || 1;

    // Render Tile Groups
    const rectangularNodes = root
      .descendants()
      .filter((d) => d.depth > 0) as unknown as d3.HierarchyRectangularNode<HierarchicalNode>[];

    const nodes = svg
      .selectAll<SVGGElement, d3.HierarchyRectangularNode<HierarchicalNode>>('g')
      .data(rectangularNodes)
      .enter()
      .append('g')
      .attr('transform', (d) => `translate(${d.x0},${d.y0})`)
      .attr('class', 'cursor-pointer select-none transition-opacity')
      .on('click', (_event, d) => {
        if (d.data.isDirectory && d.data.path !== data.path) {
          onDrillDown(d.data.path);
        }
      })
      .on('mouseenter', (event, d) => {
        const [mx, my] = d3.pointer(event, containerRef.current);
        const pct = ((d.value || d.data.value) / totalRootValue) * 100;
        setTooltip({
          visible: true,
          x: mx + 12,
          y: my + 12,
          name: d.data.name,
          path: d.data.path,
          size: d.value || d.data.value,
          category: d.data.category || 'unknown',
          percentage: pct,
          isDirectory: d.data.isDirectory,
        });
      })
      .on('mousemove', (event) => {
        if (!containerRef.current) return;
        const [mx, my] = d3.pointer(event, containerRef.current);
        setTooltip((prev) => (prev ? { ...prev, x: mx + 12, y: my + 12 } : null));
      })
      .on('mouseleave', () => {
        setTooltip(null);
      });

    // Render Rectangles
    nodes
      .append('rect')
      .attr('width', (d) => Math.max(0, d.x1 - d.x0))
      .attr('height', (d) => Math.max(0, d.y1 - d.y0))
      .attr('rx', 3)
      .attr('fill', (d) => {
        const cat = d.data.category || 'unknown';
        return CATEGORY_COLORS[cat] || '#22262C';
      })
      .attr('fill-opacity', (d) => (d.data.isDirectory ? 0.35 : 0.85))
      .attr('stroke', '#101114')
      .attr('stroke-width', 1.5)
      .attr('class', 'hover:brightness-125 transition-all');

    // Render Text Labels for sufficiently large boxes
    nodes
      .append('text')
      .filter((d) => d.x1 - d.x0 > 45 && d.y1 - d.y0 > 24)
      .attr('x', 5)
      .attr('y', 14)
      .attr('fill', '#F1F3F5')
      .attr('font-size', '10px')
      .attr('font-weight', '500')
      .attr('font-family', 'sans-serif')
      .text((d) => d.data.name)
      .each(function (d) {
        // Truncate text if it overflows the rectangle width
        const self = d3.select(this);
        const boxWidth = d.x1 - d.x0 - 8;
        let textLength = (this as SVGTextElement).getComputedTextLength();
        let text = d.data.name;
        while (textLength > boxWidth && text.length > 3) {
          text = text.slice(0, -2);
          self.text(text + '…');
          textLength = (this as SVGTextElement).getComputedTextLength();
        }
      });

    // Sub-label with formatted size
    nodes
      .append('text')
      .filter((d) => d.x1 - d.x0 > 55 && d.y1 - d.y0 > 38)
      .attr('x', 5)
      .attr('y', 27)
      .attr('fill', '#9BA3AE')
      .attr('font-size', '9px')
      .attr('font-mono', 'true')
      .text((d) => formatBytes(d.value || d.data.value));
  }, [data, onDrillDown]);

  return (
    <div className="space-y-2">
      {/* 1. Chart Toolbar & Breadcrumbs */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 p-2 bg-[#131518] border border-border rounded-lg text-xs">
        {/* Clickable Breadcrumbs */}
        <div className="flex items-center gap-1 overflow-x-auto py-0.5 text-secondary font-mono text-[11px] max-w-xl">
          <button
            onClick={onReset}
            className="hover:text-primary transition-colors flex items-center gap-1 shrink-0 font-sans"
          >
            <FolderTree className="w-3.5 h-3.5 text-accent" />
            <span>Root</span>
          </button>

          {breadcrumbs.map((crumb, idx) => {
            const isLast = idx === breadcrumbs.length - 1;
            const label = crumb.split('/').filter(Boolean).pop() || crumb;
            return (
              <React.Fragment key={crumb}>
                <ChevronRight className="w-3 h-3 text-muted shrink-0" />
                <button
                  onClick={() => !isLast && onDrillDown(crumb)}
                  disabled={isLast}
                  className={`shrink-0 truncate max-w-[140px] transition-colors ${
                    isLast ? 'text-primary font-semibold' : 'hover:text-primary'
                  }`}
                  title={crumb}
                >
                  {label}
                </button>
              </React.Fragment>
            );
          })}
        </div>

        {/* Zoom & Depth Controls */}
        <div className="flex items-center gap-2 shrink-0">
          <div className="flex items-center gap-1 text-[11px] text-muted">
            <span>Depth:</span>
            {[2, 3, 4].map((d) => (
              <button
                key={d}
                onClick={() => onSetMaxDepth(d)}
                className={`px-1.5 py-0.5 rounded text-[10px] font-mono transition-colors ${
                  maxDepth === d
                    ? 'bg-accent text-white font-semibold'
                    : 'bg-elevated text-secondary hover:text-primary hover:bg-[#282D35]'
                }`}
              >
                {d}L
              </button>
            ))}
          </div>

          <Button
            size="sm"
            variant="outline"
            icon={<ArrowUp className="w-3 h-3" />}
            onClick={onNavigateUp}
            disabled={breadcrumbs.length <= 1}
            title="Navigate up one folder level"
          >
            Up
          </Button>

          <Button
            size="sm"
            variant="outline"
            icon={<RotateCcw className="w-3 h-3" />}
            onClick={onReset}
            title="Reset treemap view to disk root"
          >
            Reset
          </Button>
        </div>
      </div>

      {/* 2. D3 Treemap SVG Canvas */}
      <div
        ref={containerRef}
        className="relative w-full h-[480px] bg-[#111316] border border-border rounded-lg overflow-hidden"
      >
        <svg ref={svgRef} className="w-full h-full block" />

        {/* Hover Tooltip Overlay */}
        {tooltip && tooltip.visible && (
          <div
            className="absolute pointer-events-none z-50 bg-[#16181D] border border-border shadow-xl rounded px-3 py-2 text-xs space-y-1 font-sans transition-all"
            style={{
              left: Math.min(tooltip.x, (containerRef.current?.clientWidth || 600) - 220),
              top: Math.min(tooltip.y, (containerRef.current?.clientHeight || 400) - 100),
            }}
          >
            <div className="flex items-center justify-between gap-3">
              <span className="font-semibold text-primary truncate max-w-[160px]">
                {tooltip.name}
              </span>
              <span className="font-mono text-accent font-semibold">
                {formatBytes(tooltip.size)}
              </span>
            </div>

            <div className="text-[11px] text-secondary font-mono truncate max-w-[200px]">
              {tooltip.path}
            </div>

            <div className="flex items-center justify-between text-[10px] pt-1 border-t border-border/60 text-muted font-mono">
              <span className="capitalize">{tooltip.category}</span>
              <span>{tooltip.percentage.toFixed(1)}% of view</span>
            </div>

            {tooltip.isDirectory && (
              <div className="text-[10px] text-accent font-medium italic pt-0.5">
                Click to drill down into folder
              </div>
            )}
          </div>
        )}
      </div>

      {/* 3. Category Legend Bar */}
      <div className="flex items-center gap-3 overflow-x-auto text-[11px] text-secondary px-1 py-0.5">
        <span className="text-muted font-medium text-[10px] uppercase">Legend:</span>
        {Object.entries(CATEGORY_COLORS)
          .filter(([key]) => key !== 'drive_root')
          .map(([category, color]) => (
            <div key={category} className="flex items-center gap-1.5 shrink-0">
              <span className="w-2.5 h-2.5 rounded-sm" style={{ backgroundColor: color }} />
              <span className="capitalize text-muted">{category.replace('_', ' ')}</span>
            </div>
          ))}
      </div>
    </div>
  );
};
