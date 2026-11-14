import React, { useEffect, useMemo, useState } from 'react';
import { prettyJson } from '../lib/helpers';
import type { FlowDebugPause, InspectedNode } from '../schemas';
import { extractNodeMediaPreviews } from '../studio/mediaExtractor';
import { AuthorizedMedia } from './AuthorizedMedia';
import type { NodeRuntimeTelemetry } from './WorkflowGraphCanvas';

type InspectorTab = 'io' | 'media' | 'config';

export interface NodeIoInspectorProps {
  runId: string;
  apiToken: string;
  tenantId: string;
  node: InspectedNode | null;
  telemetry?: NodeRuntimeTelemetry;
  capturedInput: unknown;
  capturedOutput: unknown;
  pausedAt: FlowDebugPause | null;
  hasBreakpoint: boolean;
  onToggleBreakpoint: (nodeId: string) => void;
  onStepDebug: () => void;
  onContinueDebug: () => void;
  debugActionBusy: boolean;
}

export function NodeIoInspector({
  runId,
  apiToken,
  tenantId,
  node,
  telemetry,
  capturedInput,
  capturedOutput,
  pausedAt,
  hasBreakpoint,
  onToggleBreakpoint,
  onStepDebug,
  onContinueDebug,
  debugActionBusy,
}: NodeIoInspectorProps): React.ReactElement {
  const [activeTab, setActiveTab] = useState<InspectorTab>('io');

  const mediaPreviews = useMemo(
    () => (node && runId ? extractNodeMediaPreviews(runId, node.id, capturedOutput) : []),
    [runId, node, capturedOutput],
  );

  // Keep user on I/O tab by default when switching nodes unless the current tab is valid.
  useEffect(() => {
    if (activeTab === 'media' && mediaPreviews.length === 0) {
      setActiveTab('io');
    }
  }, [node?.id, activeTab, mediaPreviews.length]);

  if (!node) {
    return (
      <section className="studio-inspector studio-inspector--empty" aria-label="节点输入输出观测台">
        <div className="studio-empty-box">
          <h3>节点 I/O 观测台</h3>
          <p>点击拓扑画布中的任意节点，或启动「单步调试」，即可在此实时观测该节点的解析后输入、实际输出与多媒体产物。</p>
        </div>
      </section>
    );
  }

  const isPausedHere = pausedAt?.node_id === node.id;
  const hasInput = capturedInput !== undefined;
  const hasOutput = capturedOutput !== undefined;

  const tabs: { id: InspectorTab; label: string; badge?: number }[] = [
    { id: 'io', label: '输入 / 输出观测' },
    { id: 'media', label: '媒体预览', badge: mediaPreviews.length || undefined },
    { id: 'config', label: '映射与配置' },
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
                ? '调度器已完成上游模板与表达式求值，请核查下方「实际输入 Input」后再放行执行。'
                : '请观测下方「实际输入 Input」与「实际输出 Output」是否符合预期。'}
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
              onClick={onStepDebug}
            >
              ▶ 单步下一步
            </button>
            <button
              type="button"
              className="studio-btn-secondary studio-btn-sm"
              disabled={debugActionBusy}
              onClick={onContinueDebug}
            >
              ⏭ 继续跑完
            </button>
          </div>
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

        {activeTab === 'config' && (
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

            {node.sub_nodes && node.sub_nodes.length > 0 && (
              <div className="studio-io-block">
                <div className="studio-io-block__head">
                  <span>内嵌子流水线 ({node.type})</span>
                  <small>{node.sub_nodes.length} 个子节点</small>
                </div>
                <pre className="studio-json-box">{prettyJson(node.sub_nodes)}</pre>
              </div>
            )}

            <div className="studio-io-block">
              <div className="studio-io-block__head">
                <span>静态参数 (parameters)</span>
              </div>
              <pre className="studio-json-box">{prettyJson(node.parameters)}</pre>
            </div>
          </div>
        )}
      </div>
    </section>
  );
}
