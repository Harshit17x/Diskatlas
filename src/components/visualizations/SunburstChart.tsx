import React, { useEffect, useRef, useState } from 'react';
import * as d3 from 'd3';
import type { HierarchicalNode } from '../../types/bindings';
import { formatBytes } from '../../utils/formatters';
import { Button } from '../common/Button';
import { RotateCcw, ArrowUp, FolderTree } from 'lucide-react';

interface SunburstProps {
  data: HierarchicalNode;
  onDrillDown: (path: string) => void;
  onNavigateUp: () => void;
  onReset: () => void;
  breadcrumbs: string[];
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

export const SunburstChart: React.FC<SunburstProps> = ({
  data,
  onDrillDown,
  onNavigateUp,
  onReset,
  breadcrumbs,
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
    depth: number;
    isDirectory: boolean;
  } | null>(null);

  const [dimensions, setDimensions] = useState<{ width: number; height: number }>({
    width: 700,
    height: 480,
  });

  useEffect(() => {
    if (!containerRef.current) return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const w = entry.contentRect.width;
        const h = Math.max(entry.contentRect.height, 480);
        if (w > 0) {
          setDimensions({ width: w, height: h });
        }
      }
    });
    observer.observe(containerRef.current);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (!svgRef.current || !data) return;

    const width = dimensions.width;
    const height = dimensions.height;
    const radius = Math.min(width, height) / 2 - 20;

    const svg = d3.select(svgRef.current);
    svg.selectAll('*').remove();

    svg
      .attr('viewBox', `0 0 ${width} ${height}`)
      .attr('width', width)
      .attr('height', height);

    const g = svg
      .append('g')
      .attr('transform', `translate(${width / 2},${height / 2})`);

    // Create D3 Hierarchy
    const root = d3
      .hierarchy<HierarchicalNode>(data)
      .sum((d) => (d.children && d.children.length > 0 ? 0 : d.value))
      .sort((a, b) => (b.value || 0) - (a.value || 0));

    // Partition Layout for Sunburst
    const partition = d3.partition<HierarchicalNode>().size([2 * Math.PI, radius]);
    partition(root);

    const totalValue = root.value || 1;

    // Arc Generator
    const arc = d3
      .arc<d3.HierarchyRectangularNode<HierarchicalNode>>()
      .startAngle((d) => d.x0)
      .endAngle((d) => d.x1)
      .padAngle((d) => Math.min((d.x1 - d.x0) / 2, 0.005))
      .padRadius(radius / 2)
      .innerRadius((d) => d.y0)
      .outerRadius((d) => Math.max(d.y0, d.y1 - 1));

    // Render Path Segments
    const rectangularDescendants = root
      .descendants()
      .filter((d) => d.depth <= 4) as unknown as d3.HierarchyRectangularNode<HierarchicalNode>[];

    g.selectAll<SVGPathElement, d3.HierarchyRectangularNode<HierarchicalNode>>('path')
      .data(rectangularDescendants)
      .enter()
      .append('path')
      .attr('d', (d) => arc(d))
      .attr('fill', (d) => {
        if (d.depth === 0) return '#15171B';
        const cat = d.data.category || 'unknown';
        return CATEGORY_COLORS[cat] || '#22262C';
      })
      .attr('fill-opacity', (d) => (d.depth === 0 ? 0.9 : 0.85 - (d.depth - 1) * 0.12))
      .attr('stroke', '#101114')
      .attr('stroke-width', 1)
      .attr('class', 'cursor-pointer hover:brightness-125 transition-all')
      .on('click', (_event, d) => {
        if (d.depth === 0) {
          onNavigateUp();
        } else if (d.data.isDirectory) {
          onDrillDown(d.data.path);
        }
      })
      .on('mouseenter', (event, d) => {
        const [mx, my] = d3.pointer(event, containerRef.current);
        const pct = ((d.value || d.data.value) / totalValue) * 100;
        setTooltip({
          visible: true,
          x: mx + 12,
          y: my + 12,
          name: d.data.name,
          path: d.data.path,
          size: d.value || d.data.value,
          category: d.data.category || 'unknown',
          percentage: pct,
          depth: d.depth,
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

    // Center Drive Icon & Label
    const centerGroup = g.append('g').attr('class', 'pointer-events-none text-center');
    centerGroup
      .append('text')
      .attr('text-anchor', 'middle')
      .attr('dy', '-0.3em')
      .attr('fill', '#F1F3F5')
      .attr('font-size', '11px')
      .attr('font-weight', '600')
      .attr('font-family', 'sans-serif')
      .text(data.name);

    centerGroup
      .append('text')
      .attr('text-anchor', 'middle')
      .attr('dy', '1.1em')
      .attr('fill', '#5687E8')
      .attr('font-size', '10px')
      .attr('font-family', 'monospace')
      .text(formatBytes(data.value));
  }, [data, dimensions, onDrillDown, onNavigateUp]);

  return (
    <div className="space-y-2">
      {/* 1. Header Toolbar */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 p-2 bg-[#131518] border border-border rounded-lg text-xs">
        <div className="flex items-center gap-1.5 font-mono text-[11px] text-secondary overflow-x-auto">
          <FolderTree className="w-3.5 h-3.5 text-accent shrink-0" />
          <span>Active Root:</span>
          <span className="text-primary font-semibold truncate max-w-sm" title={data.path}>
            {data.path}
          </span>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <Button
            size="sm"
            variant="outline"
            icon={<ArrowUp className="w-3 h-3" />}
            onClick={onNavigateUp}
            disabled={breadcrumbs.length <= 1}
          >
            Up Level
          </Button>
          <Button
            size="sm"
            variant="outline"
            icon={<RotateCcw className="w-3 h-3" />}
            onClick={onReset}
          >
            Reset
          </Button>
        </div>
      </div>

      {/* 2. D3 Sunburst Canvas */}
      <div
        ref={containerRef}
        className="relative w-full h-[480px] bg-[#111316] border border-border rounded-lg overflow-hidden flex items-center justify-center"
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
              <span>{tooltip.percentage.toFixed(1)}% of ring</span>
            </div>

            {tooltip.isDirectory && tooltip.depth > 0 && (
              <div className="text-[10px] text-accent font-medium italic pt-0.5">
                Click segment to drill down
              </div>
            )}
            {tooltip.depth === 0 && (
              <div className="text-[10px] text-muted italic pt-0.5">
                Click center to navigate up
              </div>
            )}
          </div>
        )}
      </div>

      {/* 3. Legend */}
      <div className="flex items-center gap-3 overflow-x-auto text-[11px] text-secondary px-1 py-0.5">
        <span className="text-muted font-medium text-[10px] uppercase">Rings:</span>
        <span className="text-muted">Center = Root • Outer Rings = Subdirectories & Files</span>
      </div>
    </div>
  );
};
