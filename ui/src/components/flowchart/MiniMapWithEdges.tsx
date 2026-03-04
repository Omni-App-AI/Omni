import { useMemo, useCallback, useRef } from "react";
import {
  Panel,
  useStore,
  useReactFlow,
  type Node,
  type ReactFlowState,
} from "@xyflow/react";

// ─── Store selectors (stable references to avoid re-render churn) ───

const nodesSelector = (s: ReactFlowState) => s.nodes;
const edgesSelector = (s: ReactFlowState) => s.edges;
const transformSelector = (s: ReactFlowState) => s.transform; // [x, y, zoom]
const paneSizeSelector = (s: ReactFlowState) => ({ w: s.width, h: s.height });

// ─── Props ──────────────────────────────────────────────────────────

interface MiniMapWithEdgesProps {
  width?: number;
  height?: number;
  nodeColor: (node: Node) => string;
  edgeColor?: string;
  maskColor?: string;
}

// ─── Component ──────────────────────────────────────────────────────

export function MiniMapWithEdges({
  width = 200,
  height = 150,
  nodeColor,
  edgeColor = "var(--text-muted)",
  maskColor = "rgba(0, 0, 0, 0.6)",
}: MiniMapWithEdgesProps) {
  const nodes = useStore(nodesSelector);
  const edges = useStore(edgesSelector);
  const transform = useStore(transformSelector);
  const paneSize = useStore(paneSizeSelector);
  const { setViewport } = useReactFlow();
  const svgRef = useRef<SVGSVGElement>(null);

  // Build lookup: node id → { x, y, w, h }
  const nodeLookup = useMemo(() => {
    const map = new Map<string, { x: number; y: number; w: number; h: number }>();
    for (const node of nodes) {
      const w = node.measured?.width ?? 150;
      const h = node.measured?.height ?? 40;
      map.set(node.id, { x: node.position.x, y: node.position.y, w, h });
    }
    return map;
  }, [nodes]);

  // Compute bounding box of all nodes with padding
  const bounds = useMemo(() => {
    if (nodes.length === 0) return { x: 0, y: 0, w: 400, h: 300 };

    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const node of nodes) {
      const nw = node.measured?.width ?? 150;
      const nh = node.measured?.height ?? 40;
      minX = Math.min(minX, node.position.x);
      minY = Math.min(minY, node.position.y);
      maxX = Math.max(maxX, node.position.x + nw);
      maxY = Math.max(maxY, node.position.y + nh);
    }
    const pad = 60;
    return {
      x: minX - pad,
      y: minY - pad,
      w: maxX - minX + pad * 2,
      h: maxY - minY + pad * 2,
    };
  }, [nodes]);

  // Viewport rectangle in flow coordinates
  const vpRect = useMemo(() => {
    const [tx, ty, zoom] = transform;
    return {
      x: -tx / zoom,
      y: -ty / zoom,
      w: paneSize.w / zoom,
      h: paneSize.h / zoom,
    };
  }, [transform, paneSize]);

  // Edge paths: straight lines from source bottom-center to target top-center
  const edgePaths = useMemo(() => {
    const paths: Array<{ id: string; d: string }> = [];
    for (const edge of edges) {
      const src = nodeLookup.get(edge.source);
      const tgt = nodeLookup.get(edge.target);
      if (!src || !tgt) continue;

      const sx = src.x + src.w / 2;
      const sy = src.y + src.h;
      const tx = tgt.x + tgt.w / 2;
      const ty = tgt.y;

      paths.push({ id: edge.id, d: `M${sx},${sy} L${tx},${ty}` });
    }
    return paths;
  }, [edges, nodeLookup]);

  // Click-to-navigate: center viewport on clicked point
  const handleClick = useCallback(
    (e: React.MouseEvent<SVGSVGElement>) => {
      const svg = svgRef.current;
      if (!svg) return;

      const rect = svg.getBoundingClientRect();
      // Convert screen position to SVG/flow coordinates
      const ratioX = (e.clientX - rect.left) / rect.width;
      const ratioY = (e.clientY - rect.top) / rect.height;
      const flowX = bounds.x + ratioX * bounds.w;
      const flowY = bounds.y + ratioY * bounds.h;

      // Center on the clicked point using current zoom
      const [, , zoom] = transform;
      setViewport(
        {
          x: -flowX * zoom + paneSize.w / 2,
          y: -flowY * zoom + paneSize.h / 2,
          zoom,
        },
        { duration: 200 },
      );
    },
    [bounds, transform, paneSize, setViewport],
  );

  const viewBox = `${bounds.x} ${bounds.y} ${bounds.w} ${bounds.h}`;

  return (
    <Panel position="bottom-right">
      <svg
        ref={svgRef}
        width={width}
        height={height}
        viewBox={viewBox}
        onClick={handleClick}
        style={{
          background: "var(--bg-secondary)",
          border: "1px solid var(--border)",
          borderRadius: 4,
          cursor: "pointer",
        }}
      >
        {/* Edges (rendered behind nodes) */}
        <g>
          {edgePaths.map((ep) => (
            <path
              key={ep.id}
              d={ep.d}
              fill="none"
              stroke={edgeColor}
              strokeWidth={Math.max(1, bounds.w / 300)}
              opacity={0.5}
            />
          ))}
        </g>

        {/* Nodes */}
        <g>
          {nodes.map((node) => {
            const info = nodeLookup.get(node.id);
            if (!info) return null;
            const c = nodeColor(node);
            return (
              <rect
                key={node.id}
                x={info.x}
                y={info.y}
                width={info.w}
                height={info.h}
                rx={4}
                fill={c}
                stroke={c}
                strokeWidth={2}
              />
            );
          })}
        </g>

        {/* Viewport mask — dark overlay with cutout for visible area */}
        <path
          d={`M${bounds.x},${bounds.y}h${bounds.w}v${bounds.h}h${-bounds.w}z M${vpRect.x},${vpRect.y}h${vpRect.w}v${vpRect.h}h${-vpRect.w}z`}
          fill={maskColor}
          fillRule="evenodd"
          pointerEvents="none"
        />

        {/* Viewport border */}
        <rect
          x={vpRect.x}
          y={vpRect.y}
          width={vpRect.w}
          height={vpRect.h}
          fill="none"
          stroke="var(--accent)"
          strokeWidth={Math.max(1, bounds.w / 200)}
          pointerEvents="none"
        />
      </svg>
    </Panel>
  );
}
