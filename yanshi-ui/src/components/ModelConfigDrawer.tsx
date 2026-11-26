import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { apiFetch, formatApiError } from '../lib/api';
import { ExpandableDrawer } from './ExpandableDrawer';

export interface ProviderConfigItem {
  name: string;
  protocol: string;
  base_url: string | null;
  default_base_url: string | null;
  api_key_env: string;
  api_key_set: boolean;
  api_key_masked: string | null;
  api_key_source: string;
  timeout_seconds: number | null;
  requests_per_minute: number | null;
  is_custom: boolean;
  is_overridden: boolean;
  model_count: number;
}

export interface ModelConfigItem {
  name: string;
  vendor: string;
  model_id: string;
  model_type: string;
  accepts: string[];
  base_url: string | null;
  effective_base_url: string | null;
  temperature: number | null;
  max_tokens: number | null;
  supports_streaming: boolean;
  supports_tools: boolean;
  native_tool_calling: boolean;
  supports_thinking: boolean;
  thinking_kind: string | null;
  is_custom: boolean;
  is_overridden: boolean;
}

export interface ModelConfigSnapshot {
  config_dir: string;
  models_path: string;
  env_path: string;
  providers: ProviderConfigItem[];
  models: ModelConfigItem[];
}

export interface TestProviderOutcome {
  ok: boolean;
  provider: string;
  protocol: string;
  base_url: string | null;
  latency_ms: number;
  error: string | null;
}

export const MODELS_CONFIG_UPDATED_EVENT = 'yanshi:models-updated';
export const MODEL_SUGGESTIONS_DATALIST_ID = 'yanshi-model-suggestions';

const CUSTOM_PROVIDER_PRESETS: ReadonlyArray<{
  label: string;
  name: string;
  protocol: 'openai' | 'anthropic' | 'google';
  baseUrl: string;
  envVar: string;
}> = [
  {
    label: 'OpenRouter',
    name: 'openrouter',
    protocol: 'openai',
    baseUrl: 'https://openrouter.ai/api/v1',
    envVar: 'OPENROUTER_API_KEY',
  },
  {
    label: 'Ollama (本地)',
    name: 'ollama',
    protocol: 'openai',
    baseUrl: 'http://127.0.0.1:11434/v1',
    envVar: 'OLLAMA_API_KEY',
  },
  {
    label: 'SiliconFlow',
    name: 'siliconflow',
    protocol: 'openai',
    baseUrl: 'https://api.siliconflow.cn/v1',
    envVar: 'SILICONFLOW_API_KEY',
  },
  {
    label: 'OneAPI / 中转',
    name: 'oneapi',
    protocol: 'openai',
    baseUrl: 'https://your-relay.example.com/v1',
    envVar: 'ONEAPI_API_KEY',
  },
];

const MODEL_TYPES = [
  'chat',
  'embedding',
  'text_to_image',
  'image_to_image',
  'image_edit',
  'text_to_video',
  'tts',
  'asr',
] as const;

const INPUT_MODALITIES = ['text', 'image', 'audio', 'video', 'document'] as const;

/**
 * Lightweight hook fetching configured models & vendor readiness so Studio,
 * Notebook, and Harness model inputs offer live autocomplete suggestions.
 */
export function useModelSuggestions(apiToken: string): {
  models: ModelConfigItem[];
  providersByName: Record<string, ProviderConfigItem>;
} {
  const [snapshot, setSnapshot] = useState<ModelConfigSnapshot | null>(null);

  const load = useCallback(async () => {
    try {
      const res = await apiFetch('/v1/models/config', apiToken);
      if (!res.ok) return;
      const data = (await res.json()) as ModelConfigSnapshot;
      if (data && Array.isArray(data.models) && Array.isArray(data.providers)) {
        setSnapshot(data);
      }
    } catch {
      // Non-fatal: model autocomplete degrades silently if backend is offline.
    }
  }, [apiToken]);

  useEffect(() => {
    void load();
    const handler = () => {
      void load();
    };
    window.addEventListener(MODELS_CONFIG_UPDATED_EVENT, handler);
    return () => {
      window.removeEventListener(MODELS_CONFIG_UPDATED_EVENT, handler);
    };
  }, [load]);

  const providersByName = useMemo(() => {
    const map: Record<string, ProviderConfigItem> = {};
    for (const p of snapshot?.providers ?? []) {
      map[p.name] = p;
    }
    return map;
  }, [snapshot]);

  return {
    models: snapshot?.models ?? [],
    providersByName,
  };
}

