import YAML, { type Document, isMap, isSeq, type YAMLMap, type YAMLSeq } from 'yaml';

export type NodePaletteCategory = 'LLM / Agent' | '多模态生成' | '内容渲染器' | '工具与控制流';

export interface NodePaletteItem {
  type: string;
  label: string;
  category: NodePaletteCategory;
  description: string;
  defaultInputKey: string;
  defaultParameters: Record<string, unknown>;
}

export type YamlMutationResult =
  | {
      ok: true;
      yaml: string;
      createdNodeId?: string;
      renamedTo?: string;
    }
  | {
      ok: false;
      error: string;
    };

export interface NodeUpdatePatch {
  newId?: string;
  type?: string;
  dependencies?: string[];
  input_mapping?: Record<string, string>;
  run_if?: string | null;
  timeout_ms?: number | null;
  max_retries?: number | null;
  parameters?: Record<string, unknown>;
}

export const NODE_PALETTE_CATEGORIES: NodePaletteCategory[] = [
  'LLM / Agent',
  '多模态生成',
  '内容渲染器',
  '工具与控制流',
];

export const NODE_PALETTE: NodePaletteItem[] = [
  {
    type: 'llm',
    label: 'LLM 推理 (llm)',
    category: 'LLM / Agent',
    description: '调用大模型执行提示词生成、结构化 JSON 抽取或改写',
    defaultInputKey: 'context',
    defaultParameters: {
      prompt: '请围绕以下上下文生成结构化分析：\n\n{{ context | default("") }}',
    },
  },
  {
    type: 'agent',
    label: '自主循环 Agent (agent)',
    category: 'LLM / Agent',
    description: 'ReAct / Plan-Execute 多步工具调用智能体节点',
    defaultInputKey: 'task',
    defaultParameters: {
      prompt: '{{ task | default("完成调研并输出结论") }}',
      max_steps: 6,
    },
  },
  {
    type: 'skill_agent',
    label: 'Skill 专家 (skill_agent)',
    category: 'LLM / Agent',
    description: '挂载 SKILL.md 领域人设、知识与工具集的智能体',
    defaultInputKey: 'task',
    defaultParameters: {
      prompt: '{{ task | default("执行领域专家分析") }}',
    },
  },
  {
    type: 'tts',
    label: '语音合成 (tts)',
    category: '多模态生成',
    description: '将文稿旁白合成为 WAV/MP3 音频文件',
    defaultInputKey: 'text',
    defaultParameters: {
      voice: 'alloy',
      input_template: '{{ text }}',
    },
  },
  {
    type: 'illustration',
    label: '概念艺术插图 (illustration)',
    category: '多模态生成',
    description: '艺术指导级概念配图与批量幻灯片插画生成（支持无杂乱文字约束与统一画风）',
    defaultInputKey: 'concept',
    defaultParameters: {
      concept: '{{ concept }}',
      style: 'editorial_tech',
      aspect_ratio: '16:9',
    },
  },
  {
    type: 'text_to_image',
    label: '文生图 (text_to_image)',
    category: '多模态生成',
    description: '根据视觉提示词生成插画或封面图',
    defaultInputKey: 'visual_prompt',
    defaultParameters: {
      prompt: '{{ visual_prompt }}',
      size: '1024x1024',
    },
  },
  {
    type: 'text_to_video',
    label: '文生视频 (text_to_video)',
    category: '多模态生成',
    description: '生成动态视频片段（支持时长与画幅设置）',
    defaultInputKey: 'video_prompt',
    defaultParameters: {
      prompt: '{{ video_prompt }}',
      aspect_ratio: '16:9',
      duration_seconds: 5,
    },
  },
  {
    type: 'asr',
    label: '语音转写 (asr)',
    category: '多模态生成',
    description: '将音频输入转写为带时间戳的文本',
    defaultInputKey: 'audio_source',
    defaultParameters: {
      audio_source: '{{ audio_source }}',
    },
  },
  {
    type: 'render_slides',
    label: '幻灯片渲染 (render_slides)',
    category: '内容渲染器',
    description: '将结构化 deck JSON 渲染为原生 OOXML .pptx、16:9 PDF 讲义、PNG/SVG 与 HTML 课件',
    defaultInputKey: 'deck',
    defaultParameters: {
      width: 1920,
      progressive: true,
      export_pptx: true,
    },
  },
  {
    type: 'render_video',
    label: '课件视频合成 (render_video)',
    category: '内容渲染器',
    description: '将幻灯片帧与分段配音通过受控 ffmpeg 合成为旁白视频',
    defaultInputKey: 'slides',
    defaultParameters: {
      fps: 30,
    },
  },
  {
    type: 'render_document',
    label: '文档/PDF 排版 (render_document)',
    category: '内容渲染器',
    description: '通过受控 pandoc / typst 将 Markdown 排版为 PDF 或 HTML 报告',
    defaultInputKey: 'markdown',
    defaultParameters: {
      format: 'html',
    },
  },
  {
    type: 'render_mindmap',
    label: '思维导图渲染 (render_mindmap)',
    category: '内容渲染器',
    description: '将层级主题树渲染为高清 SVG 思维导图',
    defaultInputKey: 'tree',
    defaultParameters: {},
  },
  {
    type: 'render_study',
    label: '闪卡/测验渲染 (render_study)',
    category: '内容渲染器',
    description: '将题库或闪卡 JSON 渲染为单文件交互式学习网页',
    defaultInputKey: 'study',
    defaultParameters: {
      mode: 'flashcards',
    },
  },
  {
    type: 'render_audio',
    label: '播客混音拼接 (render_audio)',
    category: '内容渲染器',
    description: '拼接多段对谈音频并执行响度标准化',
    defaultInputKey: 'segments',
    defaultParameters: {},
  },
  {
    type: 'render_svg',
    label: 'SVG 光栅化 (render_svg)',
    category: '内容渲染器',
    description: '校验 SVG 矢量图并输出 PNG 预览',
    defaultInputKey: 'svg',
    defaultParameters: {
      width: 1600,
    },
  },
  {
    type: 'render_markdown',
    label: 'Markdown 渲染 (render_markdown)',
    category: '内容渲染器',
    description: '将结构化章节拼接输出干净的 Markdown 成品文件',
    defaultInputKey: 'markdown',
    defaultParameters: {},
  },
  {
    type: 'render_table',
    label: '数据表格渲染 (render_table)',
    category: '内容渲染器',
    description: '将行列数据渲染为精美 SVG/HTML 对比矩阵',
    defaultInputKey: 'table',
    defaultParameters: {},
  },
  {
    type: 'figure',
    label: '代码绘图 (figure)',
    category: '内容渲染器',
    description: 'LLM 编写 SVG/Typst/TikZ 源码并带编译错误自修复闭环',
    defaultInputKey: 'spec',
    defaultParameters: {
      format: 'svg',
    },
  },
  {
    type: 'template',
    label: 'Jinja 模板 (template)',
    category: '工具与控制流',
    description: '零外部调用的确定性文本/JSON 组装与转换节点',
    defaultInputKey: 'upstream',
    defaultParameters: {
      template: '{{ upstream | default("") }}',
    },
  },
  {
    type: 'http',
    label: 'HTTP 请求 (http)',
    category: '工具与控制流',
    description: '带 SSRF 防护与 DNS 锁定的外部 API 请求节点',
    defaultInputKey: 'query',
    defaultParameters: {
      method: 'GET',
      url: 'https://example.com',
    },
  },
  {
    type: 'web_search',
    label: '联网检索 (web_search)',
    category: '工具与控制流',
    description: '多厂商联网搜索（Tavily/Bocha/Serper/Brave + 免 Key 聚合回退）',
    defaultInputKey: 'query',
    defaultParameters: {
      query: '{{ query | default("") }}',
      provider: 'auto',
      max_results: 6,
    },
  },
  {
    type: 'repo_snapshot',
    label: '代码仓库快照 (repo_snapshot)',
    category: '工具与控制流',
    description: '提取 GitHub 仓库或受控本地源码目录的模块树、Manifest 与依赖边',
    defaultInputKey: 'source',
    defaultParameters: {
      source: 'https://github.com/tokio-rs/axum',
    },
  },
  {
    type: 'pdf_extract',
    label: 'PDF 论文解析 (pdf_extract)',
    category: '工具与控制流',
    description: '沙箱提取 arXiv/OpenReview/本地 PDF 全文与分页结构',
    defaultInputKey: 'source',
    defaultParameters: {
      source: 'https://arxiv.org/abs/1706.03762',
    },
  },
  {
    type: 'video_ingest',
    label: '视频/字幕采集 (video_ingest)',
    category: '工具与控制流',
    description: '优先提取原生软字幕，自动回退沙箱 ffmpeg 16kHz 音频抽取供 ASR 转写',
    defaultInputKey: 'source',
    defaultParameters: {
      source: '{{ source | default("") }}',
      prefer_subtitles: true,
    },
  },
  {
    type: 'file',
    label: '受控文件读写 (file)',
    category: '工具与控制流',
    description: '在白名单目录内读取或落盘本地文件',
    defaultInputKey: 'content',
    defaultParameters: {
      allowed_paths: ['./workspace'],
      operation: 'write',
      path: './workspace/output.txt',
    },
  },
  {
    type: 'map',
    label: 'Map 并发子流 (map)',
    category: '工具与控制流',
    description: '对数组输入逐项（支持并发上限）执行内嵌子流水线',
    defaultInputKey: 'items',
    defaultParameters: {
      parallel: true,
      max_concurrent: 4,
      template: [
        {
          id: 'item_step',
          type: 'template',
          parameters: {
            template: '{{ item }}',
          },
        },
      ],
    },
  },
  {
    type: 'while',
    label: 'While 迭代循环 (while)',
    category: '工具与控制流',
    description: '在满足条件且未超最大轮次时循环执行子流水线',
    defaultInputKey: 'seed',
    defaultParameters: {
      condition: 'false',
      max_iterations: 3,
      do: [
        {
          id: 'loop_step',
          type: 'template',
          parameters: {
            template: '{{ seed | default("") }}',
          },
        },
      ],
    },
  },
];

