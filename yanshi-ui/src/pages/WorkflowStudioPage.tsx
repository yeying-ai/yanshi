import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { NodeIoInspector } from '../components/NodeIoInspector';
import { AuthorizedMedia } from '../components/AuthorizedMedia';
import { YamlCodeEditor } from '../components/YamlCodeEditor';
import {
  WorkflowGraphCanvas,
  type NodeRuntimeTelemetry,
} from '../components/WorkflowGraphCanvas';
import {
  apiFetch,
  getApiBaseUrl,
  isTauriRuntime,
  parseSseChunk,
  setApiBaseUrl,
} from '../lib/api';
import { formatTime, isTerminalRun, reconnectDelayMs, runFromEnvelope } from '../lib/helpers';
import {
  apiBaseUrlKey,
  readStorage,
  studioModeKey,
  studioPresetKey,
  tenantKey,
  writeStorage,
} from '../lib/storage';
import {
  CancelRunEnvelopeSchema,
  CreateRunEnvelopeSchema,
  DebugStatusResponseSchema,
  InspectWorkflowResponseSchema,
  ListArtifactsResponseSchema,
  ListRunsEnvelopeSchema,
  RunEnvelopeSchema,
  StreamedEventArraySchema,
  StreamedEventSchema,
  parseJsonResponse,
  type ArtifactSummary,
  type FlowDebugPause,
  type InspectedInput,
  type InspectWorkflowResponse,
  type RunRecord,
  type StreamedEvent,
} from '../schemas';
import { collectRunDeliverables } from '../studio/mediaExtractor';
import { inspectYamlLocally, WORKFLOW_PRESETS } from '../studio/presets';

export type StudioPosture = 'debug' | 'release';
type MobilePane = 'config' | 'canvas' | 'inspector' | 'deliverables';
type StreamState = 'idle' | 'loading' | 'streaming' | 'reconnecting' | 'closed' | 'error';

export interface WorkflowStudioPageProps {
  initialMode?: StudioPosture;
  apiToken: string;
  onTokenChange: (token: string) => void;
}

function defaultInputString(spec: InspectedInput): string {
  if (spec.default === undefined || spec.default === null) {
    if (spec.kind === 'boolean') return 'false';
    if (spec.kind === 'json') return '{}';
    return '';
  }
  if (spec.kind === 'json') {
    try {
      return JSON.stringify(spec.default, null, 2);
    } catch {
      return String(spec.default);
    }
  }
  return String(spec.default);
}

function coerceInputsPayload(
  specs: InspectedInput[],
  draftValues: Record<string, string>,
): { payload: Record<string, unknown>; error: string | null } {
  const payload: Record<string, unknown> = {};
  for (const spec of specs) {
    const raw = draftValues[spec.name] ?? defaultInputString(spec);
    if (spec.kind === 'boolean') {
      payload[spec.name] = raw === 'true';
    } else if (spec.kind === 'number') {
      const trimmed = raw.trim();
      if (!trimmed) {
        if (spec.required) {
          return { payload, error: `必填数值参数「${spec.name}」不能为空` };
        }
        continue;
      }
      const num = Number(trimmed);
      if (Number.isNaN(num)) {
        return { payload, error: `参数「${spec.name}」需要合法数字，当前为 "${raw}"` };
      }
      payload[spec.name] = num;
    } else if (spec.kind === 'json') {
      const trimmed = raw.trim();
      if (!trimmed) {
        continue;
      }
      try {
        payload[spec.name] = JSON.parse(trimmed);
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return { payload, error: `参数「${spec.name}」JSON 解析失败：${msg}` };
      }
    } else {
      if (spec.required && !raw.trim()) {
        return { payload, error: `必填参数「${spec.name}」不能为空` };
      }
      payload[spec.name] = raw;
    }
  }
  return { payload, error: null };
}

function reduceWorkflowEvents(events: StreamedEvent[]): {
  telemetryByNode: Record<string, NodeRuntimeTelemetry>;
  nodeInputs: Record<string, unknown>;
  nodeOutputs: Record<string, unknown>;
  pausedAt: FlowDebugPause | null;
  workflowError: string | null;
  workflowDurationMs: number | null;
} {
  const telemetryByNode: Record<string, NodeRuntimeTelemetry> = {};
  const nodeInputs: Record<string, unknown> = {};
  const nodeOutputs: Record<string, unknown> = {};
  let pausedAt: FlowDebugPause | null = null;
  let workflowError: string | null = null;
  let workflowDurationMs: number | null = null;

  const ensureNode = (nodeId: string): NodeRuntimeTelemetry => {
    if (!telemetryByNode[nodeId]) {
      telemetryByNode[nodeId] = { status: 'idle' };
    }
    return telemetryByNode[nodeId];
  };

  for (const ev of events) {
    const p = (ev.payload as Record<string, unknown>) ?? {};
    const nodeId = typeof p.node_id === 'string' ? p.node_id : null;

    switch (ev.kind) {
      case 'node.started': {
        if (nodeId) {
          const entry = ensureNode(nodeId);
          entry.status = 'running';
          entry.error = undefined;
        }
        break;
      }
      case 'node.input.captured': {
        if (nodeId) {
          nodeInputs[nodeId] = p.input;
          const entry = ensureNode(nodeId);
          entry.hasInput = true;
        }
        break;
      }
      case 'node.output.captured': {
        if (nodeId) {
          nodeOutputs[nodeId] = p.output;
          const entry = ensureNode(nodeId);
          entry.hasOutput = true;
        }
        break;
      }
      case 'node.completed': {
        if (nodeId) {
          const entry = ensureNode(nodeId);
          entry.status = 'succeeded';
          if (typeof p.duration_ms === 'number') {
            entry.durationMs = p.duration_ms;
          }
        }
        break;
      }
      case 'node.skipped': {
        if (nodeId) {
          const entry = ensureNode(nodeId);
          entry.status = 'skipped';
          if (typeof p.reason === 'string') {
            entry.skipReason = p.reason;
          }
        }
        break;
      }
      case 'node.failed': {
        if (nodeId) {
          const entry = ensureNode(nodeId);
          entry.status = 'failed';
          if (typeof p.error === 'string') {
            entry.error = p.error;
          }
          if (typeof p.duration_ms === 'number') {
            entry.durationMs = p.duration_ms;
          }
        }
        break;
      }
      case 'node.debug.paused': {
        if (nodeId) {
          const phase = p.phase === 'before_node' ? 'before_node' : 'after_node';
          const nextNodes = Array.isArray(p.next_nodes)
            ? p.next_nodes.filter((x): x is string => typeof x === 'string')
            : [];
          const entry = ensureNode(nodeId);
          entry.status = phase === 'before_node' ? 'paused_before' : 'paused_after';
          pausedAt = {
            node_id: nodeId,
            phase,
            next_nodes: nextNodes,
          };
        }
        break;
      }
      case 'node.debug.resumed': {
        if (nodeId) {
          const entry = ensureNode(nodeId);
          entry.status = entry.status === 'paused_before' ? 'running' : 'succeeded';
          if (pausedAt?.node_id === nodeId) {
            pausedAt = null;
          }
        } else {
          pausedAt = null;
        }
        break;
      }
      case 'workflow.completed': {
        pausedAt = null;
        if (typeof p.duration_ms === 'number') {
          workflowDurationMs = p.duration_ms;
        }
        break;
      }
      case 'workflow.failed': {
        pausedAt = null;
        if (typeof p.error === 'string') {
          workflowError = p.error;
        }
        if (typeof p.duration_ms === 'number') {
          workflowDurationMs = p.duration_ms;
        }
        break;
      }
      case 'workflow.cancelled': {
        pausedAt = null;
        break;
      }
      default:
        break;
    }
  }

  return {
    telemetryByNode,
    nodeInputs,
    nodeOutputs,
    pausedAt,
    workflowError,
    workflowDurationMs,
  };
}

