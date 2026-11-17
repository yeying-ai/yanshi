import React, { useMemo, useState } from 'react';
import {
  Background,
  BackgroundVariant,
  Controls,
  Handle,
  MarkerType,
  MiniMap,
  Panel,
  Position,
  ReactFlow,
  type Connection,
  type Edge,
  type Node,
  type NodeProps,
  type NodeTypes,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import type { InspectedEdge, InspectedNode } from '../schemas';
import {
  NODE_PALETTE,
  NODE_PALETTE_CATEGORIES,
  type NodePaletteCategory,
} from '../studio/yamlGraphMutator';

export type StudioNodeRunStatus =
  | 'idle'
  | 'running'
  | 'paused_before'
  | 'paused_after'
  | 'succeeded'
  | 'skipped'
  | 'failed';

export interface NodeRuntimeTelemetry {
  status: StudioNodeRunStatus;
  durationMs?: number;
  costUsd?: number;
  hasInput?: boolean;
  hasOutput?: boolean;
  error?: string;
  skipReason?: string;
}

export interface StudioDagNodeData extends Record<string, unknown> {
  node: InspectedNode;
  telemetry: NodeRuntimeTelemetry;
  hasBreakpoint: boolean;
  isSelected: boolean;
  editable: boolean;
  stepOrderIndex: number;
  onToggleBreakpoint: (nodeId: string) => void;
  onSelectNode: (nodeId: string) => void;
}

export type StudioFlowNode = Node<StudioDagNodeData, 'studioNode'>;

function nodeTypePalette(nodeType: string): { label: string; accent: string; tint: string } {
  const normalized = nodeType.toLowerCase();
  if (normalized === 'llm' || normalized.includes('agent')) {
    return { label: nodeType, accent: '#6d4aff', tint: '#f3effe' };
  }
  if (normalized === 'render_slides' || normalized === 'render_study') {
    return { label: nodeType, accent: '#0b7a8f', tint: '#e8f7fa' };
  }
  if (normalized === 'tts' || normalized === 'render_audio' || normalized === 'asr') {
    return { label: nodeType, accent: '#0d8a6a', tint: '#e7f8f2' };
  }
  if (normalized === 'map' || normalized === 'while') {
    return { label: `${nodeType} 并发子流`, accent: '#0f766e', tint: '#ecfdf5' };
  }
  if (normalized === 'render_video' || normalized === 'text_to_video') {
    return { label: nodeType, accent: '#c84b24', tint: '#fff1ec' };
  }
  if (
    normalized === 'illustration' ||
    normalized === 'text_to_image' ||
    normalized === 'image_gen' ||
    normalized === 'image_edit'
  ) {
    return { label: nodeType, accent: '#be185d', tint: '#fdf2f8' };
  }
  if (
    normalized === 'render_document' ||
    normalized === 'render_markdown' ||
    normalized === 'render_mindmap' ||
    normalized === 'render_table' ||
    normalized === 'render_svg' ||
    normalized === 'figure' ||
    normalized === 'markmap'
  ) {
    return { label: nodeType, accent: '#3b5bdb', tint: '#edf2ff' };
  }
  if (
    normalized === 'web_search' ||
    normalized === 'repo_snapshot' ||
    normalized === 'pdf_extract' ||
    normalized === 'video_ingest' ||
    normalized === 'arxiv'
  ) {
    return { label: nodeType, accent: '#b45309', tint: '#fef3c7' };
  }
  return { label: nodeType, accent: '#475569', tint: '#f1f5f9' };
}

const STATUS_LABELS: Record<StudioNodeRunStatus, string> = {
  idle: '待命',
  running: '运行中…',
  paused_before: '⏸ 执行前断点',
  paused_after: '⏸ 单步暂停 · 待核查',
  succeeded: '✓ 已完成',
  skipped: '↷ 条件跳过',
  failed: '✕ 失败',
};

function formatUsdBadge(usd: number): string {
  if (usd < 0.001 && usd > 0) return '<$0.001';
  return `$${usd.toFixed(usd < 0.1 ? 4 : 3)}`;
}

function StudioDagNodeComponent({ data }: NodeProps<StudioFlowNode>): React.ReactElement {
  const {
    node,
    telemetry,
    hasBreakpoint,
    isSelected,
    editable,
    stepOrderIndex,
    onToggleBreakpoint,
    onSelectNode,
  } = data;
  const palette = nodeTypePalette(node.type);
  const subCount = node.sub_nodes?.length ?? 0;

  return (
    <div
      className={[
        'studio-dag-node',
        `studio-dag-node--${telemetry.status}`,
        isSelected ? 'studio-dag-node--selected' : '',
        editable ? 'studio-dag-node--editable' : '',
      ]
        .filter(Boolean)
        .join(' ')}
      onClick={() => onSelectNode(node.id)}
      role="button"
      tabIndex={0}
      onKeyDown={(ev) => {
        if (ev.key === 'Enter' || ev.key === ' ') {
          ev.preventDefault();
          onSelectNode(node.id);
        }
      }}
      style={{ '--node-accent': palette.accent, '--node-tint': palette.tint } as React.CSSProperties}
    >
      <Handle
        type="target"
        position={Position.Left}
        className={`studio-dag-handle ${editable ? 'studio-dag-handle--interactive' : ''}`}
      />

      <div className="studio-dag-node__top">
        <span className="studio-dag-node__step">#{stepOrderIndex + 1}</span>
        <span className="studio-dag-node__type">{palette.label}</span>
        <button
          type="button"
          className={`studio-bp-btn ${hasBreakpoint ? 'studio-bp-btn--active' : ''}`}
          aria-label={hasBreakpoint ? `取消节点 ${node.id} 断点` : `在节点 ${node.id} 设置执行前断点`}
          aria-pressed={hasBreakpoint}
          title={hasBreakpoint ? '已设执行前断点（点击移除）' : '点击设置执行前断点'}
          onClick={(ev) => {
            ev.stopPropagation();
            onToggleBreakpoint(node.id);
          }}
        >
          <span className="studio-bp-dot" />
        </button>
      </div>

      <div className="studio-dag-node__title-row">
        <strong className="studio-dag-node__id">{node.id}</strong>
      </div>

      {subCount > 0 && (
        <div className="studio-dag-node__chip">
          内嵌 {subCount} 个子节点：{node.sub_nodes?.map((s) => `${s.id} (${s.type})`).join(', ')}
        </div>
      )}

      {node.run_if && (
        <div className="studio-dag-node__chip studio-dag-node__chip--cond" title={node.run_if}>
          run_if 条件守卫
        </div>
      )}

      <div className="studio-dag-node__footer">
        <span className={`studio-node-status studio-node-status--${telemetry.status}`}>
          {STATUS_LABELS[telemetry.status]}
        </span>
        <span className="studio-dag-node__metrics">
          {typeof telemetry.costUsd === 'number' && telemetry.costUsd > 0 && (
            <b className="studio-cost-pill" title={`节点累计模型费用 $${telemetry.costUsd.toFixed(6)}`}>
              {formatUsdBadge(telemetry.costUsd)}
            </b>
          )}
          {typeof telemetry.durationMs === 'number' && <em>{telemetry.durationMs} ms</em>}
          {(telemetry.hasInput || telemetry.hasOutput) && (
            <b className="studio-io-pill" title="已捕获节点输入/输出快照">
              I/O
            </b>
          )}
        </span>
      </div>

      <Handle
        type="source"
        position={Position.Right}
        className={`studio-dag-handle ${editable ? 'studio-dag-handle--interactive' : ''}`}
      />
    </div>
  );
}

const NODE_TYPES: NodeTypes = {
  studioNode: StudioDagNodeComponent,
};

export interface WorkflowGraphCanvasProps {
  nodes: InspectedNode[];
  edges: InspectedEdge[];
  levels: string[][];
  executionOrder: string[];
  telemetryByNode: Record<string, NodeRuntimeTelemetry>;
  breakpoints: string[];
  selectedNodeId: string | null;
  editable?: boolean;
  onSelectNode: (nodeId: string) => void;
  onToggleBreakpoint: (nodeId: string) => void;
  onAddNode?: (nodeType: string, afterNodeId?: string) => void;
  onConnectNodes?: (sourceId: string, targetId: string) => void;
  onDisconnectEdge?: (sourceId: string, targetId: string) => void;
  onDeleteNode?: (nodeId: string) => void;
}

export function WorkflowGraphCanvas({
  nodes,
  edges,
  levels,
  executionOrder,
  telemetryByNode,
  breakpoints,
  selectedNodeId,
  editable = false,
  onSelectNode,
  onToggleBreakpoint,
  onAddNode,
  onConnectNodes,
  onDisconnectEdge,
  onDeleteNode,
}: WorkflowGraphCanvasProps): React.ReactElement {
  const [paletteOpen, setPaletteOpen] = useState<boolean>(false);
  const [paletteCategory, setPaletteCategory] = useState<NodePaletteCategory>('LLM / Agent');
  const [chainAfterSelected, setChainAfterSelected] = useState<boolean>(true);
  const [selectedEdgePair, setSelectedEdgePair] = useState<{ from: string; to: string } | null>(
    null,
  );

  const bpSet = useMemo(() => new Set(breakpoints), [breakpoints]);

  const orderIndexMap = useMemo(() => {
    const map = new Map<string, number>();
    executionOrder.forEach((id, idx) => map.set(id, idx));
    return map;
  }, [executionOrder]);

  const flowNodes = useMemo<StudioFlowNode[]>(() => {
    const colWidth = 304;
    const rowHeight = 166;
    const maxRowsInAnyCol = Math.max(1, ...levels.map((col) => col.length));
    const positionById = new Map<string, { x: number; y: number }>();

    if (levels.length > 0) {
      levels.forEach((colIds, colIdx) => {
        const colSpanHeight = (colIds.length - 1) * rowHeight;
        const maxSpanHeight = (maxRowsInAnyCol - 1) * rowHeight;
        const offsetY = (maxSpanHeight - colSpanHeight) / 2 + 36;
        colIds.forEach((id, rowIdx) => {
          positionById.set(id, {
            x: colIdx * colWidth + 36,
            y: offsetY + rowIdx * rowHeight,
          });
        });
      });
    }

    return nodes.map((node, fallbackIdx) => {
      const pos = positionById.get(node.id) ?? {
        x: (fallbackIdx % 4) * colWidth + 36,
        y: Math.floor(fallbackIdx / 4) * rowHeight + 36,
      };
      const telemetry: NodeRuntimeTelemetry = telemetryByNode[node.id] ?? { status: 'idle' };
      return {
        id: node.id,
        type: 'studioNode',
        position: pos,
        selected: selectedNodeId === node.id,
        data: {
          node,
          telemetry,
          hasBreakpoint: bpSet.has(node.id),
          isSelected: selectedNodeId === node.id,
          editable,
          stepOrderIndex: orderIndexMap.get(node.id) ?? fallbackIdx,
          onToggleBreakpoint,
          onSelectNode: (nid: string) => {
            setSelectedEdgePair(null);
            onSelectNode(nid);
          },
        },
      };
    });
  }, [
    nodes,
    levels,
    telemetryByNode,
    bpSet,
    selectedNodeId,
    editable,
    orderIndexMap,
    onToggleBreakpoint,
    onSelectNode,
  ]);

  const flowEdges = useMemo<Edge[]>(() => {
    return edges.map((edge, idx) => {
      const sourceStatus = telemetryByNode[edge.from]?.status ?? 'idle';
      const targetStatus = telemetryByNode[edge.to]?.status ?? 'idle';
      const isCond = edge.kind === 'condition';
      const isEdgeSelected =
        selectedEdgePair?.from === edge.from && selectedEdgePair?.to === edge.to;
      const isLive =
        targetStatus === 'running' ||
        targetStatus === 'paused_before' ||
        sourceStatus === 'paused_after';
      const stroke = isEdgeSelected
        ? '#dc2626'
        : isCond
          ? '#d97706'
          : sourceStatus === 'succeeded'
            ? '#0d8a6a'
            : isLive
              ? '#2563eb'
              : '#94a3b8';

      return {
        id: `e-${edge.from}-${edge.to}-${edge.kind}-${idx}`,
        source: edge.from,
        target: edge.to,
        type: 'smoothstep',
        animated: isLive,
        selected: isEdgeSelected,
        label: edge.label ?? undefined,
        labelStyle: {
          fill: isCond ? '#92400e' : '#334155',
          fontSize: 11,
          fontWeight: 600,
        },
        labelBgStyle: {
          fill: isCond ? '#fef3c7' : '#f8fafc',
          fillOpacity: 0.92,
        },
        labelBgPadding: [5, 3],
        labelBgBorderRadius: 4,
        style: {
          stroke,
          strokeWidth: isEdgeSelected ? 2.8 : isLive || sourceStatus === 'succeeded' ? 2.2 : 1.7,
          strokeDasharray: isCond ? '6 4' : undefined,
        },
        markerEnd: {
          type: MarkerType.ArrowClosed,
          color: stroke,
          width: 16,
          height: 16,
        },
      };
    });
  }, [edges, telemetryByNode, selectedEdgePair]);

  const filteredPalette = useMemo(
    () => NODE_PALETTE.filter((item) => item.category === paletteCategory),
    [paletteCategory],
  );

  const handleConnect = (conn: Connection) => {
    if (!editable || !onConnectNodes) return;
    if (conn.source && conn.target) {
      onConnectNodes(conn.source, conn.target);
    }
  };

  const handlePaletteAdd = (nodeType: string) => {
    if (!onAddNode) return;
    const afterId = chainAfterSelected && selectedNodeId ? selectedNodeId : undefined;
    onAddNode(nodeType, afterId);
    setPaletteOpen(false);
  };

  if (nodes.length === 0) {
    return (
      <div className="studio-canvas-empty">
        <p>当前 YAML 尚未解析出节点，请在左侧选择预设模板、修复语法错误，或直接添加首个节点。</p>
        {editable && onAddNode && (
          <div className="studio-empty-add-row">
            <button
              type="button"
              className="studio-btn-primary studio-btn-sm"
              onClick={() => onAddNode('llm')}
            >
              + 添加 LLM 节点起步
            </button>
            <button
              type="button"
              className="studio-btn-secondary studio-btn-sm"
              onClick={() => onAddNode('template')}
            >
              + 添加 Template 节点起步
            </button>
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="studio-canvas-wrap" data-testid="workflow-graph-canvas">
      <ReactFlow
        nodes={flowNodes}
        edges={flowEdges}
        nodeTypes={NODE_TYPES}
        fitView
        fitViewOptions={{ padding: 0.18, maxZoom: 1.15 }}
        minZoom={0.25}
        maxZoom={1.8}
        nodesConnectable={editable && Boolean(onConnectNodes)}
        onConnect={handleConnect}
        onEdgeClick={(_ev, edge) => {
          if (editable) {
            setSelectedEdgePair({ from: edge.source, to: edge.target });
          }
        }}
        onPaneClick={() => setSelectedEdgePair(null)}
        elementsSelectable
        proOptions={{ hideAttribution: true }}
      >
        <Background variant={BackgroundVariant.Dots} gap={20} size={1.2} color="#cbd5e1" />
        <Controls showInteractive={false} position="bottom-left" />
        <MiniMap
          pannable
          zoomable
          position="bottom-right"
          nodeColor={(n) => {
            const st = (n.data as StudioDagNodeData | undefined)?.telemetry?.status ?? 'idle';
            if (st === 'paused_after' || st === 'paused_before') return '#f59e0b';
            if (st === 'running') return '#2563eb';
            if (st === 'succeeded') return '#10b981';
            if (st === 'failed') return '#ef4444';
            if (st === 'skipped') return '#94a3b8';
            return '#cbd5e1';
          }}
        />
        <Panel position="top-left" className="studio-canvas-legend">
          <span>
            <i className="legend-line legend-line--data" /> 数据映射边
          </span>
          <span>
            <i className="legend-line legend-line--cond" /> run_if 条件边
          </span>
          <span>
            <i className="legend-bp" /> {editable ? '拖拽节点右端口连线 · 点右上圆点设断点' : '点击节点右上圆点设执行前断点'}
          </span>
        </Panel>

        {editable && (
          <Panel position="top-right" className="studio-canvas-builder-panel">
            <div className="studio-canvas-builder-bar">
              {selectedEdgePair && onDisconnectEdge && (
                <button
                  type="button"
                  data-testid="canvas-disconnect-edge-btn"
                  className="studio-btn-danger-ghost studio-btn-sm"
                  onClick={() => {
                    onDisconnectEdge(selectedEdgePair.from, selectedEdgePair.to);
                    setSelectedEdgePair(null);
                  }}
                >
                  ✂ 断开 {selectedEdgePair.from} → {selectedEdgePair.to}
                </button>
              )}

              {selectedNodeId && onDeleteNode && nodes.length > 1 && (
                <button
                  type="button"
                  data-testid="canvas-delete-node-btn"
                  className="studio-btn-ghost studio-btn-sm"
                  title={`删除当前选中节点 #${selectedNodeId} 并自动清理下游引用`}
                  onClick={() => onDeleteNode(selectedNodeId)}
                >
                  🗑 删除 #{selectedNodeId}
                </button>
              )}

              {onAddNode && (
                <button
                  type="button"
                  data-testid="canvas-open-palette-btn"
                  className="studio-btn-primary studio-btn-sm"
                  aria-expanded={paletteOpen}
                  onClick={() => setPaletteOpen((prev) => !prev)}
                >
                  {paletteOpen ? '✕ 收起组件库' : '+ 添加节点'}
                </button>
              )}
            </div>

            {paletteOpen && onAddNode && (
              <div
                className="studio-node-palette-drawer"
                role="region"
                aria-label="可视化添加工作流节点"
              >
                <div className="studio-node-palette__cats">
                  {NODE_PALETTE_CATEGORIES.map((cat) => (
                    <button
                      key={cat}
                      type="button"
                      className={`studio-cat-pill ${paletteCategory === cat ? 'active' : ''}`}
                      onClick={() => setPaletteCategory(cat)}
                    >
                      {cat}
                    </button>
                  ))}
                </div>

                {selectedNodeId && (
                  <label className="studio-palette-chain-toggle">
                    <input
                      type="checkbox"
                      checked={chainAfterSelected}
                      onChange={(ev) => setChainAfterSelected(ev.target.checked)}
                    />
                    <span>自动连接在当前选中节点 #{selectedNodeId} 之后</span>
                  </label>
                )}

                <div className="studio-node-palette__list">
                  {filteredPalette.map((item) => (
                    <button
                      key={item.type}
                      type="button"
                      data-testid={`palette-add-${item.type}`}
                      className="studio-palette-item"
                      onClick={() => handlePaletteAdd(item.type)}
                    >
                      <strong>{item.label}</strong>
                      <small>{item.description}</small>
                    </button>
                  ))}
                </div>
              </div>
            )}
          </Panel>
        )}
      </ReactFlow>
    </div>
  );
}