const NODE_ID_RE = /^[a-zA-Z0-9_-]+$/;

function parseWorkflowDoc(
  yamlText: string,
): { ok: true; doc: Document; root: YAMLMap; nodesSeq: YAMLSeq } | { ok: false; error: string } {
  let doc: Document;
  try {
    doc = YAML.parseDocument(yamlText);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return { ok: false, error: `YAML 语法解析失败：${msg}` };
  }
  if (doc.errors.length > 0) {
    return { ok: false, error: `YAML 语法错误：${doc.errors[0]?.message ?? '未知错误'}` };
  }
  if (!doc.contents || !isMap(doc.contents)) {
    return { ok: false, error: '工作流顶层必须是 YAML 映射对象' };
  }
  const root = doc.contents;
  let nodesNode: unknown = root.get('nodes', true);
  if (!nodesNode) {
    const created = doc.createNode([]);
    root.set('nodes', created);
    nodesNode = created;
  }
  if (!isSeq(nodesNode)) {
    return { ok: false, error: '工作流 `nodes` 字段必须是数组序列' };
  }
  return { ok: true, doc, root, nodesSeq: nodesNode };
}

function stringifyDoc(doc: Document): string {
  return doc.toString({ indent: 2, lineWidth: 0 });
}

function getNodeId(map: YAMLMap): string | null {
  const raw = map.get('id');
  return typeof raw === 'string' && raw.trim() ? raw.trim() : null;
}