export function WorkflowStudioPage({
  initialMode,
  apiToken,
  onTokenChange,
}: WorkflowStudioPageProps): React.ReactElement {
  const [mode, setMode] = useState<StudioPosture>(() => {
    if (initialMode) return initialMode;
    const saved = readStorage(studioModeKey, 'debug');
    return saved === 'release' ? 'release' : 'debug';
  });

  const [selectedPresetId, setSelectedPresetId] = useState<string>(() =>
    readStorage(studioPresetKey, WORKFLOW_PRESETS[0].id),
  );
  const [yamlDraft, setYamlDraft] = useState<string>(() => {
    const presetId = readStorage(studioPresetKey, WORKFLOW_PRESETS[0].id);
    const found = WORKFLOW_PRESETS.find((p) => p.id === presetId) ?? WORKFLOW_PRESETS[0];
    return found.yaml;
  });

  const [inspected, setInspected] = useState<InspectWorkflowResponse>(() =>
    inspectYamlLocally(yamlDraft),
  );
  const [inspectSource, setInspectSource] = useState<'local' | 'server'>('local');
  const [inputDrafts, setInputDrafts] = useState<Record<string, string>>({});

  const [tenantId, setTenantId] = useState<string>(() => readStorage(tenantKey, 'default'));
  const [customBaseUrl, setCustomBaseUrl] = useState<string>(() =>
    readStorage(apiBaseUrlKey, ''),
  );
  const [showConnPopover, setShowConnPopover] = useState<boolean>(false);
  const [leftCollapsed, setLeftCollapsed] = useState<boolean>(false);
  const [showYamlEditor, setShowYamlEditor] = useState<boolean>(true);
  const [yamlModalOpen, setYamlModalOpen] = useState<boolean>(false);
  const [mobilePane, setMobilePane] = useState<MobilePane>('canvas');

  // Debug & run state
  const [stepEveryNode, setStepEveryNode] = useState<boolean>(true);
  const [breakpoints, setBreakpoints] = useState<string[]>([]);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  const [runId, setRunId] = useState<string>('');
  const [activeRun, setActiveRun] = useState<RunRecord | null>(null);
  const [recentRuns, setRecentRuns] = useState<RunRecord[]>([]);
  const [events, setEvents] = useState<StreamedEvent[]>([]);
  const [artifacts, setArtifacts] = useState<ArtifactSummary[]>([]);
  const [serverDebugPaused, setServerDebugPaused] = useState<FlowDebugPause | null>(null);

  const [streamState, setStreamState] = useState<StreamState>('idle');
  const [submitBusy, setSubmitBusy] = useState<boolean>(false);
  const [debugActionBusy, setDebugActionBusy] = useState<boolean>(false);
  const [bannerError, setBannerError] = useState<string | null>(null);

  const abortRef = useRef<AbortController | null>(null);
  const latestSeqRef = useRef<number>(-1);
  const reconnectAttemptRef = useRef<number>(0);
  const lastAutoFocusedPauseRef = useRef<string>('');

  // Sync input defaults whenever `inspected.inputs` introduces new keys.
  useEffect(() => {
    setInputDrafts((prev) => {
      const next = { ...prev };
      let changed = false;
      for (const spec of inspected.inputs) {
        if (!(spec.name in next)) {
          next[spec.name] = defaultInputString(spec);
          changed = true;
        }
      }
      return changed ? next : prev;
    });
    setSelectedNodeId((prev) => {
      if (prev && inspected.nodes.some((n) => n.id === prev)) {
        return prev;
      }
      return inspected.execution_order[0] ?? inspected.nodes[0]?.id ?? null;
    });
  }, [inspected]);

  // Persist posture & tenant preferences.
  useEffect(() => {
    writeStorage(studioModeKey, mode);
  }, [mode]);

  useEffect(() => {
    writeStorage(tenantKey, tenantId);
  }, [tenantId]);

  // Debounced `/v1/workflows/inspect` call with immediate local fallback.
  useEffect(() => {
    const optimistic = inspectYamlLocally(yamlDraft);
    setInspected(optimistic);
    setInspectSource('local');

    let cancelled = false;
    const timer = window.setTimeout(async () => {
      try {
        const res = await apiFetch(
          '/v1/workflows/inspect',
          apiToken,
          {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ workflow: yamlDraft }),
          },
          tenantId,
        );
        if (!res.ok || cancelled) {
          return;
        }
        const parsed = await parseJsonResponse(
          InspectWorkflowResponseSchema,
          res,
          'POST /v1/workflows/inspect',
        );
        if (!cancelled) {
          setInspected(parsed);
          setInspectSource('server');
        }
      } catch {
        // Offline / Mobile before gateway configured: local inspection stays active.
      }
    }, 260);

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [yamlDraft, apiToken, tenantId, customBaseUrl]);

  // Read `?run=<uuid>` deep link on mount (also satisfies `/ui/runs/new` -> `/ui?run=<uuid>`).
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const initialRun = params.get('run');
    if (initialRun) {
      setRunId(initialRun);
      setStreamState('loading');
    }
  }, []);

  const loadRecentRuns = useCallback(async () => {
    try {
      const res = await apiFetch(
        `/v1/runs?tenant_id=${encodeURIComponent(tenantId.trim() || 'default')}&limit=10`,
        apiToken,
        {},
        tenantId,
      );
      if (!res.ok) return;
      const list = await parseJsonResponse(ListRunsEnvelopeSchema, res, 'GET /v1/runs');
      setRecentRuns(list.runs);
    } catch {
      // Non-fatal when gateway is not connected yet.
    }
  }, [apiToken, tenantId]);

  useEffect(() => {
    void loadRecentRuns();
  }, [loadRecentRuns, customBaseUrl]);

  const loadRunArtifacts = useCallback(
    async (targetRunId: string) => {
      if (!targetRunId) return;
      try {
        const res = await apiFetch(
          `/v1/runs/${encodeURIComponent(targetRunId)}/artifacts`,
          apiToken,
          {},
          tenantId,
        );
        if (!res.ok) return;
        const parsed = await parseJsonResponse(
          ListArtifactsResponseSchema,
          res,
          `GET /v1/runs/${targetRunId}/artifacts`,
        );
        setArtifacts(parsed.artifacts);
      } catch {
        // Artifacts are optional; live node outputs also populate deliverables.
      }
    },
    [apiToken, tenantId],
  );

  // Connect and stream SSE events whenever `runId` enters `'loading'`.
  useEffect(() => {
    if (!runId || streamState !== 'loading') {
      return undefined;
    }

    let cancelled = false;
    let reconnectTimer: number | undefined;
    abortRef.current?.abort();
    latestSeqRef.current = -1;
    reconnectAttemptRef.current = 0;

    const appendEvent = (ev: StreamedEvent) => {
      setEvents((curr) => {
        if (curr.some((item) => item.seq === ev.seq)) {
          return curr;
        }
        return [...curr, ev].sort((a, b) => a.seq - b.seq);
      });
      if (ev.seq > latestSeqRef.current) {
        latestSeqRef.current = ev.seq;
      }
      if (ev.kind === 'node.debug.paused') {
        setServerDebugPaused(null);
      }
    };

    const connectStream = async (afterSeq: number) => {
      const controller = new AbortController();
      abortRef.current = controller;
      const res = await apiFetch(
        `/v1/runs/${encodeURIComponent(runId)}/events?after_seq=${encodeURIComponent(String(afterSeq))}`,
        apiToken,
        { signal: controller.signal },
        tenantId,
      );
      if (!res.ok || !res.body) {
        throw new Error(`SSE 事件流连接失败 (HTTP ${res.status})`);
      }
      setStreamState('streaming');
      reconnectAttemptRef.current = 0;
      const reader = res.body.pipeThrough(new TextDecoderStream()).getReader();
      let buffer = '';
      while (!cancelled) {
        const { done, value } = await reader.read();
        if (done) break;
        buffer += value;
        const parsed = parseSseChunk(buffer, StreamedEventSchema, 'studio SSE');
        buffer = parsed.rest;
        for (const item of parsed.events) {
          appendEvent(item);
        }
      }
    };

    const scheduleReconnect = () => {
      if (cancelled) return;
      const attempt = reconnectAttemptRef.current;
      reconnectAttemptRef.current = attempt + 1;
      setStreamState('reconnecting');
      reconnectTimer = window.setTimeout(() => {
        if (cancelled) return;
        void connectStream(latestSeqRef.current).catch((err) => {
          if (cancelled || (err instanceof DOMException && err.name === 'AbortError')) {
            return;
          }
          setBannerError(err instanceof Error ? err.message : String(err));
          scheduleReconnect();
        });
      }, reconnectDelayMs(attempt));
    };

    const bootstrap = async () => {
      try {
        const runRes = await apiFetch(
          `/v1/runs/${encodeURIComponent(runId)}`,
          apiToken,
          {},
          tenantId,
        );
        if (!runRes.ok) {
          throw new Error(`查询运行记录失败 (HTTP ${runRes.status})`);
        }
        const envelope = await parseJsonResponse(
          RunEnvelopeSchema,
          runRes,
          `GET /v1/runs/${runId}`,
        );
        if (cancelled) return;
        const record = runFromEnvelope(envelope);
        setActiveRun(record);
        if (record.workflow && record.workflow.trim()) {
          setYamlDraft(record.workflow);
        }
        setEvents([]);
        setArtifacts([]);
        setBannerError(null);

        // Preserve `/ui?run=<uuid>` when already under `/ui`, or keep `/ui/app?run=...` / `/ui/studio?run=...`.
        const currentPath = window.location.pathname || '/ui';
        window.history.replaceState(
          null,
          '',
          `${currentPath}?run=${encodeURIComponent(runId)}`,
        );

        // Best-effort debug snapshot check in case we reattach while paused.
        void apiFetch(`/v1/runs/${encodeURIComponent(runId)}/debug`, apiToken, {}, tenantId)
          .then((r) => (r.ok ? parseJsonResponse(DebugStatusResponseSchema, r, 'GET debug') : null))
          .then((dbg) => {
            if (!cancelled && dbg?.active && dbg.snapshot) {
              setBreakpoints(dbg.snapshot.breakpoints);
              setStepEveryNode(dbg.snapshot.step_every_node);
              setServerDebugPaused(dbg.snapshot.paused_at ?? null);
            }
          })
          .catch(() => {});

        let afterSeq = -1;
        const histRes = await apiFetch(
          `/v1/runs/${encodeURIComponent(runId)}/events/history`,
          apiToken,
          {},
          tenantId,
        );
        if (histRes.ok) {
          const hist = await parseJsonResponse(
            StreamedEventArraySchema,
            histRes,
            'GET events/history',
          );
          if (!cancelled) {
            setEvents(hist);
            afterSeq = hist.at(-1)?.seq ?? -1;
            latestSeqRef.current = afterSeq;
          }
        }

        if (isTerminalRun(record)) {
          setStreamState('closed');
          await loadRunArtifacts(runId);
          return;
        }

        await connectStream(afterSeq);
        if (!cancelled) {
          setStreamState('closed');
          setServerDebugPaused(null);
          const finalRes = await apiFetch(
            `/v1/runs/${encodeURIComponent(runId)}`,
            apiToken,
            {},
            tenantId,
          );
          if (finalRes.ok) {
            const finalEnv = await parseJsonResponse(
              RunEnvelopeSchema,
              finalRes,
              'GET final run',
            );
            setActiveRun(runFromEnvelope(finalEnv));
          }
          await loadRunArtifacts(runId);
          void loadRecentRuns();
        }
      } catch (err) {
        if (cancelled || (err instanceof DOMException && err.name === 'AbortError')) {
          return;
        }
        setBannerError(err instanceof Error ? err.message : String(err));
        scheduleReconnect();
      }
    };

    void bootstrap();

    return () => {
      cancelled = true;
      abortRef.current?.abort();
      if (reconnectTimer) {
        window.clearTimeout(reconnectTimer);
      }
    };
  }, [runId, streamState, apiToken, tenantId, loadRunArtifacts, loadRecentRuns]);

  const reduced = useMemo(() => reduceWorkflowEvents(events), [events]);
  const effectivePausedAt: FlowDebugPause | null = reduced.pausedAt ?? serverDebugPaused;

  // Auto-focus paused node in Debug Mode so I/O is immediately visible.
  useEffect(() => {
    if (!effectivePausedAt) return;
    const pauseSignature = `${runId}:${effectivePausedAt.node_id}:${effectivePausedAt.phase}:${events.length}`;
    if (lastAutoFocusedPauseRef.current !== pauseSignature) {
      lastAutoFocusedPauseRef.current = pauseSignature;
      setSelectedNodeId(effectivePausedAt.node_id);
    }
  }, [effectivePausedAt, runId, events.length]);

  const selectedNode = useMemo(
    () => inspected.nodes.find((n) => n.id === selectedNodeId) ?? null,
    [inspected.nodes, selectedNodeId],
  );

  const deliverables = useMemo(
    () => (runId ? collectRunDeliverables(runId, artifacts, reduced.nodeOutputs) : []),
    [runId, artifacts, reduced.nodeOutputs],
  );

  // Extract final text/markdown summary when a non-media workflow finishes.
  const finalTextSummary = useMemo(() => {
    if (inspected.execution_order.length === 0) return null;
    for (let i = inspected.execution_order.length - 1; i >= 0; i -= 1) {
      const nid = inspected.execution_order[i];
      const out = reduced.nodeOutputs[nid];
      if (!out || typeof out !== 'object') continue;
      const rec = out as Record<string, unknown>;
      if (typeof rec.output === 'string' && rec.output.trim()) {
        return { nodeId: nid, text: rec.output };
      }
    }
    return null;
  }, [inspected.execution_order, reduced.nodeOutputs]);

  const completedNodeCount = useMemo(() => {
    return inspected.nodes.filter((n) => {
      const st = reduced.telemetryByNode[n.id]?.status;
      return st === 'succeeded' || st === 'skipped' || st === 'paused_after';
    }).length;
  }, [inspected.nodes, reduced.telemetryByNode]);

  const isRunActive = Boolean(
    runId &&
      (streamState === 'loading' ||
        streamState === 'streaming' ||
        streamState === 'reconnecting') &&
      (!activeRun || !isTerminalRun(activeRun)),
  );

  const switchMode = (nextMode: StudioPosture) => {
    setMode(nextMode);
    const targetPath = nextMode === 'release' ? '/ui/app' : '/ui/studio';
    const suffix = runId ? `?run=${encodeURIComponent(runId)}` : '';
    window.history.replaceState(null, '', `${targetPath}${suffix}`);
    if (nextMode === 'release') {
      setMobilePane(runId ? 'deliverables' : 'config');
    } else {
      setMobilePane('canvas');
    }
  };

  const selectPreset = (presetId: string) => {
    const preset = WORKFLOW_PRESETS.find((p) => p.id === presetId);
    if (!preset) return;
    setSelectedPresetId(preset.id);
    writeStorage(studioPresetKey, preset.id);
    setYamlDraft(preset.yaml);
    setInputDrafts({});
    setBreakpoints([]);
  };

  const toggleBreakpoint = async (nodeId: string) => {
    const next = breakpoints.includes(nodeId)
      ? breakpoints.filter((b) => b !== nodeId)
      : [...breakpoints, nodeId];
    setBreakpoints(next);

    if (runId && isRunActive) {
      try {
        await apiFetch(
          `/v1/runs/${encodeURIComponent(runId)}:debug`,
          apiToken,
          {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              action: 'set_breakpoints',
              breakpoints: next,
              step_every_node: stepEveryNode,
            }),
          },
          tenantId,
        );
      } catch {
        // Local state still applies to next run if current run already finished.
      }
    }
  };

  const toggleStepEveryNode = async (nextVal: boolean) => {
    setStepEveryNode(nextVal);
    if (runId && isRunActive) {
      try {
        await apiFetch(
          `/v1/runs/${encodeURIComponent(runId)}:debug`,
          apiToken,
          {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              action: 'set_breakpoints',
              breakpoints,
              step_every_node: nextVal,
            }),
          },
          tenantId,
        );
      } catch {
        // Ignore transient sync error.
      }
    }
  };

  const launchWorkflowRun = async (targetRunMode: StudioPosture) => {
    if (!yamlDraft.trim()) {
      setBannerError('工作流 YAML 不能为空');
      return;
    }
    const coerced = coerceInputsPayload(inspected.inputs, inputDrafts);
    if (coerced.error) {
      setBannerError(coerced.error);
      return;
    }

    setSubmitBusy(true);
    setBannerError(null);
    setServerDebugPaused(null);
    try {
      const res = await apiFetch(
        '/v1/runs',
        apiToken,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            tenant_id: tenantId.trim() || 'default',
            workflow: yamlDraft,
            inputs: coerced.payload,
            run_mode: targetRunMode,
            step_every_node: targetRunMode === 'debug' ? stepEveryNode : false,
            breakpoints: targetRunMode === 'debug' ? breakpoints : [],
          }),
        },
        tenantId,
      );
      if (!res.ok) {
        const detail = await res.text().catch(() => '');
        throw new Error(`启动运行失败 (HTTP ${res.status})${detail ? `: ${detail.slice(0, 180)}` : ''}`);
      }
      const created = await parseJsonResponse(
        CreateRunEnvelopeSchema,
        res,
        'POST /v1/runs (studio)',
      );
      setEvents([]);
      setArtifacts([]);
      setRunId(created.run_id);
      setStreamState('loading');
      if (targetRunMode === 'release') {
        setMobilePane('deliverables');
      } else {
        setMobilePane('canvas');
      }
    } catch (err) {
      setBannerError(err instanceof Error ? err.message : String(err));
    } finally {
      setSubmitBusy(false);
    }
  };

  const sendDebugAction = async (action: 'step' | 'continue') => {
    if (!runId) return;
    setDebugActionBusy(true);
    setBannerError(null);
    try {
      const res = await apiFetch(
        `/v1/runs/${encodeURIComponent(runId)}:debug`,
        apiToken,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ action }),
        },
        tenantId,
      );
      if (!res.ok) {
        throw new Error(`调试指令失败 (HTTP ${res.status})`);
      }
      const parsed = await parseJsonResponse(
        DebugStatusResponseSchema,
        res,
        `POST /v1/runs/${runId}:debug`,
      );
      setServerDebugPaused(parsed.snapshot?.paused_at ?? null);
    } catch (err) {
      setBannerError(err instanceof Error ? err.message : String(err));
    } finally {
      setDebugActionBusy(false);
    }
  };

  const cancelCurrentRun = async () => {
    if (!runId) return;
    setDebugActionBusy(true);
    try {
      const res = await apiFetch(
        `/v1/runs/${encodeURIComponent(runId)}:cancel`,
        apiToken,
        { method: 'POST' },
        tenantId,
      );
      if (res.ok) {
        const parsed = await parseJsonResponse(
          CancelRunEnvelopeSchema,
          res,
          `POST /v1/runs/${runId}:cancel`,
        );
        setActiveRun(parsed.run);
        setServerDebugPaused(null);
        setStreamState('closed');
        abortRef.current?.abort();
      }
    } catch (err) {
      setBannerError(err instanceof Error ? err.message : String(err));
    } finally {
      setDebugActionBusy(false);
    }
  };

  const importLocalYaml = (ev: React.ChangeEvent<HTMLInputElement>) => {
    const file = ev.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      if (typeof reader.result === 'string') {
        setSelectedPresetId('custom');
        setYamlDraft(reader.result);
        setInputDrafts({});
      }
    };
    reader.readAsText(file);
  };

  const activeBase = getApiBaseUrl();
  const connectionBadge = customBaseUrl.trim()
    ? `远端网关 · ${customBaseUrl.trim().replace(/^https?:\/\//, '')}`
    : typeof window !== 'undefined' && window.__YANSHI_API_BASE__
      ? '桌面内置引擎'
      : isTauriRuntime()
        ? '移动端 · 待配网关'
        : 'Web 同源网关';

  const renderTypedInputsForm = (compact: boolean) => {
    if (inspected.inputs.length === 0) {
      return <p className="studio-muted-note">该工作流未声明外部 inputs 参数，可直接启动运行。</p>;
    }
    return (
      <div className={`studio-inputs-grid ${compact ? 'studio-inputs-grid--compact' : ''}`}>
        {inspected.inputs.map((spec) => {
          const val = inputDrafts[spec.name] ?? defaultInputString(spec);
          const isLongText =
            spec.kind === 'string' &&
            (spec.name.includes('source') ||
              spec.name.includes('notes') ||
              spec.name.includes('prompt') ||
              val.length > 72);

          if (spec.kind === 'boolean') {
            const checked = val === 'true';
            return (
              <label key={spec.name} className="studio-bool-field">
                <input
                  type="checkbox"
                  checked={checked}
                  onChange={(ev) =>
                    setInputDrafts((prev) => ({
                      ...prev,
                      [spec.name]: ev.target.checked ? 'true' : 'false',
                    }))
                  }
                />
                <div>
                  <strong>{spec.name}</strong>
                  {spec.description && <small>{spec.description}</small>}
                </div>
              </label>
            );
          }

          return (
            <label key={spec.name} className="studio-input-field">
              <div className="studio-input-field__label">
                <strong>
                  {spec.name}
                  {spec.required ? <em className="studio-req">*</em> : null}
                </strong>
                <span>{spec.kind}</span>
              </div>
              {spec.description && <small className="studio-input-field__desc">{spec.description}</small>}
              {spec.kind === 'json' || isLongText ? (
                <textarea
                  rows={spec.kind === 'json' ? 3 : 2}
                  value={val}
                  onChange={(ev) =>
                    setInputDrafts((prev) => ({ ...prev, [spec.name]: ev.target.value }))
                  }
                />
              ) : (
                <input
                  type={spec.kind === 'number' ? 'number' : 'text'}
                  value={val}
                  onChange={(ev) =>
                    setInputDrafts((prev) => ({ ...prev, [spec.name]: ev.target.value }))
                  }
                />
              )}
            </label>
          );
        })}
      </div>
    );
  };

  const progressPct =
    inspected.nodes.length > 0
      ? Math.min(100, Math.round((completedNodeCount / inspected.nodes.length) * 100))
      : 0;

  return (
    <div className={`studio-shell studio-shell--${mode}`} data-mobile-pane={mobilePane}>
      {/* ── Top Bar ─────────────────────────────────────────────── */}
      <header className="studio-topbar">
        <div className="studio-brand">
          <span className="studio-brand__mark" aria-hidden="true">
            ◈
          </span>
          <div>
            <div className="studio-brand__row">
              <h1>Yanshi Studio</h1>
              <span className="studio-workflow-pill">
                {inspected.name || '未命名工作流'} · {inspected.nodes.length} 节点
              </span>
            </div>
            <p className="studio-brand__sub">
              可视化编排 · 单步 I/O 调试与一键发布运行工作台
            </p>
          </div>
        </div>

        <div
          className="studio-mode-switch"
          role="group"
          aria-label="运行姿态切换（调试状态 / 发布状态）"
        >
          <button
            type="button"
            data-testid="mode-switch-debug"
            className={`studio-mode-btn ${mode === 'debug' ? 'studio-mode-btn--active' : ''}`}
            aria-pressed={mode === 'debug'}
            onClick={() => switchMode('debug')}
          >
            🔍 调试状态
            <small>每步暂停观测 I/O</small>
          </button>
          <button
            type="button"
            data-testid="mode-switch-release"
            className={`studio-mode-btn ${mode === 'release' ? 'studio-mode-btn--active' : ''}`}
            aria-pressed={mode === 'release'}
            onClick={() => switchMode('release')}
          >
            🚀 发布状态
            <small>一键全速运行</small>
          </button>
        </div>

        <div className="studio-topbar__right">
          <div className="studio-conn-anchor">
            <button
              type="button"
              className="studio-btn-ghost studio-btn-sm"
              aria-expanded={showConnPopover}
              onClick={() => setShowConnPopover((v) => !v)}
            >
              <span className={`studio-conn-dot ${activeBase || !isTauriRuntime() ? 'on' : 'warn'}`} />
              {connectionBadge}
            </button>

            {showConnPopover && (
              <div className="studio-conn-popover" role="dialog" aria-label="网关与凭据配置">
                <label>
                  <span>网关地址 API Base URL（留空使用当前引擎）</span>
                  <input
                    type="url"
                    placeholder="例如 http://192.168.1.10:8080"
                    value={customBaseUrl}
                    onChange={(ev) => {
                      setCustomBaseUrl(ev.target.value);
                      setApiBaseUrl(ev.target.value);
                    }}
                  />
                </label>
                <label>
                  <span>租户 Tenant ID</span>
                  <input
                    type="text"
                    value={tenantId}
                    onChange={(ev) => setTenantId(ev.target.value)}
                  />
                </label>
                <label>
                  <span>Bearer Token（仅保存在当前会话）</span>
                  <input
                    type="password"
                    autoComplete="off"
                    placeholder="可选鉴权令牌"
                    value={apiToken}
                    onChange={(ev) => onTokenChange(ev.target.value)}
                  />
                </label>
                <div className="studio-conn-popover__foot">
                  <small>
                    当前生效：{activeBase || '同源 /ui 网关'}（校验源：{inspectSource === 'server' ? 'Rust 引擎' : '本地解析'}）
                  </small>
                  <button
                    type="button"
                    className="studio-btn-primary studio-btn-sm"
                    onClick={() => setShowConnPopover(false)}
                  >
                    完成
                  </button>
                </div>
              </div>
            )}
          </div>

          <nav className="studio-aux-links" aria-label="辅助控制台导航">
            <a href="/ui/console">经典事件台</a>
            <a href="/ui/harness/sessions">Harness</a>
            <a href="/ui/diagnostics">诊断</a>
          </nav>
        </div>
      </header>

      {bannerError && (
        <div className="studio-alert studio-alert--error" role="alert">
          <span>{bannerError}</span>
          <button type="button" onClick={() => setBannerError(null)}>
            关闭
          </button>
        </div>
      )}

      {!inspected.diagnostics.valid && inspected.diagnostics.issues.length > 0 && (
        <div className="studio-alert studio-alert--warn" role="alert">
          <span>工作流校验问题：{inspected.diagnostics.issues.join('；')}</span>
        </div>
      )}

      {/* ── Posture 1: DEBUG MODE (Studio) ──────────────────────── */}
      {mode === 'debug' ? (
        <main className={`studio-debug-grid ${leftCollapsed ? 'studio-debug-grid--collapsed' : ''}`}>
          <aside className="studio-pane studio-pane--config" aria-label="工作流预设与输入配置">
            <div className="studio-pane__head">
              <strong>工作流与输入配置</strong>
              <button
                type="button"
                className="studio-btn-ghost studio-btn-sm studio-desktop-only"
                onClick={() => setLeftCollapsed((v) => !v)}
              >
                {leftCollapsed ? '展开' : '收起'}
              </button>
            </div>

            {!leftCollapsed && (
              <div className="studio-pane__scroll">
                <div className="studio-section">
                  <div className="studio-section__head">
                    <span>预设工作流模板</span>
                    <label className="studio-upload-btn">
                      导入 YAML
                      <input type="file" accept=".yml,.yaml" onChange={importLocalYaml} />
                    </label>
                  </div>
                  <div className="studio-preset-list">
                    {WORKFLOW_PRESETS.map((preset) => (
                      <button
                        key={preset.id}
                        type="button"
                        className={`studio-preset-card ${
                          selectedPresetId === preset.id ? 'studio-preset-card--active' : ''
                        }`}
                        onClick={() => selectPreset(preset.id)}
                      >
                        <div className="studio-preset-card__top">
                          <strong>{preset.title}</strong>
                          <span>{preset.badge}</span>
                        </div>
                        <small>{preset.subtitle}</small>
                      </button>
                    ))}
                  </div>
                </div>

                <div className="studio-section">
                  <div className="studio-section__head">
                    <span>调试输入参数 ({inspected.inputs.length})</span>
                  </div>
                  {renderTypedInputsForm(true)}
                </div>

                <div className="studio-section">
                  <div className="studio-section__head">
                    <span>YAML 编排源码</span>
                    <button
                      type="button"
                      className="studio-link-btn"
                      onClick={() => setShowYamlEditor((v) => !v)}
                    >
                      {showYamlEditor ? '收起编辑器' : '展开编辑 YAML'}
                    </button>
                  </div>
                  {showYamlEditor && (
                    <YamlCodeEditor
                      testId="studio-yaml-editor"
                      ariaLabel="YAML 编排源码"
                      value={yamlDraft}
                      minHeight={280}
                      onExpandToggle={() => setYamlModalOpen(true)}
                      onChange={(nextValue) => {
                        setSelectedPresetId('custom');
                        setYamlDraft(nextValue);
                      }}
                    />
                  )}
                </div>

                {recentRuns.length > 0 && (
                  <div className="studio-section">
                    <div className="studio-section__head">
                      <span>最近运行记录</span>
                    </div>
                    <ul className="studio-recent-runs">
                      {recentRuns.slice(0, 6).map((r) => (
                        <li key={r.id}>
                          <button
                            type="button"
                            className={r.id === runId ? 'active' : ''}
                            onClick={() => {
                              setRunId(r.id);
                              setStreamState('loading');
                            }}
                          >
                            <span>{r.id.slice(0, 8)}</span>
                            <em>{r.status}</em>
                            <small>{formatTime(r.started_at)}</small>
                          </button>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </div>
            )}
          </aside>

          <section className="studio-pane studio-pane--canvas" aria-label="可视化 DAG 调试画布">
            <div className="studio-debug-deck" role="toolbar" aria-label="单步调试传输控台">
              <div className="studio-debug-deck__primary">
                {leftCollapsed && (
                  <button
                    type="button"
                    className="studio-btn-ghost studio-btn-sm studio-desktop-only"
                    onClick={() => setLeftCollapsed(false)}
                  >
                    展开左栏
                  </button>
                )}

                <button
                  type="button"
                  data-testid="launch-debug-run"
                  className="studio-btn-primary"
                  disabled={submitBusy || !inspected.diagnostics.valid}
                  onClick={() => void launchWorkflowRun('debug')}
                >
                  {submitBusy ? '启动中…' : '🔍 启动单步调试'}
                </button>

                <label className="studio-step-toggle" title="开启后每完成一个节点自动挂起，等待您观测完输入与输出后再进入下一步">
                  <input
                    type="checkbox"
                    checked={stepEveryNode}
                    onChange={(ev) => void toggleStepEveryNode(ev.target.checked)}
                  />
                  <span>每步暂停观测</span>
                </label>

                <div className="studio-deck-divider" />

                <button
                  type="button"
                  data-testid="debug-step-next"
                  className="studio-btn-step"
                  disabled={!effectivePausedAt || debugActionBusy}
                  onClick={() => void sendDebugAction('step')}
                >
                  ▶ 单步下一步
                </button>

                <button
                  type="button"
                  data-testid="debug-continue-run"
                  className="studio-btn-secondary"
                  disabled={!effectivePausedAt || debugActionBusy}
                  onClick={() => void sendDebugAction('continue')}
                >
                  ⏭ 继续跑完
                </button>

                <button
                  type="button"
                  className="studio-btn-danger-ghost"
                  disabled={!isRunActive || debugActionBusy}
                  onClick={() => void cancelCurrentRun()}
                >
                  ⏹ 终止
                </button>
              </div>

              <div className="studio-debug-deck__status">
                {effectivePausedAt ? (
                  <span className="studio-live-pill studio-live-pill--paused">
                    ⏸ 已暂停于 #{effectivePausedAt.node_id}（
                    {effectivePausedAt.phase === 'before_node' ? '执行前断点' : '本步已完成'}）
                  </span>
                ) : isRunActive ? (
                  <span className="studio-live-pill studio-live-pill--running">
                    ● 运行中 · {completedNodeCount}/{inspected.nodes.length} 步
                  </span>
                ) : activeRun ? (
                  <span className="studio-live-pill studio-live-pill--done">
                    状态：{activeRun.status} · {completedNodeCount}/{inspected.nodes.length} 步
                    {typeof reduced.workflowDurationMs === 'number'
                      ? ` (${(reduced.workflowDurationMs / 1000).toFixed(2)}s)`
                      : ''}
                  </span>
                ) : (
                  <span className="studio-live-pill">
                    就绪 · 共 {inspected.levels.length} 层拓扑阶段
                  </span>
                )}
              </div>
            </div>

            <WorkflowGraphCanvas
              nodes={inspected.nodes}
              edges={inspected.edges}
              levels={inspected.levels}
              executionOrder={inspected.execution_order}
              telemetryByNode={reduced.telemetryByNode}
              breakpoints={breakpoints}
              selectedNodeId={selectedNodeId}
              onSelectNode={(nid) => {
                setSelectedNodeId(nid);
                if (window.innerWidth < 860) {
                  setMobilePane('inspector');
                }
              }}
              onToggleBreakpoint={(nid) => void toggleBreakpoint(nid)}
            />
          </section>

          <aside className="studio-pane studio-pane--inspector">
            <NodeIoInspector
              runId={runId}
              apiToken={apiToken}
              tenantId={tenantId}
              node={selectedNode}
              telemetry={selectedNode ? reduced.telemetryByNode[selectedNode.id] : undefined}
              capturedInput={selectedNode ? reduced.nodeInputs[selectedNode.id] : undefined}
              capturedOutput={selectedNode ? reduced.nodeOutputs[selectedNode.id] : undefined}
              pausedAt={effectivePausedAt}
              hasBreakpoint={selectedNode ? breakpoints.includes(selectedNode.id) : false}
              onToggleBreakpoint={(nid) => void toggleBreakpoint(nid)}
              onStepDebug={() => void sendDebugAction('step')}
              onContinueDebug={() => void sendDebugAction('continue')}
              debugActionBusy={debugActionBusy}
            />
          </aside>

          {/* Mobile-only Deliverables view in Debug mode */}
          <section className="studio-pane studio-pane--mobile-deliverables">
            <div className="studio-pane__head">
              <strong>本轮产出预览 ({deliverables.length})</strong>
            </div>
            <div className="studio-pane__scroll">
              {deliverables.length === 0 && !finalTextSummary ? (
                <p className="studio-muted-note">运行产生文件或文本结果后将在此汇总展示。</p>
              ) : (
                <div className="studio-media-list">
                  {deliverables.map((item) => (
                    <AuthorizedMedia
                      key={item.key}
                      item={item}
                      apiToken={apiToken}
                      tenantId={tenantId}
                    />
                  ))}
                  {finalTextSummary && (
                    <article className="studio-text-deliverable">
                      <header>
                        <strong>文本汇总产出（#{finalTextSummary.nodeId}）</strong>
                      </header>
                      <pre>{finalTextSummary.text}</pre>
                    </article>
                  )}
                </div>
              )}
            </div>
          </section>
        </main>
      ) : (
        /* ── Posture 2: RELEASE MODE (One-Click App Runner) ──────── */
        <main className="studio-release-grid">
          <section className="studio-release-form-card" aria-label="应用运行参数配置">
            <div className="studio-release-card__head">
              <div>
                <span className="studio-kicker">RELEASE APP · 一键全速运行</span>
                <h2>{inspected.name || '工作流应用'}</h2>
              </div>
              <select
                aria-label="选择应用工作流"
                className="studio-preset-select"
                value={selectedPresetId}
                onChange={(ev) => selectPreset(ev.target.value)}
              >
                {WORKFLOW_PRESETS.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.title}
                  </option>
                ))}
                {selectedPresetId === 'custom' && <option value="custom">自定义导入 YAML</option>}
              </select>
            </div>

            <div className="studio-release-form-body">{renderTypedInputsForm(false)}</div>

            <div className="studio-release-cta-bar">
              <button
                type="button"
                data-testid="launch-release-run"
                className="studio-btn-launch"
                disabled={submitBusy || !inspected.diagnostics.valid}
                onClick={() => void launchWorkflowRun('release')}
              >
                {submitBusy
                  ? '正在提交并发任务…'
                  : isRunActive
                    ? '🚀 重新一键运行整个流程'
                    : '🚀 一键运行整个流程'}
              </button>

              {isRunActive && (
                <button
                  type="button"
                  className="studio-btn-danger-ghost"
                  onClick={() => void cancelCurrentRun()}
                >
                  终止当前运行
                </button>
              )}
            </div>
          </section>

          <section className="studio-release-stage-card" aria-label="全流程进度与成果展台">
            <div className="studio-progress-panel">
              <div className="studio-progress-panel__top">
                <div>
                  <strong>
                    {isRunActive
                      ? '正在全速并发执行流水线…'
                      : activeRun?.status === 'succeeded' ||
                          (completedNodeCount > 0 && completedNodeCount === inspected.nodes.length)
                        ? '✓ 全流程执行完成'
                        : reduced.workflowError || activeRun?.status === 'failed'
                          ? '✕ 执行过程中发生异常'
                          : '等待启动'}
                  </strong>
                  <small>
                    已完成 {completedNodeCount} / {inspected.nodes.length} 个阶段
                    {typeof reduced.workflowDurationMs === 'number'
                      ? ` · 总耗时 ${(reduced.workflowDurationMs / 1000).toFixed(2)}s`
                      : ''}
                  </small>
                </div>
                <div className="studio-progress-panel__right">
                  <span className="studio-pct">{progressPct}%</span>
                  {runId && (
                    <button
                      type="button"
                      className="studio-link-btn"
                      onClick={() => switchMode('debug')}
                    >
                      切至调试视图核查节点 I/O →
                    </button>
                  )}
                </div>
              </div>

              <div
                className="studio-progress-bar"
                role="progressbar"
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={progressPct}
              >
                <div className="studio-progress-bar__fill" style={{ width: `${progressPct}%` }} />
              </div>

              <ol className="studio-stage-pills" aria-label="阶段进度列表">
                {inspected.execution_order.map((nid, idx) => {
                  const st = reduced.telemetryByNode[nid]?.status ?? 'idle';
                  return (
                    <li key={nid} className={`studio-stage-pill studio-stage-pill--${st}`}>
                      <span>{idx + 1}</span>
                      <strong>{nid}</strong>
                    </li>
                  );
                })}
              </ol>
            </div>

            {reduced.workflowError && (
              <div className="studio-node-error" role="alert">
                <strong>执行失败详情：</strong> {reduced.workflowError}
              </div>
            )}

            <div className="studio-deliverables-showcase">
              <div className="studio-section__head">
                <span>最终成果展台 ({deliverables.length + (finalTextSummary ? 1 : 0)})</span>
                {runId && <small>Run ID: {runId.slice(0, 12)}…</small>}
              </div>

              {deliverables.length === 0 && !finalTextSummary ? (
                <div className="studio-showcase-placeholder">
                  <p>
                    点击左侧 <strong>「🚀 一键运行整个流程」</strong>，系统将并发调度全部无依赖阶段，并在此自动呈现合成好的
                    1080p 讲解视频、交互式 HTML 幻灯片、PPTX/PDF 课件或文本讲义。
                  </p>
                </div>
              ) : (
                <div className="studio-deliverables-grid">
                  {deliverables.map((item) => (
                    <AuthorizedMedia
                      key={item.key}
                      item={item}
                      apiToken={apiToken}
                      tenantId={tenantId}
                    />
                  ))}

                  {finalTextSummary && (
                    <article className="studio-text-deliverable">
                      <header>
                        <strong>最终讲义 / 文本输出（#{finalTextSummary.nodeId}）</strong>
                      </header>
                      <pre data-testid="release-final-text">{finalTextSummary.text}</pre>
                    </article>
                  )}
                </div>
              )}
            </div>
          </section>
        </main>
      )}

      {/* ── Mobile Safe-Area Bottom Navigation (< 860px) ────────── */}
      <nav className="studio-mobile-nav" aria-label="移动端视图切换">
        {mode === 'debug' ? (
          <>
            <button
              type="button"
              className={mobilePane === 'config' ? 'active' : ''}
              onClick={() => setMobilePane('config')}
            >
              配置
            </button>
            <button
              type="button"
              className={mobilePane === 'canvas' ? 'active' : ''}
              onClick={() => setMobilePane('canvas')}
            >
              拓扑调试
            </button>
            <button
              type="button"
              className={mobilePane === 'inspector' ? 'active' : ''}
              onClick={() => setMobilePane('inspector')}
            >
              I/O 观测
              {effectivePausedAt ? <i className="studio-nav-dot" /> : null}
            </button>
            <button
              type="button"
              className={mobilePane === 'deliverables' ? 'active' : ''}
              onClick={() => setMobilePane('deliverables')}
            >
              成果 ({deliverables.length})
            </button>
          </>
        ) : (
          <>
            <button
              type="button"
              className={mobilePane === 'config' ? 'active' : ''}
              onClick={() => setMobilePane('config')}
            >
              参数配置
            </button>
            <button
              type="button"
              className={mobilePane !== 'config' ? 'active' : ''}
              onClick={() => setMobilePane('deliverables')}
            >
              进度与成果 ({deliverables.length})
            </button>
          </>
        )}
      </nav>

      {yamlModalOpen && (
        <div
          className="studio-yaml-modal-backdrop"
          role="dialog"
          aria-modal="true"
          aria-label="放大编辑工作流 YAML"
          onKeyDown={(ev) => {
            if (ev.key === 'Escape') {
              setYamlModalOpen(false);
            }
          }}
        >
          <div className="studio-yaml-modal">
            <header className="studio-yaml-modal__head">
              <div>
                <strong>YAML 编排源码编辑器</strong>
                <span
                  className={
                    inspected.diagnostics.valid
                      ? 'studio-yaml-modal__badge ok'
                      : 'studio-yaml-modal__badge warn'
                  }
                >
                  {inspected.diagnostics.valid
                    ? `✓ 拓扑有效 · ${inspected.nodes.length} 节点 / ${inspected.edges.length} 边`
                    : `⚠ ${inspected.diagnostics.issues[0] ?? '语法或依赖有误'}`}
                </span>
              </div>
              <button
                type="button"
                className="studio-btn-primary studio-btn-sm"
                onClick={() => setYamlModalOpen(false)}
              >
                完成并关闭 (Esc)
              </button>
            </header>

            <YamlCodeEditor
              ariaLabel="放大编辑工作流 YAML"
              value={yamlDraft}
              isExpanded
              onExpandToggle={() => setYamlModalOpen(false)}
              onChange={(nextValue) => {
                setSelectedPresetId('custom');
                setYamlDraft(nextValue);
              }}
            />
          </div>
        </div>
      )}
    </div>
  );
}
