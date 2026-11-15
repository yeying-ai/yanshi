import React, { useEffect, useMemo, useState } from 'react';
import { prettyJson } from '../lib/helpers';
import type { FlowDebugPause, InspectedNode } from '../schemas';
import { extractNodeMediaPreviews } from '../studio/mediaExtractor';
import type { NodeUpdatePatch } from '../studio/yamlGraphMutator';
import { AuthorizedMedia } from './AuthorizedMedia';
import { NodeConfigEditor } from './NodeConfigEditor';
import type { NodeRuntimeTelemetry } from './WorkflowGraphCanvas';

type InspectorTab = 'io' | 'media' | 'config';
type BeforeOverrideKind = 'input' | 'mock_output';

export interface DebugResumeOverrides {
  input_override?: Record<string, unknown>;
  output_override?: Record<string, unknown>;
}

/**
 * Unwraps `{ type: "json", value: ... }` envelopes into plain JSON values so
 * editing captured inputs/outputs in the debug override editor stays ergonomic,
 * while keeping `{ type: "file", ... }` and `{ type: "url", ... }` intact.
 */
export function unwrapCapturedFlowJson(raw: unknown): Record<string, unknown> {
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) {
    return {};
  }
  const out: Record<string, unknown> = {};
  for (const [key, val] of Object.entries(raw as Record<string, unknown>)) {
    if (val && typeof val === 'object' && !Array.isArray(val)) {
      const rec = val as Record<string, unknown>;
      if (rec.type === 'json' && 'value' in rec) {
        out[key] = rec.value;
        continue;
      }
    }
    out[key] = val;
  }
  return out;
}

export function parseDebugOverrideObject(rawText: string): {
  value: Record<string, unknown> | null;
  error: string | null;
} {
  const trimmed = rawText.trim();
  if (!trimmed) {
    return { value: null, error: '覆盖载荷不能为空，请输入合法 JSON 对象' };
  }
  try {
    const parsed = JSON.parse(trimmed) as unknown;
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
      return { value: null, error: '覆盖载荷顶层必须是 JSON 对象（如 {"output": "..."}）' };
    }
    return { value: parsed as Record<string, unknown>, error: null };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return { value: null, error: `JSON 解析失败：${msg}` };
  }
}

export interface NodeIoInspectorProps {
  runId: string;
  apiToken: string;
  tenantId: string;
  node: InspectedNode | null;
  allNodes?: InspectedNode[];
  editable?: boolean;
  telemetry?: NodeRuntimeTelemetry;
  capturedInput: unknown;
  capturedOutput: unknown;
  pausedAt: FlowDebugPause | null;
  hasBreakpoint: boolean;
  onToggleBreakpoint: (nodeId: string) => void;
  onStepDebug: (overrides?: DebugResumeOverrides) => void;
  onContinueDebug: (overrides?: DebugResumeOverrides) => void;
  debugActionBusy: boolean;
  onApplyNodePatch?: (nodeId: string, patch: NodeUpdatePatch) => string | null;
  onDeleteNode?: (nodeId: string) => void;
}