function collectNodeMaps(nodesSeq: YAMLSeq): { id: string; map: YAMLMap; index: number }[] {
  const result: { id: string; map: YAMLMap; index: number }[] = [];
  nodesSeq.items.forEach((item, index) => {
    if (isMap(item)) {
      const id = getNodeId(item);
      if (id) {
        result.push({ id, map: item, index });
      }
    }
  });
  return result;
}

function extractNodeRefsFromString(expr: string): string[] {
  const refs: string[] = [];
  const regex = /(?:^|[^a-zA-Z0-9_-])nodes\.([a-zA-Z0-9_-]+)/g;
  let match = regex.exec(expr);
  while (match) {
    const id = match[1];
    if (id && !refs.includes(id)) {
      refs.push(id);
    }
    match = regex.exec(expr);
  }
  return refs;
}

function buildAdjacency(nodesSeq: YAMLSeq): Map<string, Set<string>> {
  const entries = collectNodeMaps(nodesSeq);
  const known = new Set(entries.map((e) => e.id));
  const adj = new Map<string, Set<string>>();
  for (const id of known) {
    adj.set(id, new Set());
  }

  for (const { id: targetId, map } of entries) {
    const deps = map.get('dependencies', true);
    if (isSeq(deps)) {
      for (const d of deps.items) {
        const depId = isScalarString(d);
        if (depId && depId !== targetId && known.has(depId)) {
          adj.get(depId)?.add(targetId);
        }
      }
    }
    const mapping = map.get('input_mapping', true);
    if (isMap(mapping)) {
      for (const pair of mapping.items) {
        const val = isScalarString(pair.value);
        if (val) {
          for (const src of extractNodeRefsFromString(val)) {
            if (src !== targetId && known.has(src)) {
              adj.get(src)?.add(targetId);
            }
          }
        }
      }
    }
    const runIf = map.get('run_if');
    if (typeof runIf === 'string') {
      for (const src of extractNodeRefsFromString(runIf)) {
        if (src !== targetId && known.has(src)) {
          adj.get(src)?.add(targetId);
        }
      }
    }
  }
  return adj;
}

