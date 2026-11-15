import React, { useEffect, useMemo, useState } from 'react';
import type { InspectedNode } from '../schemas';
import { NODE_PALETTE, type NodeUpdatePatch } from '../studio/yamlGraphMutator';

export interface MappingRowDraft {
  key: string;
  expr: string;
}

export interface NodeEditorDraft {
  idDraft: string;
  typeDraft: string;
  dependenciesDraft: string[];
  mappingRows: MappingRowDraft[];
  runIfDraft: string;
  timeoutDraft: string;
  retriesDraft: string;
  quickTextField: 'prompt' | 'template' | 'input_template' | null;
  quickTextValue: string;
  paramsJsonDraft: string;
}

function detectQuickTextField(
  nodeType: string,
  params: Record<string, unknown>,
): 'prompt' | 'template' | 'input_template' | null {
  if (
    nodeType === 'llm' ||
    nodeType === 'agent' ||
    nodeType === 'skill_agent' ||
    nodeType === 'text_to_image' ||
    nodeType === 'text_to_video'
  ) {
    return 'prompt';
  }
  if (nodeType === 'template') {
    return 'template';
  }
  if (nodeType === 'tts' && ('input_template' in params || Object.keys(params).length === 0)) {
    return 'input_template';
  }
  return null;
}

export function createNodeEditorDraft(node: InspectedNode): NodeEditorDraft {
  const paramsObj: Record<string, unknown> =
    node.parameters && typeof node.parameters === 'object' && !Array.isArray(node.parameters)
      ? { ...(node.parameters as Record<string, unknown>) }
      : {};
  const quickField = detectQuickTextField(node.type, paramsObj);
  const quickVal =
    quickField && typeof paramsObj[quickField] === 'string'
      ? (paramsObj[quickField] as string)
      : '';

  const restParams = { ...paramsObj };
  if (quickField) {
    delete restParams[quickField];
  }

  return {
    idDraft: node.id,
    typeDraft: node.type,
    dependenciesDraft: [...node.dependencies],
    mappingRows: Object.entries(node.input_mapping).map(([key, expr]) => ({ key, expr })),
    runIfDraft: node.run_if ?? '',
    timeoutDraft: typeof node.timeout_ms === 'number' ? String(node.timeout_ms) : '',
    retriesDraft: typeof node.max_retries === 'number' ? String(node.max_retries) : '',
    quickTextField: quickField,
    quickTextValue: quickVal,
    paramsJsonDraft: JSON.stringify(restParams, null, 2),
  };
}

export function buildNodePatchFromDraft(
  original: InspectedNode,
  draft: NodeEditorDraft,
): { patch: NodeUpdatePatch | null; error: string | null } {
  const trimmedId = draft.idDraft.trim();
  if (!trimmedId) {
    return { patch: null, error: '节点 ID 不能为空' };
  }

  let parsedParams: Record<string, unknown> = {};
  const rawJson = draft.paramsJsonDraft.trim();
  if (rawJson) {
    try {
      const parsed = JSON.parse(rawJson) as unknown;
      if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
        return { patch: null, error: 'parameters JSON 必须是对象（如 {"temperature": 0.3}）' };
      }
      parsedParams = { ...(parsed as Record<string, unknown>) };
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      return { patch: null, error: `parameters JSON 解析失败：${msg}` };
    }
  }

  if (draft.quickTextField) {
    parsedParams[draft.quickTextField] = draft.quickTextValue;
  }

  let timeoutMs: number | null = null;
  if (draft.timeoutDraft.trim()) {
    const num = Number(draft.timeoutDraft.trim());
    if (Number.isNaN(num) || num <= 0) {
      return { patch: null, error: '超时时间 timeout_ms 必须是正整数（毫秒）' };
    }
    timeoutMs = Math.round(num);
  }

  let maxRetries: number | null = null;
  if (draft.retriesDraft.trim()) {
    const num = Number(draft.retriesDraft.trim());
    if (Number.isNaN(num) || num < 0) {
      return { patch: null, error: '最大重试次数 max_retries 必须是非负整数' };
    }
    maxRetries = Math.round(num);
  }

  const inputMapping: Record<string, string> = {};
  for (const row of draft.mappingRows) {
    const k = row.key.trim();
    if (!k) continue;
    inputMapping[k] = row.expr;
  }

  const patch: NodeUpdatePatch = {
    newId: trimmedId !== original.id ? trimmedId : undefined,
    type: draft.typeDraft.trim() || original.type,
    dependencies: draft.dependenciesDraft,
    input_mapping: inputMapping,
    run_if: draft.runIfDraft.trim() || null,
    timeout_ms: timeoutMs,
    max_retries: maxRetries,
    parameters: parsedParams,
  };

  return { patch, error: null };
}