export function NodeIoInspector({
  runId,
  apiToken,
  tenantId,
  node,
  allNodes = [],
  editable = false,
  telemetry,
  capturedInput,
  capturedOutput,
  pausedAt,
  hasBreakpoint,
  onToggleBreakpoint,
  onStepDebug,
  onContinueDebug,
  debugActionBusy,
  onApplyNodePatch,
  onDeleteNode,
}: NodeIoInspectorProps): React.ReactElement {
  const [activeTab, setActiveTab] = useState<InspectorTab>(() =>
    !runId && onApplyNodePatch ? 'config' : 'io',
  );
  const [overrideOpen, setOverrideOpen] = useState<boolean>(false);
  const [beforeKind, setBeforeKind] = useState<BeforeOverrideKind>('input');
  const [overrideJsonText, setOverrideJsonText] = useState<string>('{\n  "output": ""\n}');
  const [overrideError, setOverrideError] = useState<string | null>(null);

  const mediaPreviews = useMemo(
    () => (node && runId ? extractNodeMediaPreviews(runId, node.id, capturedOutput) : []),
    [runId, node, capturedOutput],
  );

  // Switch to I/O tab automatically when a run starts or pauses at this node.
  useEffect(() => {
    if (runId || pausedAt) {
      setActiveTab((prev) => (prev === 'media' && mediaPreviews.length > 0 ? 'media' : 'io'));
    }
  }, [runId, pausedAt?.node_id, pausedAt?.phase]);

  // Reset override draft whenever pause target or phase changes.
  useEffect(() => {
    setOverrideOpen(false);
    setOverrideError(null);
    if (!pausedAt || pausedAt.node_id !== node?.id) return;
    if (pausedAt.phase === 'before_node') {
      setBeforeKind('input');
      const unwrapped = unwrapCapturedFlowJson(capturedInput);
      setOverrideJsonText(JSON.stringify(unwrapped, null, 2));
    } else {
      const unwrapped = unwrapCapturedFlowJson(capturedOutput);
      setOverrideJsonText(
        Object.keys(unwrapped).length > 0
          ? JSON.stringify(unwrapped, null, 2)
          : '{\n  "output": ""\n}',
      );
    }
  }, [pausedAt?.node_id, pausedAt?.phase, node?.id, capturedInput, capturedOutput]);

  // Keep user off an empty media tab when switching nodes.
  useEffect(() => {
    if (activeTab === 'media' && mediaPreviews.length === 0) {
      setActiveTab(!runId && onApplyNodePatch ? 'config' : 'io');
    }
  }, [node?.id, activeTab, mediaPreviews.length, runId, onApplyNodePatch]);

  if (!node) {
    return (
      <section className="studio-inspector studio-inspector--empty" aria-label="节点配置与观测台">
        <div className="studio-empty-box">
          <h3>节点配置与 I/O 观测台</h3>
          <p>
            点击拓扑画布中的任意节点即可表单化编辑参数与依赖连线（实时同步左侧 YAML）；启动「单步调试」后在此实时核查输入、输出与多媒体产物。
          </p>
        </div>
      </section>
    );
  }

  const isPausedHere = pausedAt?.node_id === node.id;
  const hasInput = capturedInput !== undefined;
  const hasOutput = capturedOutput !== undefined;

  const triggerWithOverride = (mode: 'step' | 'continue') => {
    if (!pausedAt) return;
    const parsed = parseDebugOverrideObject(overrideJsonText);
    if (parsed.error || !parsed.value) {
      setOverrideError(parsed.error ?? 'JSON 格式无效');
      return;
    }
    setOverrideError(null);
    const overrides: DebugResumeOverrides =
      pausedAt.phase === 'before_node' && beforeKind === 'input'
        ? { input_override: parsed.value }
        : { output_override: parsed.value };
    if (mode === 'step') {
      onStepDebug(overrides);
    } else {
      onContinueDebug(overrides);
    }
  };

  const tabs: { id: InspectorTab; label: string; badge?: number }[] = [
    { id: 'io', label: '运行 I/O 观测' },
    { id: 'media', label: '媒体预览', badge: mediaPreviews.length || undefined },
    { id: 'config', label: '节点配置' },
  ];

  const onTabKeyDown = (ev: React.KeyboardEvent<HTMLDivElement>) => {
    if (ev.key !== 'ArrowRight' && ev.key !== 'ArrowLeft') return;
    ev.preventDefault();
    const idx = tabs.findIndex((t) => t.id === activeTab);
    const delta = ev.key === 'ArrowRight' ? 1 : -1;
    const next = tabs[(idx + delta + tabs.length) % tabs.length];
    if (next) {
      setActiveTab(next.id);
    }
  };

  return (
    <section className="studio-inspector" aria-label={`节点 ${node.id} 观测详情`}>
      <header className="studio-inspector__header">
        <div className="studio-inspector__title-group">
          <div className="studio-inspector__eyebrow">
            <span className="studio-inspector__type-pill">{node.type}</span>
            {typeof telemetry?.durationMs === 'number' && (
              <span className="studio-inspector__duration">{telemetry.durationMs} ms</span>
            )}
            {typeof telemetry?.costUsd === 'number' && telemetry.costUsd > 0 && (
              <span className="studio-inspector__duration">
                ${telemetry.costUsd.toFixed(telemetry.costUsd < 0.1 ? 4 : 3)}
              </span>
            )}
          </div>
          <h2 className="studio-inspector__title">{node.id}</h2>
        </div>

        <button
          type="button"
          className={`studio-btn-ghost studio-btn-sm ${hasBreakpoint ? 'studio-btn-bp-on' : ''}`}
          aria-pressed={hasBreakpoint}
          onClick={() => onToggleBreakpoint(node.id)}
        >
          <span className="studio-bp-dot" />
          {hasBreakpoint ? '已设断点' : '设执行前断点'}
        </button>
      </header>

      {isPausedHere && pausedAt && (
        <div className="studio-pause-callout" role="status" aria-live="polite">
          <div className="studio-pause-callout__text">
            <strong>
              {pausedAt.phase === 'before_node'
                ? `⏸ 已在执行前断点挂起：${node.id}`
                : `⏸ 本步执行完毕并暂停：${node.id}`}
            </strong>
            <p>
              {pausedAt.phase === 'before_node'
                ? '调度器已完成上游表达式求值，可核查下方输入直接放行，或修改输入 / Mock 输出跳过真实外部调用。'
                : '请观测下方实际输入与输出；如需修正结果供下游消费，可展开数据注入覆盖本步输出。'}
              {pausedAt.next_nodes.length > 0 && (
                <> 下一就绪节点：{pausedAt.next_nodes.join('、')}。</>
              )}
            </p>
          </div>
          <div className="studio-pause-callout__actions">
            <button
              type="button"
              className="studio-btn-primary studio-btn-sm"
              disabled={debugActionBusy}
              onClick={() => onStepDebug()}
            >
              ▶ 单步下一步
            </button>
            <button
              type="button"
              className="studio-btn-secondary studio-btn-sm"
              disabled={debugActionBusy}
              onClick={() => onContinueDebug()}
            >
              ⏭ 继续跑完
            </button>
            <button
              type="button"
              data-testid="toggle-debug-override-btn"
              className="studio-btn-ghost studio-btn-sm"
              aria-expanded={overrideOpen}
              onClick={() => setOverrideOpen((prev) => !prev)}
            >
              {overrideOpen
                ? '收起注入'
                : pausedAt.phase === 'before_node'
                  ? '🧪 改输入 / Mock 跳过'
                  : '🧪 覆盖输出'}
            </button>
          </div>

          {overrideOpen && (
            <div className="studio-override-panel" data-testid="debug-override-panel">
              {pausedAt.phase === 'before_node' ? (
                <div className="studio-override-modes">
                  <button
                    type="button"
                    className={`studio-cat-pill ${beforeKind === 'input' ? 'active' : ''}`}
                    onClick={() => {
                      setBeforeKind('input');
                      setOverrideError(null);
                      setOverrideJsonText(JSON.stringify(unwrapCapturedFlowJson(capturedInput), null, 2));
                    }}
                  >
                    ✏ 修改输入并执行 (input_override)
                  </button>
                  <button
                    type="button"
                    className={`studio-cat-pill ${beforeKind === 'mock_output' ? 'active' : ''}`}
                    onClick={() => {
                      setBeforeKind('mock_output');
                      setOverrideError(null);
                      setOverrideJsonText('{\n  "output": "mocked_value"\n}');
                    }}
                  >
                    ⚡ Mock 输出跳过调用 (output_override)
                  </button>
                </div>
              ) : (
                <small className="studio-override-hint">
                  将替换 #{node.id} 在状态池与 Checkpoint 中的输出，下游节点将读取下方 JSON：
                </small>
              )}

              <textarea
                rows={5}
                aria-label="调试覆盖 JSON 载荷"
                className="studio-mono-textarea studio-override-textarea"
                value={overrideJsonText}
                onChange={(ev) => setOverrideJsonText(ev.target.value)}
              />

              {overrideError && (
                <p className="studio-override-error" role="alert">
                  {overrideError}
                </p>
              )}

              <div className="studio-pause-callout__actions">
                <button
                  type="button"
                  data-testid="apply-override-step-btn"
                  className="studio-btn-primary studio-btn-sm"
                  disabled={debugActionBusy}
                  onClick={() => triggerWithOverride('step')}
                >
                  {pausedAt.phase === 'before_node' && beforeKind === 'mock_output'
                    ? '⚡ Mock 跳过并单步'
                    : '▶ 注入并单步'}
                </button>
                <button
                  type="button"
                  data-testid="apply-override-continue-btn"
                  className="studio-btn-secondary studio-btn-sm"
                  disabled={debugActionBusy}
                  onClick={() => triggerWithOverride('continue')}
                >
                  ⏭ 注入并继续跑完
                </button>
              </div>
            </div>
          )}
        </div>
      )}

      {telemetry?.error && (
        <div className="studio-node-error" role="alert">
          <strong>节点执行报错：</strong> {telemetry.error}
        </div>
      )}

      {telemetry?.skipReason && (
        <div className="studio-node-skip-note">
          <strong>条件跳过原因：</strong> {telemetry.skipReason}
        </div>
      )}

      <div
        className="studio-tabs"
        role="tablist"
        aria-label="节点观测视图切换"
        onKeyDown={onTabKeyDown}
      >
        {tabs.map((tab) => {
          const selected = activeTab === tab.id;
          return (
            <button
              key={tab.id}
              type="button"
              role="tab"
              aria-selected={selected}
              tabIndex={selected ? 0 : -1}
              className={`studio-tab ${selected ? 'studio-tab--active' : ''}`}
              onClick={() => setActiveTab(tab.id)}
            >
              <span>{tab.label}</span>
              {typeof tab.badge === 'number' && (
                <em className="studio-tab__badge">{tab.badge}</em>
              )}
            </button>
          );
        })}
      </div>

      <div className="studio-inspector__body">
        {activeTab === 'io' && (
          <div className="studio-io-stack">
            <div className="studio-io-block">
              <div className="studio-io-block__head">
                <span>实际输入 Input（表达式求值后）</span>
                <small>{hasInput ? '已捕获' : '等待节点启动'}</small>
              </div>
              {hasInput ? (
                <pre className="studio-json-box" data-testid="inspector-node-input">
                  {prettyJson(capturedInput)}
                </pre>
              ) : (
                <div className="studio-io-placeholder">
                  尚未执行到该节点。映射契约：
                  {Object.keys(node.input_mapping).length > 0 ? (
                    <pre className="studio-json-box studio-json-box--muted">
                      {prettyJson(node.input_mapping)}
                    </pre>
                  ) : (
                    <span>直接读取工作流全局 inputs 与静态 parameters。</span>
                  )}
                </div>
              )}
            </div>

            <div className="studio-io-block">
              <div className="studio-io-block__head">
                <span>实际输出 Output（节点返回载荷）</span>
                <small>
                  {hasOutput
                    ? mediaPreviews.length > 0
                      ? `已捕获 · 含 ${mediaPreviews.length} 项媒体产物`
                      : '已捕获'
                    : '等待节点完成'}
                </small>
              </div>
              {hasOutput ? (
                <pre className="studio-json-box" data-testid="inspector-node-output">
                  {prettyJson(capturedOutput)}
                </pre>
              ) : (
                <div className="studio-io-placeholder">节点完成后将在此展示完整输出 JSON。</div>
              )}
            </div>

            {mediaPreviews.length > 0 && (
              <div className="studio-io-block">
                <div className="studio-io-block__head">
                  <span>即时产物快览</span>
                  <button
                    type="button"
                    className="studio-link-btn"
                    onClick={() => setActiveTab('media')}
                  >
                    查看全部 {mediaPreviews.length} 项 →
                  </button>
                </div>
                <div className="studio-media-list">
                  {mediaPreviews.slice(0, 2).map((item) => (
                    <AuthorizedMedia
                      key={item.key}
                      item={item}
                      apiToken={apiToken}
                      tenantId={tenantId}
                      compact
                    />
                  ))}
                </div>
              </div>
            )}
          </div>
        )}

        {activeTab === 'media' && (
          <div className="studio-media-list">
            {mediaPreviews.length === 0 ? (
              <p className="studio-io-placeholder">
                当前节点暂无图像、音频、视频或交互式 HTML 产物。
              </p>
            ) : (
              mediaPreviews.map((item) => (
                <AuthorizedMedia
                  key={item.key}
                  item={item}
                  apiToken={apiToken}
                  tenantId={tenantId}
                />
              ))
            )}
          </div>
        )}

        {activeTab === 'config' &&
          (onApplyNodePatch ? (
            <NodeConfigEditor
              node={node}
              allNodes={allNodes}
              editable={editable}
              onApplyPatch={onApplyNodePatch}
              onDeleteNode={onDeleteNode}
            />
          ) : (
            <div className="studio-io-stack">
              {node.run_if && (
                <div className="studio-io-block">
                  <div className="studio-io-block__head">
                    <span>条件守卫 (run_if)</span>
                  </div>
                  <pre className="studio-json-box">{node.run_if}</pre>
                </div>
              )}

              <div className="studio-io-block">
                <div className="studio-io-block__head">
                  <span>输入映射表达式 (input_mapping)</span>
                  <small>{Object.keys(node.input_mapping).length} 项</small>
                </div>
                <pre className="studio-json-box">
                  {Object.keys(node.input_mapping).length > 0
                    ? prettyJson(node.input_mapping)
                    : '// 无上游显式字段映射'}
                </pre>
              </div>

              <div className="studio-io-block">
                <div className="studio-io-block__head">
                  <span>静态参数 (parameters)</span>
                </div>
                <pre className="studio-json-box">{prettyJson(node.parameters)}</pre>
              </div>
            </div>
          ))}
      </div>
    </section>
  );
}