function isScalarString(node: unknown): string | null {
  if (typeof node === 'string') return node;
  if (node && typeof node === 'object' && 'value' in node) {
    const v = (node as { value: unknown }).value;
    if (typeof v === 'string') return v;
  }
  return null;
}

function canReach(adj: Map<string, Set<string>>, start: string, goal: string): boolean {
  if (start === goal) return true;
  const visited = new Set<string>([start]);
  const queue: string[] = [start];
  while (queue.length > 0) {
    const cur = queue.shift()!;
    for (const next of adj.get(cur) ?? []) {
      if (next === goal) return true;
      if (!visited.has(next)) {
        visited.add(next);
        queue.push(next);
      }
    }
  }
  return false;
}

function hasAnyCycle(adj: Map<string, Set<string>>): boolean {
  const inDegree = new Map<string, number>();
  for (const id of adj.keys()) {
    inDegree.set(id, 0);
  }
  for (const targets of adj.values()) {
    for (const t of targets) {
      inDegree.set(t, (inDegree.get(t) ?? 0) + 1);
    }
  }
  const queue: string[] = [];
  for (const [id, deg] of inDegree.entries()) {
    if (deg === 0) queue.push(id);
  }
  let visited = 0;
  while (queue.length > 0) {
    const u = queue.shift()!;
    visited += 1;
    for (const v of adj.get(u) ?? []) {
      const next = (inDegree.get(v) ?? 1) - 1;
      inDegree.set(v, next);
      if (next === 0) queue.push(v);
    }
  }
  return visited !== adj.size;
}

function defaultMappingForTarget(
  sourceId: string,
  sourceType: string,
  targetType: string,
  existingKeys: Set<string>,
): { key: string; expr: string } {
  const palette = NODE_PALETTE.find((p) => p.type === targetType);
  const preferredKey = palette?.defaultInputKey ?? 'input';
  const key = existingKeys.has(preferredKey) ? `from_${sourceId}` : preferredKey;

  if (targetType === 'render_slides' && key === 'deck') {
    const outField = sourceType === 'template' || sourceType === 'llm' ? 'deck' : 'output';
    return { key, expr: `{{ nodes.${sourceId}.outputs.${outField} }}` };
  }
  return { key, expr: `{{ nodes.${sourceId}.outputs.output }}` };
}