export interface NodeConfigEditorProps {
  node: InspectedNode;
  allNodes: InspectedNode[];
  editable: boolean;
  onApplyPatch: (nodeId: string, patch: NodeUpdatePatch) => string | null;
  onDeleteNode?: (nodeId: string) => void;
}

export function NodeConfigEditor({
  node,
  allNodes,
  editable,
  onApplyPatch,
  onDeleteNode,
}: NodeConfigEditorProps): React.ReactElement {
  const nodeSignature = useMemo(() => JSON.stringify(node), [node]);
  const [draft, setDraft] = useState<NodeEditorDraft>(() => createNodeEditorDraft(node));
  const [localError, setLocalError] = useState<string | null>(null);
  const [savedToast, setSavedToast] = useState<boolean>(false);

  useEffect(() => {
    setDraft(createNodeEditorDraft(node));
    setLocalError(null);
    setSavedToast(false);
  }, [nodeSignature, node]);

  const otherNodes = useMemo(
    () => allNodes.filter((candidate) => candidate.id !== node.id),
    [allNodes, node.id],
  );

  const typeOptions = useMemo(() => {
    const list = NODE_PALETTE.map((p) => ({ value: p.type, label: p.label }));
    if (!list.some((item) => item.value === draft.typeDraft)) {
      list.unshift({ value: draft.typeDraft, label: `${draft.typeDraft} (自定义)` });
    }
    return list;
  }, [draft.typeDraft]);

  const applyChanges = () => {
    const built = buildNodePatchFromDraft(node, draft);
    if (built.error || !built.patch) {
      setLocalError(built.error ?? '配置校验未通过');
      return;
    }
    const err = onApplyPatch(node.id, built.patch);
    if (err) {
      setLocalError(err);
      return;
    }
    setLocalError(null);
    setSavedToast(true);
  };

  const toggleDependency = (depId: string) => {
    setDraft((prev) => {
      const exists = prev.dependenciesDraft.includes(depId);
      const nextDeps = exists
        ? prev.dependenciesDraft.filter((d) => d !== depId)
        : [...prev.dependenciesDraft, depId];
      return { ...prev, dependenciesDraft: nextDeps };
    });
    setSavedToast(false);
  };

  const quickFieldLabel =
    draft.quickTextField === 'prompt'
      ? '提示词模板 (parameters.prompt)'
      : draft.quickTextField === 'template'
        ? 'Jinja 模板内容 (parameters.template)'
        : '语音输入模板 (parameters.input_template)';

  return (
    <div
      className="studio-node-config-editor"
      data-testid="node-config-editor"
      onKeyDown={(ev) => {
        if ((ev.metaKey || ev.ctrlKey) && ev.key === 'Enter' && editable) {
          ev.preventDefault();
          applyChanges();
        }
      }}
    >
      {!editable && (
        <div className="studio-config-readonly-banner">
          工作流运行中已锁定拓扑结构编辑，运行结束或终止后可随时修改同步至 YAML。
        </div>
      )}

      {localError && (
        <div className="studio-node-error" role="alert">
          {localError}
        </div>
      )}

      <div className="studio-config-section">
        <div className="studio-config-grid-2">
          <label className="studio-input-field">
            <span className="studio-input-field__label">
              <strong>节点 ID</strong>
              <span>级联更新引用</span>
            </span>
            <input
              type="text"
              disabled={!editable}
              value={draft.idDraft}
              onChange={(ev) => {
                setDraft((prev) => ({ ...prev, idDraft: ev.target.value }));
                setSavedToast(false);
              }}
            />
          </label>

          <label className="studio-input-field">
            <span className="studio-input-field__label">
              <strong>节点类型 (type)</strong>
            </span>
            <select
              disabled={!editable}
              value={draft.typeDraft}
              onChange={(ev) => {
                const nextType = ev.target.value;
                setDraft((prev) => {
                  const nextQuick = detectQuickTextField(nextType, {});
                  return {
                    ...prev,
                    typeDraft: nextType,
                    quickTextField: nextQuick,
                  };
                });
                setSavedToast(false);
              }}
            >
              {typeOptions.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
          </label>
        </div>
      </div>

      <div className="studio-config-section">
        <div className="studio-io-block__head">
          <span>上游依赖节点 (dependencies)</span>
          <small>也可直接在画布拖拽右端口连线</small>
        </div>
        {otherNodes.length === 0 ? (
          <p className="studio-muted-note">当前画布仅有 1 个节点，添加更多节点后可配置上下游依赖。</p>
        ) : (
          <div className="studio-dep-pills">
            {otherNodes.map((cand) => {
              const checked = draft.dependenciesDraft.includes(cand.id);
              return (
                <button
                  key={cand.id}
                  type="button"
                  disabled={!editable}
                  aria-pressed={checked}
                  className={`studio-dep-pill ${checked ? 'studio-dep-pill--active' : ''}`}
                  onClick={() => toggleDependency(cand.id)}
                >
                  {checked ? '✓ ' : '+ '}
                  {cand.id}
                </button>
              );
            })}
          </div>
        )}
      </div>

      <div className="studio-config-section">
        <div className="studio-io-block__head">
          <span>输入字段映射 (input_mapping)</span>
          {editable && (
            <button
              type="button"
              className="studio-link-btn"
              onClick={() => {
                const firstUpstream = draft.dependenciesDraft[0] ?? otherNodes[0]?.id;
                const expr = firstUpstream
                  ? `{{ nodes.${firstUpstream}.outputs.output }}`
                  : '{{ inputs.topic }}';
                setDraft((prev) => ({
                  ...prev,
                  mappingRows: [
                    ...prev.mappingRows,
                    { key: `arg_${prev.mappingRows.length + 1}`, expr },
                  ],
                }));
                setSavedToast(false);
              }}
            >
              + 新增映射项
            </button>
          )}
        </div>

        {draft.mappingRows.length === 0 ? (
          <p className="studio-muted-note">暂无显式字段映射（节点直接读取全局 inputs 或静态参数）。</p>
        ) : (
          <div className="studio-mapping-rows">
            {draft.mappingRows.map((row, idx) => (
              <div key={idx} className="studio-mapping-row">
                <input
                  type="text"
                  aria-label={`映射参数名 ${idx + 1}`}
                  placeholder="字段名"
                  disabled={!editable}
                  value={row.key}
                  onChange={(ev) => {
                    const val = ev.target.value;
                    setDraft((prev) => ({
                      ...prev,
                      mappingRows: prev.mappingRows.map((r, i) =>
                        i === idx ? { ...r, key: val } : r,
                      ),
                    }));
                    setSavedToast(false);
                  }}
                />
                <span className="studio-mapping-arrow">←</span>
                <input
                  type="text"
                  aria-label={`映射表达式 ${idx + 1}`}
                  placeholder="{{ nodes.prev.outputs.output }}"
                  disabled={!editable}
                  value={row.expr}
                  onChange={(ev) => {
                    const val = ev.target.value;
                    setDraft((prev) => ({
                      ...prev,
                      mappingRows: prev.mappingRows.map((r, i) =>
                        i === idx ? { ...r, expr: val } : r,
                      ),
                    }));
                    setSavedToast(false);
                  }}
                />
                {editable && (
                  <button
                    type="button"
                    className="studio-icon-del-btn"
                    title="移除该映射"
                    aria-label={`移除映射 ${row.key || idx + 1}`}
                    onClick={() => {
                      setDraft((prev) => ({
                        ...prev,
                        mappingRows: prev.mappingRows.filter((_, i) => i !== idx),
                      }));
                      setSavedToast(false);
                    }}
                  >
                    ×
                  </button>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {draft.quickTextField && (
        <div className="studio-config-section">
          <label className="studio-input-field">
            <span className="studio-input-field__label">
              <strong>{quickFieldLabel}</strong>
            </span>
            <textarea
              rows={5}
              disabled={!editable}
              value={draft.quickTextValue}
              onChange={(ev) => {
                setDraft((prev) => ({ ...prev, quickTextValue: ev.target.value }));
                setSavedToast(false);
              }}
            />
          </label>
        </div>
      )}

      <div className="studio-config-section">
        <label className="studio-input-field">
          <span className="studio-input-field__label">
            <strong>
              {draft.quickTextField ? '附加静态参数 JSON (parameters)' : '静态参数 JSON (parameters)'}
            </strong>
          </span>
          <textarea
            rows={4}
            className="studio-mono-textarea"
            disabled={!editable}
            value={draft.paramsJsonDraft}
            onChange={(ev) => {
              setDraft((prev) => ({ ...prev, paramsJsonDraft: ev.target.value }));
              setSavedToast(false);
            }}
          />
        </label>
      </div>

      <div className="studio-config-section">
        <label className="studio-input-field">
          <span className="studio-input-field__label">
            <strong>条件守卫 (run_if)</strong>
            <span>optional</span>
          </span>
          <input
            type="text"
            placeholder="例如 {{ nodes.check.outputs.passed }}"
            disabled={!editable}
            value={draft.runIfDraft}
            onChange={(ev) => {
              setDraft((prev) => ({ ...prev, runIfDraft: ev.target.value }));
              setSavedToast(false);
            }}
          />
        </label>

        <div className="studio-config-grid-2">
          <label className="studio-input-field">
            <span className="studio-input-field__label">
              <strong>超时毫秒 (timeout_ms)</strong>
            </span>
            <input
              type="number"
              min={100}
              step={1000}
              placeholder="默认无额外限制"
              disabled={!editable}
              value={draft.timeoutDraft}
              onChange={(ev) => {
                setDraft((prev) => ({ ...prev, timeoutDraft: ev.target.value }));
                setSavedToast(false);
              }}
            />
          </label>

          <label className="studio-input-field">
            <span className="studio-input-field__label">
              <strong>最大重试 (max_retries)</strong>
            </span>
            <input
              type="number"
              min={0}
              max={10}
              placeholder="0"
              disabled={!editable}
              value={draft.retriesDraft}
              onChange={(ev) => {
                setDraft((prev) => ({ ...prev, retriesDraft: ev.target.value }));
                setSavedToast(false);
              }}
            />
          </label>
        </div>
      </div>

      {node.sub_nodes && node.sub_nodes.length > 0 && (
        <div className="studio-config-section">
          <div className="studio-io-block__head">
            <span>内嵌子流水线 ({node.sub_nodes.length} 个子节点自动保留)</span>
          </div>
          <p className="studio-muted-note">
            子节点：{node.sub_nodes.map((s) => `${s.id} (${s.type})`).join(' → ')}
          </p>
        </div>
      )}

      {editable && (
        <div className="studio-config-actions">
          <button
            type="button"
            data-testid="apply-node-config-btn"
            className="studio-btn-primary studio-btn-sm"
            onClick={applyChanges}
          >
            💾 同步到 YAML (⌘/Ctrl+Enter)
          </button>
          {savedToast && <span className="studio-saved-hint">✓ 已同步至 YAML 与画布</span>}
          {onDeleteNode && (
            <button
              type="button"
              data-testid="delete-node-btn"
              className="studio-btn-danger-ghost studio-btn-sm"
              onClick={() => onDeleteNode(node.id)}
            >
              🗑 删除节点
            </button>
          )}
        </div>
      )}
    </div>
  );
}