/** Shared `<datalist>` rendering live configured model suggestions. */
export function ModelSuggestionsDatalist({
  apiToken,
  id = MODEL_SUGGESTIONS_DATALIST_ID,
}: {
  apiToken: string;
  id?: string;
}): React.ReactElement {
  const { models, providersByName } = useModelSuggestions(apiToken);
  const sorted = useMemo(() => {
    return [...models].sort((a, b) => {
      const aReady = providersByName[a.vendor]?.api_key_set ? 1 : 0;
      const bReady = providersByName[b.vendor]?.api_key_set ? 1 : 0;
      if (aReady !== bReady) return bReady - aReady;
      return a.name.localeCompare(b.name);
    });
  }, [models, providersByName]);

  return (
    <datalist id={id}>
      {sorted.map((m) => {
        const ready = providersByName[m.vendor]?.api_key_set;
        return (
          <option
            key={m.name}
            value={m.name}
            label={`${m.vendor} · ${m.model_type}${ready ? ' · ✓已配 Key' : ''}`}
          />
        );
      })}
    </datalist>
  );
}

export interface ModelConfigDrawerProps {
  open: boolean;
  onClose: () => void;
  apiToken: string;
}

export function ModelConfigDrawer({
  open,
  onClose,
  apiToken,
}: ModelConfigDrawerProps): React.ReactElement | null {
  const [activeTab, setActiveTab] = useState<'providers' | 'models'>('providers');
  const [snapshot, setSnapshot] = useState<ModelConfigSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  // Provider editor state
  const [selectedProvider, setSelectedProvider] = useState<string>('openai');
  const [creatingCustomProvider, setCreatingCustomProvider] = useState(false);
  const [providerNameDraft, setProviderNameDraft] = useState('');
  const [protocolDraft, setProtocolDraft] = useState<'openai' | 'anthropic' | 'google'>('openai');
  const [baseUrlDraft, setBaseUrlDraft] = useState('');
  const [apiKeyEnvDraft, setApiKeyEnvDraft] = useState('');
  const [apiKeyDraft, setApiKeyDraft] = useState('');
  const [showApiKey, setShowApiKey] = useState(false);
  const [timeoutDraft, setTimeoutDraft] = useState('');
  const [rpmDraft, setRpmDraft] = useState('');
  const [saveBusy, setSaveBusy] = useState(false);
  const [testBusy, setTestBusy] = useState(false);
  const [testOutcome, setTestOutcome] = useState<TestProviderOutcome | null>(null);

  // Model catalog editor state
  const [modelSearch, setModelSearch] = useState('');
  const [modelVendorFilter, setModelVendorFilter] = useState('all');
  const [editingModelName, setEditingModelName] = useState<string | null>(null);
  const [creatingModel, setCreatingModel] = useState(false);
  const [modelNameDraft, setModelNameDraft] = useState('');
  const [modelVendorDraft, setModelVendorDraft] = useState('openai');
  const [modelIdDraft, setModelIdDraft] = useState('');
  const [modelTypeDraft, setModelTypeDraft] = useState('chat');
  const [modelAcceptsDraft, setModelAcceptsDraft] = useState<string[]>(['text']);
  const [modelBaseUrlDraft, setModelBaseUrlDraft] = useState('');
  const [modelTempDraft, setModelTempDraft] = useState('');
  const [modelMaxTokensDraft, setModelMaxTokensDraft] = useState('');
  const [modelToolsDraft, setModelToolsDraft] = useState(true);
  const [modelThinkingDraft, setModelThinkingDraft] = useState(false);

  const hydrateProviderForm = useCallback((item: ProviderConfigItem) => {
    setCreatingCustomProvider(false);
    setSelectedProvider(item.name);
    setProviderNameDraft(item.name);
    setProtocolDraft(
      item.protocol === 'anthropic' || item.protocol === 'google' ? item.protocol : 'openai',
    );
    setBaseUrlDraft(item.base_url ?? item.default_base_url ?? '');
    setApiKeyEnvDraft(item.api_key_env);
    setApiKeyDraft('');
    setShowApiKey(false);
    setTimeoutDraft(item.timeout_seconds ? String(item.timeout_seconds) : '');
    setRpmDraft(item.requests_per_minute ? String(item.requests_per_minute) : '');
    setTestOutcome(null);
  }, []);

  const fetchSnapshot = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await apiFetch('/v1/models/config', apiToken);
      if (!res.ok) {
        const text = await res.text().catch(() => '');
        throw new Error(text || `HTTP ${res.status}`);
      }
      const data = (await res.json()) as ModelConfigSnapshot;
      setSnapshot(data);
      const current =
        data.providers.find((p) => p.name === selectedProvider) ?? data.providers[0];
      if (current && !creatingCustomProvider) {
        hydrateProviderForm(current);
      }
    } catch (err) {
      setError(formatApiError(err));
    } finally {
      setLoading(false);
    }
  }, [apiToken, selectedProvider, creatingCustomProvider, hydrateProviderForm]);

  useEffect(() => {
    if (open) {
      setNotice(null);
      void fetchSnapshot();
    }
  }, [open, fetchSnapshot]);

  const activeProviderObj = useMemo(
    () => snapshot?.providers.find((p) => p.name === selectedProvider) ?? null,
    [snapshot, selectedProvider],
  );

  const notifyModelsUpdated = () => {
    window.dispatchEvent(new CustomEvent(MODELS_CONFIG_UPDATED_EVENT));
  };

  const handleSaveProvider = async (clearKey = false) => {
    const targetName = (creatingCustomProvider ? providerNameDraft : selectedProvider).trim();
    if (!targetName) {
      setError('请填写厂商标识名（如 openrouter、ollama、my-relay）');
      return;
    }
    setSaveBusy(true);
    setError(null);
    setNotice(null);
    try {
      const payload: Record<string, unknown> = {
        base_url: baseUrlDraft.trim(),
        api_key_env: apiKeyEnvDraft.trim() || undefined,
      };
      if (creatingCustomProvider || activeProviderObj?.is_custom) {
        payload.protocol = protocolDraft;
      }
      if (clearKey) {
        payload.api_key = '';
      } else if (apiKeyDraft.trim()) {
        payload.api_key = apiKeyDraft.trim();
      }
      const parsedTimeout = Number(timeoutDraft.trim());
      if (timeoutDraft.trim() && Number.isFinite(parsedTimeout) && parsedTimeout > 0) {
        payload.timeout_seconds = Math.round(parsedTimeout);
      }
      const parsedRpm = Number(rpmDraft.trim());
      if (rpmDraft.trim() && Number.isFinite(parsedRpm) && parsedRpm > 0) {
        payload.requests_per_minute = Math.round(parsedRpm);
      }

      const res = await apiFetch(`/v1/models/providers/${encodeURIComponent(targetName)}`, apiToken, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });
      if (!res.ok) {
        const text = await res.text().catch(() => '');
        throw new Error(text || `HTTP ${res.status}`);
      }
      const updated = (await res.json()) as ModelConfigSnapshot;
      setSnapshot(updated);
      const savedItem = updated.providers.find((p) => p.name === targetName);
      if (savedItem) {
        hydrateProviderForm(savedItem);
      }
      setNotice(
        clearKey
          ? `已清除厂商 ${targetName} 的 API Key 并热重载生效。`
          : `已保存厂商 ${targetName} 配置到 ~/.yanshi/ 并立即热生效。`,
      );
      notifyModelsUpdated();
    } catch (err) {
      setError(formatApiError(err));
    } finally {
      setSaveBusy(false);
    }
  };

  const handleDeleteOrResetProvider = async () => {
    if (!activeProviderObj) return;
    setSaveBusy(true);
    setError(null);
    setNotice(null);
    try {
      const clearKey = activeProviderObj.is_custom ? 'true' : 'false';
      const res = await apiFetch(
        `/v1/models/providers/${encodeURIComponent(activeProviderObj.name)}?clear_api_key=${clearKey}`,
        apiToken,
        { method: 'DELETE' },
      );
      if (!res.ok) {
        const text = await res.text().catch(() => '');
        throw new Error(text || `HTTP ${res.status}`);
      }
      const updated = (await res.json()) as ModelConfigSnapshot;
      setSnapshot(updated);
      const next =
        updated.providers.find((p) => p.name === activeProviderObj.name) ?? updated.providers[0];
      if (next) {
        hydrateProviderForm(next);
      }
      setNotice(
        activeProviderObj.is_custom
          ? `已删除自定义厂商 ${activeProviderObj.name}。`
          : `已将 ${activeProviderObj.name} 恢复为内置默认地址与参数。`,
      );
      notifyModelsUpdated();
    } catch (err) {
      setError(formatApiError(err));
    } finally {
      setSaveBusy(false);
    }
  };

  const handleTestProvider = async () => {
    const targetName = (creatingCustomProvider ? providerNameDraft : selectedProvider).trim();
    if (!targetName) {
      setError('请先填写厂商标识名');
      return;
    }
    setTestBusy(true);
    setError(null);
    setTestOutcome(null);
    try {
      const res = await apiFetch(
        `/v1/models/providers/${encodeURIComponent(targetName)}/test`,
        apiToken,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            protocol:
              creatingCustomProvider || activeProviderObj?.is_custom ? protocolDraft : undefined,
            base_url: baseUrlDraft.trim() || undefined,
            api_key: apiKeyDraft.trim() || undefined,
          }),
        },
      );
      if (!res.ok) {
        const text = await res.text().catch(() => '');
        throw new Error(text || `HTTP ${res.status}`);
      }
      const result = (await res.json()) as TestProviderOutcome;
      setTestOutcome(result);
    } catch (err) {
      setError(formatApiError(err));
    } finally {
      setTestBusy(false);
    }
  };

  const openModelEditor = (item: ModelConfigItem) => {
    setCreatingModel(false);
    setEditingModelName(item.name);
    setModelNameDraft(item.name);
    setModelVendorDraft(item.vendor);
    setModelIdDraft(item.model_id);
    setModelTypeDraft(item.model_type || 'chat');
    setModelAcceptsDraft(item.accepts.length > 0 ? item.accepts : ['text']);
    setModelBaseUrlDraft(item.base_url ?? '');
    setModelTempDraft(item.temperature !== null ? String(item.temperature) : '');
    setModelMaxTokensDraft(item.max_tokens !== null ? String(item.max_tokens) : '');
    setModelToolsDraft(item.supports_tools);
    setModelThinkingDraft(item.supports_thinking);
  };

  const startCreateModel = () => {
    setCreatingModel(true);
    setEditingModelName(null);
    setModelNameDraft('');
    setModelVendorDraft(selectedProvider || snapshot?.providers[0]?.name || 'openai');
    setModelIdDraft('');
    setModelTypeDraft('chat');
    setModelAcceptsDraft(['text']);
    setModelBaseUrlDraft('');
    setModelTempDraft('');
    setModelMaxTokensDraft('');
    setModelToolsDraft(true);
    setModelThinkingDraft(false);
  };

  const handleSaveModel = async () => {
    const name = (creatingModel ? modelNameDraft : editingModelName ?? modelNameDraft).trim();
    if (!name) {
      setError('请填写模型别名（例如 my-gpt-4o 或 deepseek-r1）');
      return;
    }
    setSaveBusy(true);
    setError(null);
    setNotice(null);
    try {
      const payload: Record<string, unknown> = {
        vendor: modelVendorDraft.trim(),
        model_id: modelIdDraft.trim() || undefined,
        model_type: modelTypeDraft,
        accepts: modelAcceptsDraft,
        base_url: modelBaseUrlDraft.trim(),
        supports_tools: modelToolsDraft,
        native_tool_calling: modelToolsDraft,
        supports_thinking: modelThinkingDraft,
      };
      const parsedTemp = Number(modelTempDraft.trim());
      if (modelTempDraft.trim() && Number.isFinite(parsedTemp)) {
        payload.temperature = parsedTemp;
      }
      const parsedMax = Number(modelMaxTokensDraft.trim());
      if (modelMaxTokensDraft.trim() && Number.isFinite(parsedMax) && parsedMax > 0) {
        payload.max_tokens = Math.round(parsedMax);
      }

      const res = await apiFetch(`/v1/models/items/${encodeURIComponent(name)}`, apiToken, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });
      if (!res.ok) {
        const text = await res.text().catch(() => '');
        throw new Error(text || `HTTP ${res.status}`);
      }
      const updated = (await res.json()) as ModelConfigSnapshot;
      setSnapshot(updated);
      const saved = updated.models.find((m) => m.name === name);
      if (saved) {
        openModelEditor(saved);
      }
      setNotice(`已保存模型 ${name} 到 ~/.yanshi/models.yml 并热生效。`);
      notifyModelsUpdated();
    } catch (err) {
      setError(formatApiError(err));
    } finally {
      setSaveBusy(false);
    }
  };

  const handleDeleteOrResetModel = async (name: string, isCustom: boolean) => {
    setSaveBusy(true);
    setError(null);
    setNotice(null);
    try {
      const res = await apiFetch(`/v1/models/items/${encodeURIComponent(name)}`, apiToken, {
        method: 'DELETE',
      });
      if (!res.ok) {
        const text = await res.text().catch(() => '');
        throw new Error(text || `HTTP ${res.status}`);
      }
      const updated = (await res.json()) as ModelConfigSnapshot;
      setSnapshot(updated);
      if (isCustom && editingModelName === name) {
        setEditingModelName(null);
      } else {
        const refreshed = updated.models.find((m) => m.name === name);
        if (refreshed) openModelEditor(refreshed);
      }
      setNotice(isCustom ? `已删除自定义模型 ${name}。` : `已恢复模型 ${name} 为内置默认配置。`);
      notifyModelsUpdated();
    } catch (err) {
      setError(formatApiError(err));
    } finally {
      setSaveBusy(false);
    }
  };

  const filteredModels = useMemo(() => {
    const q = modelSearch.trim().toLowerCase();
    return (snapshot?.models ?? []).filter((m) => {
      if (modelVendorFilter !== 'all' && m.vendor !== modelVendorFilter) {
        return false;
      }
      if (!q) return true;
      return (
        m.name.toLowerCase().includes(q) ||
        m.vendor.toLowerCase().includes(q) ||
        m.model_id.toLowerCase().includes(q) ||
        m.model_type.toLowerCase().includes(q)
      );
    });
  }, [snapshot, modelSearch, modelVendorFilter]);

  const readyProviderCount = useMemo(
    () => (snapshot?.providers ?? []).filter((p) => p.api_key_set).length,
    [snapshot],
  );

  return (
    <ExpandableDrawer
      open={open}
      onClose={onClose}
      title="模型与厂商配置"
      badge={
        snapshot
          ? `${readyProviderCount}/${snapshot.providers.length} 厂商就绪 · ${snapshot.models.length} 模型`
          : undefined
      }
      subtitle={
        snapshot
          ? `持久化于 ${snapshot.models_path} 与 ${snapshot.env_path}（与 CLI 互通，保存后热生效）`
          : '配置各厂商 Base URL、API Key、自定义中转站与模型参数'
      }
      side="right"
      defaultSize="drawer"
      testId="model-config-drawer"
      headerActions={
        <div className="ys-model-tabs" role="tablist" aria-label="模型配置视图切换">
          <button
            type="button"
            role="tab"
            data-testid="model-config-tab-providers"
            aria-selected={activeTab === 'providers'}
            className={`studio-btn-ghost studio-btn-sm ${
              activeTab === 'providers' ? 'ys-model-tab--active' : ''
            }`}
            onClick={() => setActiveTab('providers')}
          >
            厂商与密钥
          </button>
          <button
            type="button"
            role="tab"
            data-testid="model-config-tab-models"
            aria-selected={activeTab === 'models'}
            className={`studio-btn-ghost studio-btn-sm ${
              activeTab === 'models' ? 'ys-model-tab--active' : ''
            }`}
            onClick={() => setActiveTab('models')}
          >
            模型目录
          </button>
        </div>
      }
    >
      {error ? (
        <div className="studio-alert studio-alert--error" role="alert">
          <span>{error}</span>
          <button type="button" onClick={() => setError(null)}>
            关闭
          </button>
        </div>
      ) : null}
      {notice ? (
        <div className="studio-alert studio-alert--info" role="status">
          <span>{notice}</span>
          <button type="button" onClick={() => setNotice(null)}>
            知道了
          </button>
        </div>
      ) : null}

      {loading && !snapshot ? (
        <p className="studio-muted-note">正在读取当前厂商与模型注册表…</p>
      ) : activeTab === 'providers' ? (
        <div className="ys-model-grid">
          <aside className="ys-model-sidebar" aria-label="厂商列表">
            <button
              type="button"
              data-testid="add-custom-provider-btn"
              className="studio-btn-secondary studio-btn-sm ys-model-add-btn"
              onClick={() => {
                setCreatingCustomProvider(true);
                setProviderNameDraft('');
                setProtocolDraft('openai');
                setBaseUrlDraft('');
                setApiKeyEnvDraft('');
                setApiKeyDraft('');
                setTimeoutDraft('180');
                setRpmDraft('');
                setTestOutcome(null);
              }}
            >
              + 新增自定义厂商 / 中转
            </button>

            <ul className="ys-provider-list">
              {(snapshot?.providers ?? []).map((p) => {
                const isSelected = !creatingCustomProvider && p.name === selectedProvider;
                return (
                  <li key={p.name}>
                    <button
                      type="button"
                      data-testid={`provider-item-${p.name}`}
                      className={`ys-provider-item ${isSelected ? 'ys-provider-item--active' : ''}`}
                      onClick={() => hydrateProviderForm(p)}
                    >
                      <div className="ys-provider-item__top">
                        <span
                          className={`ys-status-dot ${
                            p.api_key_set ? 'ys-status-dot--ok' : 'ys-status-dot--idle'
                          }`}
                          title={p.api_key_set ? 'API Key 已配置' : '未配置 API Key'}
                        />
                        <strong>{p.name}</strong>
                        {p.is_custom ? (
                          <span className="ys-pill ys-pill--custom">自定义</span>
                        ) : p.is_overridden ? (
                          <span className="ys-pill ys-pill--override">已覆写</span>
                        ) : null}
                      </div>
                      <small>
                        {p.model_count} 模型 · {p.api_key_set ? p.api_key_masked ?? '已配 Key' : '未配 Key'}
                      </small>
                    </button>
                  </li>
                );
              })}
            </ul>
          </aside>

          <section className="ys-model-form" aria-label="厂商参数编辑">
            {creatingCustomProvider ? (
              <div className="ys-preset-chips">
                <span>快捷模板：</span>
                {CUSTOM_PROVIDER_PRESETS.map((preset) => (
                  <button
                    key={preset.name}
                    type="button"
                    className="studio-btn-ghost studio-btn-sm"
                    onClick={() => {
                      setProviderNameDraft(preset.name);
                      setProtocolDraft(preset.protocol);
                      setBaseUrlDraft(preset.baseUrl);
                      setApiKeyEnvDraft(preset.envVar);
                    }}
                  >
                    {preset.label}
                  </button>
                ))}
              </div>
            ) : null}

            <div className="ys-form-row">
              <label className="studio-input-field">
                <div className="studio-input-field__label">
                  <strong>厂商标识 (vendor)</strong>
                  <span>{creatingCustomProvider ? 'required' : activeProviderObj?.is_custom ? 'custom' : 'built-in'}</span>
                </div>
                <input
                  type="text"
                  data-testid="provider-name-input"
                  value={providerNameDraft}
                  readOnly={!creatingCustomProvider}
                  placeholder="例如 openrouter / ollama / my-relay"
                  onChange={(ev) => setProviderNameDraft(ev.target.value)}
                />
              </label>

              {creatingCustomProvider || activeProviderObj?.is_custom ? (
                <label className="studio-input-field">
                  <div className="studio-input-field__label">
                    <strong>兼容协议 (protocol)</strong>
                  </div>
                  <select
                    data-testid="provider-protocol-select"
                    value={protocolDraft}
                    onChange={(ev) =>
                      setProtocolDraft(ev.target.value as 'openai' | 'anthropic' | 'google')
                    }
                  >
                    <option value="openai">OpenAI 兼容 (/chat/completions)</option>
                    <option value="anthropic">Anthropic 兼容 (/v1/messages)</option>
                    <option value="google">Google Gemini 兼容</option>
                  </select>
                </label>
              ) : null}
            </div>

            <label className="studio-input-field">
              <div className="studio-input-field__label">
                <strong>Base URL</strong>
                {activeProviderObj?.default_base_url ? (
                  <button
                    type="button"
                    className="studio-link-btn"
                    onClick={() => setBaseUrlDraft(activeProviderObj.default_base_url ?? '')}
                  >
                    恢复官方默认
                  </button>
                ) : null}
              </div>
              <input
                type="url"
                data-testid="provider-base-url-input"
                value={baseUrlDraft}
                placeholder={activeProviderObj?.default_base_url ?? 'https://api.example.com/v1'}
                onChange={(ev) => setBaseUrlDraft(ev.target.value)}
              />
            </label>

            <label className="studio-input-field">
              <div className="studio-input-field__label">
                <strong>API Key（写入 ~/.yanshi/.env，权限 0600）</strong>
                <div className="studio-pane__head-actions">
                  <button
                    type="button"
                    className="studio-link-btn"
                    onClick={() => setShowApiKey((v) => !v)}
                  >
                    {showApiKey ? '隐藏' : '显示'}
                  </button>
                  {activeProviderObj?.api_key_set && !creatingCustomProvider ? (
                    <button
                      type="button"
                      data-testid="provider-clear-key-btn"
                      className="studio-link-btn nb-danger-link"
                      disabled={saveBusy}
                      onClick={() => void handleSaveProvider(true)}
                    >
                      清空已存 Key
                    </button>
                  ) : null}
                </div>
              </div>
              <input
                type={showApiKey ? 'text' : 'password'}
                data-testid="provider-api-key-input"
                autoComplete="off"
                value={apiKeyDraft}
                placeholder={
                  activeProviderObj?.api_key_set && !creatingCustomProvider
                    ? `当前已配置 (${activeProviderObj.api_key_masked ?? '已脱敏'}) — 留空保持不变，输入新 Key 覆盖`
                    : '输入 API Key（本地 Ollama 可填 ollama）'
                }
                onChange={(ev) => setApiKeyDraft(ev.target.value)}
              />
            </label>

            <div className="ys-form-row">
              <label className="studio-input-field">
                <div className="studio-input-field__label">
                  <strong>环境变量名 (api_key_env)</strong>
                </div>
                <input
                  type="text"
                  value={apiKeyEnvDraft}
                  placeholder="自动生成，如 OPENROUTER_API_KEY"
                  onChange={(ev) => setApiKeyEnvDraft(ev.target.value)}
                />
              </label>

              <label className="studio-input-field">
                <div className="studio-input-field__label">
                  <strong>超时秒数</strong>
                </div>
                <input
                  type="number"
                  min="5"
                  value={timeoutDraft}
                  placeholder="180"
                  onChange={(ev) => setTimeoutDraft(ev.target.value)}
                />
              </label>

              <label className="studio-input-field">
                <div className="studio-input-field__label">
                  <strong>限流 RPM</strong>
                </div>
                <input
                  type="number"
                  min="1"
                  value={rpmDraft}
                  placeholder="500"
                  onChange={(ev) => setRpmDraft(ev.target.value)}
                />
              </label>
            </div>

            {testOutcome ? (
              <div
                data-testid="provider-test-result"
                className={`studio-alert ${
                  testOutcome.ok ? 'studio-alert--info' : 'studio-alert--error'
                }`}
              >
                <span>
                  {testOutcome.ok
                    ? `✓ 连接校验通过（协议 ${testOutcome.protocol} · 延迟 ${testOutcome.latency_ms} ms）`
                    : `✕ 连接校验失败（${testOutcome.latency_ms} ms）：${testOutcome.error ?? '未知错误'}`}
                </span>
              </div>
            ) : null}

            <div className="ys-model-form__actions">
              <button
                type="button"
                data-testid="provider-save-btn"
                className="studio-btn-primary"
                disabled={saveBusy}
                onClick={() => void handleSaveProvider(false)}
              >
                {saveBusy ? '保存中…' : '💾 保存并热生效'}
              </button>
              <button
                type="button"
                data-testid="provider-test-btn"
                className="studio-btn-secondary"
                disabled={testBusy}
                onClick={() => void handleTestProvider()}
              >
                {testBusy ? '测试连接中…' : '🔌 测试连接'}
              </button>
              {!creatingCustomProvider &&
              activeProviderObj &&
              (activeProviderObj.is_custom || activeProviderObj.is_overridden) ? (
                <button
                  type="button"
                  data-testid="provider-delete-btn"
                  className="studio-btn-danger-ghost"
                  disabled={saveBusy}
                  onClick={() => void handleDeleteOrResetProvider()}
                >
                  {activeProviderObj.is_custom ? '删除自定义厂商' : '恢复内置默认'}
                </button>
              ) : null}
            </div>
          </section>
        </div>
      ) : (
        <div className="ys-model-grid">
          <aside className="ys-model-sidebar" aria-label="模型目录列表">
            <button
              type="button"
              data-testid="add-custom-model-btn"
              className="studio-btn-secondary studio-btn-sm ys-model-add-btn"
              onClick={startCreateModel}
            >
              + 新增自定义模型
            </button>
            <input
              type="search"
              className="ys-model-search"
              placeholder="搜索模型名 / ID / 厂商…"
              value={modelSearch}
              onChange={(ev) => setModelSearch(ev.target.value)}
            />
            <select
              className="ys-model-filter"
              aria-label="按厂商筛选模型"
              value={modelVendorFilter}
              onChange={(ev) => setModelVendorFilter(ev.target.value)}
            >
              <option value="all">全部厂商 ({snapshot?.models.length ?? 0})</option>
              {(snapshot?.providers ?? []).map((p) => (
                <option key={p.name} value={p.name}>
                  {p.name} ({p.model_count})
                </option>
              ))}
            </select>

            <ul className="ys-provider-list">
              {filteredModels.map((m) => (
                <li key={m.name}>
                  <button
                    type="button"
                    data-testid={`model-item-${m.name}`}
                    className={`ys-provider-item ${
                      !creatingModel && editingModelName === m.name
                        ? 'ys-provider-item--active'
                        : ''
                    }`}
                    onClick={() => openModelEditor(m)}
                  >
                    <div className="ys-provider-item__top">
                      <strong>{m.name}</strong>
                      {m.is_custom ? (
                        <span className="ys-pill ys-pill--custom">自定义</span>
                      ) : m.is_overridden ? (
                        <span className="ys-pill ys-pill--override">已覆写</span>
                      ) : null}
                    </div>
                    <small>
                      {m.vendor} · {m.model_type}
                    </small>
                  </button>
                </li>
              ))}
            </ul>
          </aside>

          <section className="ys-model-form" aria-label="模型参数编辑">
            {!creatingModel && !editingModelName ? (
              <p className="studio-muted-note">
                点击左侧任意模型查看或覆写其上游 Model ID / 独立 Base URL / 推理参数，或点击「+ 新增自定义模型」为自定义厂商添加模型。
              </p>
            ) : (
              <>
                <div className="ys-form-row">
                  <label className="studio-input-field">
                    <div className="studio-input-field__label">
                      <strong>模型别名 (name)</strong>
                    </div>
                    <input
                      type="text"
                      data-testid="model-name-input"
                      value={modelNameDraft}
                      readOnly={!creatingModel}
                      placeholder="例如 my-qwen3-coder 或 llama3.3"
                      onChange={(ev) => setModelNameDraft(ev.target.value)}
                    />
                  </label>

                  <label className="studio-input-field">
                    <div className="studio-input-field__label">
                      <strong>所属厂商 (vendor)</strong>
                    </div>
                    <select
                      data-testid="model-vendor-select"
                      value={modelVendorDraft}
                      onChange={(ev) => setModelVendorDraft(ev.target.value)}
                    >
                      {(snapshot?.providers ?? []).map((p) => (
                        <option key={p.name} value={p.name}>
                          {p.name} {p.is_custom ? '(自定义)' : ''}
                        </option>
                      ))}
                    </select>
                  </label>
                </div>

                <div className="ys-form-row">
                  <label className="studio-input-field">
                    <div className="studio-input-field__label">
                      <strong>上游实际 Model ID（可选）</strong>
                    </div>
                    <input
                      type="text"
                      data-testid="model-id-input"
                      value={modelIdDraft}
                      placeholder="留空则与模型别名一致（如 openai/gpt-4o）"
                      onChange={(ev) => setModelIdDraft(ev.target.value)}
                    />
                  </label>

                  <label className="studio-input-field">
                    <div className="studio-input-field__label">
                      <strong>模型类型</strong>
                    </div>
                    <select
                      value={modelTypeDraft}
                      onChange={(ev) => setModelTypeDraft(ev.target.value)}
                    >
                      {MODEL_TYPES.map((t) => (
                        <option key={t} value={t}>
                          {t}
                        </option>
                      ))}
                    </select>
                  </label>
                </div>

                <label className="studio-input-field">
                  <div className="studio-input-field__label">
                    <strong>单模型独立 Base URL（可选，留空继承厂商地址）</strong>
                  </div>
                  <input
                    type="url"
                    value={modelBaseUrlDraft}
                    placeholder="例如 https://special-endpoint.example.com/v1"
                    onChange={(ev) => setModelBaseUrlDraft(ev.target.value)}
                  />
                </label>

                <div className="ys-form-row">
                  <label className="studio-input-field">
                    <div className="studio-input-field__label">
                      <strong>默认 Temperature</strong>
                    </div>
                    <input
                      type="number"
                      step="0.1"
                      min="0"
                      max="2"
                      value={modelTempDraft}
                      placeholder="0.7"
                      onChange={(ev) => setModelTempDraft(ev.target.value)}
                    />
                  </label>

                  <label className="studio-input-field">
                    <div className="studio-input-field__label">
                      <strong>默认 Max Tokens</strong>
                    </div>
                    <input
                      type="number"
                      min="1"
                      value={modelMaxTokensDraft}
                      placeholder="8192"
                      onChange={(ev) => setModelMaxTokensDraft(ev.target.value)}
                    />
                  </label>
                </div>

                <div className="ys-modality-checks">
                  <span>支持输入模态：</span>
                  {INPUT_MODALITIES.map((mod) => {
                    const checked = modelAcceptsDraft.includes(mod);
                    return (
                      <label key={mod} className="ys-check-pill">
                        <input
                          type="checkbox"
                          checked={checked}
                          onChange={(ev) => {
                            if (ev.target.checked) {
                              setModelAcceptsDraft((prev) => [...new Set([...prev, mod])]);
                            } else {
                              setModelAcceptsDraft((prev) => prev.filter((x) => x !== mod));
                            }
                          }}
                        />
                        <span>{mod}</span>
                      </label>
                    );
                  })}
                </div>

                <div className="ys-modality-checks">
                  <label className="ys-check-pill">
                    <input
                      type="checkbox"
                      checked={modelToolsDraft}
                      onChange={(ev) => setModelToolsDraft(ev.target.checked)}
                    />
                    <span>支持原生 Tool Calling</span>
                  </label>
                  <label className="ys-check-pill">
                    <input
                      type="checkbox"
                      checked={modelThinkingDraft}
                      onChange={(ev) => setModelThinkingDraft(ev.target.checked)}
                    />
                    <span>支持深度思考 (Thinking)</span>
                  </label>
                </div>

                <div className="ys-model-form__actions">
                  <button
                    type="button"
                    data-testid="model-save-btn"
                    className="studio-btn-primary"
                    disabled={saveBusy}
                    onClick={() => void handleSaveModel()}
                  >
                    {saveBusy ? '保存中…' : '💾 保存模型并热生效'}
                  </button>
                  {!creatingModel && editingModelName ? (() => {
                    const current = snapshot?.models.find((m) => m.name === editingModelName);
                    if (!current || (!current.is_custom && !current.is_overridden)) return null;
                    return (
                      <button
                        type="button"
                        data-testid="model-delete-btn"
                        className="studio-btn-danger-ghost"
                        disabled={saveBusy}
                        onClick={() => void handleDeleteOrResetModel(current.name, current.is_custom)}
                      >
                        {current.is_custom ? '删除自定义模型' : '恢复内置默认'}
                      </button>
                    );
                  })() : null}
                </div>
              </>
            )}
          </section>
        </div>
      )}
    </ExpandableDrawer>
  );
}