function generateUniqueNodeId(
  existingIds: Set<string>,
  nodeType: string,
  preferredId?: string,
): string {
  const baseRaw = (preferredId?.trim() || nodeType || 'step').replace(/[^a-zA-Z0-9_-]/g, '_');
  if (preferredId && !existingIds.has(baseRaw)) {
    return baseRaw;
  }
  let idx = 1;
  while (existingIds.has(`${baseRaw}_${idx}`)) {
    idx += 1;
  }
  return `${baseRaw}_${idx}`;
}

export function addNodeToYaml(
  yamlText: string,
  nodeType: string,
  options?: { afterNodeId?: string; preferredId?: string },
): YamlMutationResult {
  const parsed = parseWorkflowDoc(yamlText);
  if (!parsed.ok) return parsed;
  const { doc, nodesSeq } = parsed;

  const entries = collectNodeMaps(nodesSeq);
  const existingIds = new Set(entries.map((e) => e.id));
  const createdNodeId = generateUniqueNodeId(existingIds, nodeType, options?.preferredId);
  const palette = NODE_PALETTE.find((p) => p.type === nodeType);

  const nodeObj: Record<string, unknown> = {
    id: createdNodeId,
    type: nodeType,
  };

  const upstream = options?.afterNodeId
    ? entries.find((e) => e.id === options.afterNodeId)
    : undefined;
  if (upstream) {
    const upstreamType =
      typeof upstream.map.get('type') === 'string'
        ? (upstream.map.get('type') as string)
        : 'unknown';
    const mapping = defaultMappingForTarget(upstream.id, upstreamType, nodeType, new Set());
    nodeObj.dependencies = [upstream.id];
    nodeObj.input_mapping = {
      [mapping.key]: mapping.expr,
    };
  }

  nodeObj.parameters = palette ? structuredClone(palette.defaultParameters) : {};

  nodesSeq.add(doc.createNode(nodeObj));
  return {
    ok: true,
    yaml: stringifyDoc(doc),
    createdNodeId,
  };
}

export function connectNodesInYaml(
  yamlText: string,
  sourceId: string,
  targetId: string,
): YamlMutationResult {
  if (sourceId === targetId) {
    return { ok: false, error: '节点不能连接到自身（禁止自环）' };
  }
  const parsed = parseWorkflowDoc(yamlText);
  if (!parsed.ok) return parsed;
  const { doc, nodesSeq } = parsed;

  const entries = collectNodeMaps(nodesSeq);
  const sourceEntry = entries.find((e) => e.id === sourceId);
  const targetEntry = entries.find((e) => e.id === targetId);
  if (!sourceEntry || !targetEntry) {
    return { ok: false, error: `未找到待连接节点（${sourceId} → ${targetId}）` };
  }

  const adj = buildAdjacency(nodesSeq);
  if (canReach(adj, targetId, sourceId)) {
    return {
      ok: false,
      error: `无法连接 ${sourceId} → ${targetId}：该连线会形成有向环路（DAG 必须无环）`,
    };
  }

  // Ensure `sourceId` is in `target.dependencies`
  const depsRaw: unknown = targetEntry.map.get('dependencies', true);
  if (!depsRaw || !isSeq(depsRaw)) {
    targetEntry.map.set('dependencies', doc.createNode([sourceId]));
  } else {
    const existingDeps = depsRaw.items.map(isScalarString).filter((x): x is string => Boolean(x));
    if (!existingDeps.includes(sourceId)) {
      depsRaw.add(doc.createNode(sourceId));
    }
  }

  // Add smart `input_mapping` if `target` does not already reference `nodes.<sourceId>.`
  let mappingMap = targetEntry.map.get('input_mapping', true);
  const existingKeys = new Set<string>();
  let alreadyReferencesSource = false;
  if (mappingMap && isMap(mappingMap)) {
    for (const pair of mappingMap.items) {
      const k = isScalarString(pair.key);
      const v = isScalarString(pair.value);
      if (k) existingKeys.add(k);
      if (v && extractNodeRefsFromString(v).includes(sourceId)) {
        alreadyReferencesSource = true;
      }
    }
  }
  if (!alreadyReferencesSource) {
    const sourceType =
      typeof sourceEntry.map.get('type') === 'string'
        ? (sourceEntry.map.get('type') as string)
        : 'unknown';
    const targetType =
      typeof targetEntry.map.get('type') === 'string'
        ? (targetEntry.map.get('type') as string)
        : 'unknown';
    const def = defaultMappingForTarget(sourceId, sourceType, targetType, existingKeys);
    if (!mappingMap || !isMap(mappingMap)) {
      targetEntry.map.set('input_mapping', doc.createNode({ [def.key]: def.expr }));
    } else {
      mappingMap.set(def.key, def.expr);
    }
  }

  return { ok: true, yaml: stringifyDoc(doc) };
}

export function disconnectEdgeInYaml(
  yamlText: string,
  sourceId: string,
  targetId: string,
): YamlMutationResult {
  const parsed = parseWorkflowDoc(yamlText);
  if (!parsed.ok) return parsed;
  const { doc, nodesSeq } = parsed;

  const targetEntry = collectNodeMaps(nodesSeq).find((e) => e.id === targetId);
  if (!targetEntry) {
    return { ok: false, error: `未找到目标节点 ${targetId}` };
  }

  stripUpstreamRefsFromNodeMap(targetEntry.map, sourceId);
  return { ok: true, yaml: stringifyDoc(doc) };
}

function stripUpstreamRefsFromNodeMap(map: YAMLMap, removedId: string): void {
  const deps = map.get('dependencies', true);
  if (isSeq(deps)) {
    deps.items = deps.items.filter((item) => isScalarString(item) !== removedId);
    if (deps.items.length === 0) {
      map.delete('dependencies');
    }
  }

  const mapping = map.get('input_mapping', true);
  if (isMap(mapping)) {
    mapping.items = mapping.items.filter((pair) => {
      const val = isScalarString(pair.value);
      return !(val && extractNodeRefsFromString(val).includes(removedId));
    });
    if (mapping.items.length === 0) {
      map.delete('input_mapping');
    }
  }

  const runIf = map.get('run_if');
  if (typeof runIf === 'string' && extractNodeRefsFromString(runIf).includes(removedId)) {
    map.delete('run_if');
  }
}

export function deleteNodeFromYaml(yamlText: string, nodeId: string): YamlMutationResult {
  const parsed = parseWorkflowDoc(yamlText);
  if (!parsed.ok) return parsed;
  const { doc, nodesSeq } = parsed;

  const entries = collectNodeMaps(nodesSeq);
  const target = entries.find((e) => e.id === nodeId);
  if (!target) {
    return { ok: false, error: `节点 "${nodeId}" 不存在` };
  }

  nodesSeq.items.splice(target.index, 1);
  for (const entry of collectNodeMaps(nodesSeq)) {
    stripUpstreamRefsFromNodeMap(entry.map, nodeId);
  }

  return { ok: true, yaml: stringifyDoc(doc) };
}

function rewriteNodeIdReferencesInString(expr: string, oldId: string, newId: string): string {
  const escaped = oldId.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const re = new RegExp(`(^|[^a-zA-Z0-9_-])nodes\\.${escaped}(?=[^a-zA-Z0-9_-]|$)`, 'g');
  return expr.replace(re, `$1nodes.${newId}`);
}

export function updateNodeInYaml(
  yamlText: string,
  nodeId: string,
  patch: NodeUpdatePatch,
): YamlMutationResult {
  const parsed = parseWorkflowDoc(yamlText);
  if (!parsed.ok) return parsed;
  const { doc, nodesSeq } = parsed;

  const entries = collectNodeMaps(nodesSeq);
  const target = entries.find((e) => e.id === nodeId);
  if (!target) {
    return { ok: false, error: `未找到节点 "${nodeId}"` };
  }

  let effectiveId = nodeId;
  if (patch.newId !== undefined && patch.newId.trim() !== nodeId) {
    const candidate = patch.newId.trim();
    if (!candidate || !NODE_ID_RE.test(candidate)) {
      return { ok: false, error: '节点 ID 仅允许字母、数字、下划线与连字符' };
    }
    if (entries.some((e) => e.id === candidate)) {
      return { ok: false, error: `节点 ID "${candidate}" 已存在，请换一个名称` };
    }
    target.map.set('id', candidate);
    effectiveId = candidate;

    for (const entry of entries) {
      if (entry.id === nodeId) continue;
      const deps = entry.map.get('dependencies', true);
      if (isSeq(deps)) {
        deps.items = deps.items.map((item) =>
          isScalarString(item) === nodeId ? doc.createNode(candidate) : item,
        );
      }
      const mapping = entry.map.get('input_mapping', true);
      if (isMap(mapping)) {
        for (const pair of mapping.items) {
          const val = isScalarString(pair.value);
          if (val) {
            pair.value = doc.createNode(rewriteNodeIdReferencesInString(val, nodeId, candidate));
          }
        }
      }
      const runIf = entry.map.get('run_if');
      if (typeof runIf === 'string') {
        entry.map.set('run_if', rewriteNodeIdReferencesInString(runIf, nodeId, candidate));
      }
    }
  }

  if (patch.type !== undefined && patch.type.trim()) {
    target.map.set('type', patch.type.trim());
  }

  if (patch.dependencies !== undefined) {
    const cleaned = [...new Set(patch.dependencies.map((d) => d.trim()).filter(Boolean))].filter(
      (d) => d !== effectiveId,
    );
    if (cleaned.length === 0) {
      target.map.delete('dependencies');
    } else {
      target.map.set('dependencies', doc.createNode(cleaned));
    }
  }

  if (patch.input_mapping !== undefined) {
    const entriesClean = Object.entries(patch.input_mapping)
      .map(([k, v]) => [k.trim(), v] as const)
      .filter(([k]) => Boolean(k));
    if (entriesClean.length === 0) {
      target.map.delete('input_mapping');
    } else {
      target.map.set('input_mapping', doc.createNode(Object.fromEntries(entriesClean)));
    }
  }

  if (patch.run_if !== undefined) {
    const trimmed = patch.run_if?.trim() ?? '';
    if (!trimmed) {
      target.map.delete('run_if');
    } else {
      target.map.set('run_if', trimmed);
    }
  }

  if (patch.timeout_ms !== undefined) {
    if (patch.timeout_ms === null || Number.isNaN(patch.timeout_ms) || patch.timeout_ms <= 0) {
      target.map.delete('timeout_ms');
    } else {
      target.map.set('timeout_ms', Math.round(patch.timeout_ms));
    }
  }

  if (patch.max_retries !== undefined) {
    if (patch.max_retries === null || Number.isNaN(patch.max_retries) || patch.max_retries < 0) {
      target.map.delete('max_retries');
    } else {
      target.map.set('max_retries', Math.round(patch.max_retries));
    }
  }

  if (patch.parameters !== undefined) {
    const currentType =
      typeof target.map.get('type') === 'string' ? (target.map.get('type') as string) : '';
    const subKey = currentType === 'map' ? 'template' : currentType === 'while' ? 'do' : null;
    const existingParams = target.map.get('parameters', true);
    const preservedSubSeq =
      subKey && isMap(existingParams) && !(subKey in patch.parameters)
        ? existingParams.get(subKey, true)
        : undefined;

    const nextParamsNode = doc.createNode(patch.parameters) as YAMLMap;
    if (subKey && preservedSubSeq !== undefined) {
      nextParamsNode.set(subKey, preservedSubSeq);
    }
    target.map.set('parameters', nextParamsNode);
  }

  // Guard against accidental DAG cycle introduced via dependencies / input_mapping / run_if edit
  const adj = buildAdjacency(nodesSeq);
  if (hasAnyCycle(adj)) {
    return { ok: false, error: '该修改会引入节点循环依赖（DAG 必须无环），已拒绝变更' };
  }

  return {
    ok: true,
    yaml: stringifyDoc(doc),
    renamedTo: effectiveId !== nodeId ? effectiveId : undefined,
  };
}
