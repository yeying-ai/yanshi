import YAML from 'yaml';
import {
  OFFICIAL_GENERATOR_PRESETS,
  type PresetCategory,
  TECH_SLIDES_VIDEO_RAW_YAML,
} from './generatorCatalog';
import type {
  InspectedEdge,
  InspectedInput,
  InspectedInputKind,
  InspectedNode,
  InspectWorkflowResponse,
} from '../schemas';

export interface WorkflowPreset {
  id: string;
  title: string;
  subtitle: string;
  badge: string;
  category?: PresetCategory;
  yaml: string;
}

export const TECH_SLIDES_VIDEO_YAML = TECH_SLIDES_VIDEO_RAW_YAML;

export const INSTANT_STEP_DEBUG_YAML = `name: instant_step_debug_demo
inputs:
  topic:
    description: "演练主题（免 API Key 本地秒级单步观测）"
    default: "Rust 所有权与借用检查器"
  difficulty:
    description: "目标深度（入门 / 进阶 / 架构级）"
    default: "架构级"
  include_quiz:
    description: "是否生成随堂思考题分支（触发 run_if 条件边）"
    default: true

nodes:
  - id: normalize_brief
    type: template
    parameters:
      output_format: json
      template: |
        {
          "headline": {{ topic | json_encode() }},
          "level": {{ difficulty | json_encode() }},
          "quiz_enabled": {{ include_quiz }}
        }

  - id: core_outline
    type: template
    dependencies: ["normalize_brief"]
    input_mapping:
      headline: "{{ nodes.normalize_brief.outputs.headline }}"
      level: "{{ nodes.normalize_brief.outputs.level }}"
    parameters:
      template: |
        ## {{ headline }}（{{ level }}）
        1. 核心心智模型与不变式
        2. 编译器静态消解 vs 运行期开销对比
        3. 工程边界与常见反模式

  - id: quiz_branch
    type: template
    dependencies: ["normalize_brief"]
    run_if: "{{ nodes.normalize_brief.outputs.quiz_enabled }}"
    input_mapping:
      headline: "{{ nodes.normalize_brief.outputs.headline }}"
    parameters:
      template: |
        ### 随堂自测（{{ headline }}）
        - 为什么可变引用在同一时刻具有独占别名语义？
        - 何时应当把生命周期上界收拢到方法签名而非结构体定义？

  - id: assemble_handout
    type: template
    dependencies: ["core_outline", "quiz_branch"]
    input_mapping:
      outline: "{{ nodes.core_outline.outputs.output }}"
      quiz: "{{ nodes.quiz_branch.outputs.output }}"
    parameters:
      template: |
        # 课程讲义汇总

        {{ outline }}

        {% if quiz is defined %}{{ quiz }}{% else %}_（已跳过随堂思考题分支）_{% endif %}
`;

export const PARALLEL_SLIDE_PREVIEW_YAML = `name: instant_slide_render_demo
inputs:
  deck_title:
    description: "幻灯片主标题（免 LLM 直接验证 SVG/PNG 课件渲染与预览）"
    default: "Rust Trait 分发机制可视化"
  slide_theme:
    description: "配色主题：dark 或 light"
    default: "dark"

nodes:
  - id: sample_deck
    type: template
    parameters:
      output_format: json
      template: |
        {
          "deck": {
            "title": {{ deck_title | json_encode() }},
            "subtitle": "静态单态化 vs vtable 胖指针内存模型",
            "author": "Yanshi Studio",
            "theme": {{ slide_theme | json_encode() }},
            "slides": [
              {
                "section": "01 · 核心抽象",
                "title": "用行为契约替代继承层级",
                "left": {
                  "kind": "code",
                  "language": "rust",
                  "title": "trait Summary",
                  "code": "pub trait Summary {\\n  fn summarize(&self) -> String;\\n}\\n\\npub fn notify(item: &impl Summary) {\\n  println!(\\"Breaking: {}\\", item.summarize());\\n}",
                  "callouts": [{"line": 5, "text": "静态分发：编译期单态化零虚表开销", "tone": "rust"}]
                },
                "right": {
                  "kind": "table",
                  "columns": ["维度", "Rust Trait", "OOP 继承", "C 函数指针"],
                  "highlight_col": 1,
                  "rows": [
                    ["编译期类型安全", "✓ 强约束", "✓ 强约束", "✗ void* 易出错"],
                    ["运行期零开销", "✓ 单态化内联", "✗ 虚表跳转", "✗ 间接调用"],
                    ["为外部类型扩展", "✓ 孤儿规则内支持", "✗ 需改基类", "⚠ 手工封装"]
                  ]
                },
                "takeaway": "优先使用泛型静态分发，仅在异构集合边界按需退化到 &dyn Trait。",
                "notes": "我们先看 Trait 的声明方式。编译器通过单态化把泛型调用直接内联。右边对比了三种抽象方案的取舍。"
              }
            ]
          }
        }

  - id: render_preview
    type: render_slides
    dependencies: ["sample_deck"]
    input_mapping:
      deck: "{{ nodes.sample_deck.outputs.deck }}"
    parameters:
      width: 1920
      progressive: true
`;

export const WORKFLOW_PRESETS: WorkflowPreset[] = [
  ...OFFICIAL_GENERATOR_PRESETS,
  {
    id: 'instant_slide_render_demo',
    title: '免 Key 幻灯片秒级渲染演练',
    subtitle: '2 节点直出 1080p 代码/对比矩阵幻灯片与交互式 HTML，立即可观测图像预览',
    badge: '秒级图像观测',
    category: '免 Key 演练',
    yaml: PARALLEL_SLIDE_PREVIEW_YAML,
  },
  {
    id: 'instant_step_debug_demo',
    title: '免 Key 单步与条件分支演练',
    subtitle: '4 节点含 run_if 条件边，零外部依赖体验每步暂停核查输入/输出',
    badge: '调试入门',
    category: '免 Key 演练',
    yaml: INSTANT_STEP_DEBUG_YAML,
  },
];

function inferKind(defVal: unknown): InspectedInputKind {
  if (typeof defVal === 'boolean') return 'boolean';
  if (typeof defVal === 'number') return 'number';
  if (Array.isArray(defVal) || (defVal !== null && typeof defVal === 'object')) return 'json';
  return 'string';
}

function parseMappingSource(expr: string): { source: string; output: string } | null {
  const trimmed = expr.trim().replace(/^\{\{/, '').replace(/\}\}$/, '').trim();
  const parts = trimmed.split('.');
  if (parts.length === 4 && parts[0] === 'nodes' && parts[2] === 'outputs') {
    return { source: parts[1], output: parts[3] };
  }
  return null;
}

function parseConditionSources(cond: string): string[] {
  const refs: string[] = [];
  const regex = /(?:^|[^a-zA-Z0-9_-])nodes\.([a-zA-Z0-9_-]+)/g;
  let match = regex.exec(cond);
  while (match) {
    const candidate = match[1];
    if (candidate && !refs.includes(candidate)) {
      refs.push(candidate);
    }
    match = regex.exec(cond);
  }
  return refs;
}

/**
 * Instant client-side YAML topology parser used as an optimistic preview
 * before `/v1/workflows/inspect` returns (or when editing offline on Mobile).
 */
export function inspectYamlLocally(yamlText: string): InspectWorkflowResponse {
  let rawDoc: unknown;
  try {
    rawDoc = YAML.parse(yamlText);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return {
      valid: false,
      name: null,
      cost_limit_usd: null,
      inputs: [],
      nodes: [],
      edges: [],
      execution_order: [],
      levels: [],
      diagnostics: {
        valid: false,
        issues: [`YAML 语法错误: ${msg}`],
        warnings: [],
      },
    };
  }

  if (!rawDoc || typeof rawDoc !== 'object' || Array.isArray(rawDoc)) {
    return {
      valid: false,
      name: null,
      cost_limit_usd: null,
      inputs: [],
      nodes: [],
      edges: [],
      execution_order: [],
      levels: [],
      diagnostics: {
        valid: false,
        issues: ['工作流顶层必须是 YAML 对象（包含 name 与 nodes）'],
        warnings: [],
      },
    };
  }

  const doc = rawDoc as Record<string, unknown>;
  const name = typeof doc.name === 'string' ? doc.name : null;
  const costLimit = typeof doc.cost_limit_usd === 'number' ? doc.cost_limit_usd : null;

  const rawInputs =
    doc.inputs && typeof doc.inputs === 'object' && !Array.isArray(doc.inputs)
      ? (doc.inputs as Record<string, Record<string, unknown>>)
      : {};
  const inputs: InspectedInput[] = Object.entries(rawInputs)
    .map(([key, spec]) => ({
      name: key,
      description: typeof spec?.description === 'string' ? spec.description : null,
      required: Boolean(spec?.required),
      default: spec?.default,
      kind: inferKind(spec?.default),
    }))
    .sort((a, b) => Number(b.required) - Number(a.required) || a.name.localeCompare(b.name));

  const rawNodes = Array.isArray(doc.nodes) ? doc.nodes : [];
  const knownIds = new Set<string>();
  for (const item of rawNodes) {
    if (item && typeof item === 'object' && typeof (item as Record<string, unknown>).id === 'string') {
      knownIds.add((item as Record<string, unknown>).id as string);
    }
  }

  const edges: InspectedEdge[] = [];
  const baseNodes: Omit<InspectedNode, 'level'>[] = [];

  for (const item of rawNodes) {
    if (!item || typeof item !== 'object') continue;
    const n = item as Record<string, unknown>;
    const id = typeof n.id === 'string' ? n.id : '';
    const nodeType = typeof n.type === 'string' ? n.type : 'unknown';
    if (!id) continue;

    const dependencies = Array.isArray(n.dependencies)
      ? n.dependencies.filter((d): d is string => typeof d === 'string')
      : [];
    const inputMapping: Record<string, string> = {};
    if (n.input_mapping && typeof n.input_mapping === 'object' && !Array.isArray(n.input_mapping)) {
      for (const [k, v] of Object.entries(n.input_mapping as Record<string, unknown>)) {
        if (typeof v === 'string') inputMapping[k] = v;
      }
    }
    const runIf = typeof n.run_if === 'string' ? n.run_if : null;
    const connectedFrom = new Set<string>();

    const bySource = new Map<string, string[]>();
    for (const [targetKey, expr] of Object.entries(inputMapping).sort(([a], [b]) => a.localeCompare(b))) {
      const parsed = parseMappingSource(expr);
      if (parsed && parsed.source !== id && knownIds.has(parsed.source)) {
        const list = bySource.get(parsed.source) ?? [];
        list.push(`${parsed.output} → ${targetKey}`);
        bySource.set(parsed.source, list);
      }
    }
    for (const [src, labels] of bySource.entries()) {
      connectedFrom.add(src);
      edges.push({ from: src, to: id, kind: 'input_mapping', label: labels.join(', ') });
    }

    if (runIf) {
      for (const src of parseConditionSources(runIf)) {
        if (src !== id && knownIds.has(src)) {
          connectedFrom.add(src);
          edges.push({ from: src, to: id, kind: 'condition', label: 'run_if' });
        }
      }
    }

    for (const dep of dependencies) {
      if (dep !== id && knownIds.has(dep) && !connectedFrom.has(dep)) {
        connectedFrom.add(dep);
        edges.push({ from: dep, to: id, kind: 'dependency', label: null });
      }
    }

    const paramsObj =
      n.parameters && typeof n.parameters === 'object' && !Array.isArray(n.parameters)
        ? { ...(n.parameters as Record<string, unknown>) }
        : {};
    let subNodes: InspectedNode[] = [];
    const subKey = nodeType === 'map' ? 'template' : nodeType === 'while' ? 'do' : null;
    if (subKey && Array.isArray(paramsObj[subKey])) {
      subNodes = (paramsObj[subKey] as Record<string, unknown>[])
        .filter((c) => c && typeof c.id === 'string')
        .map((c, idx) => ({
          id: String(c.id),
          type: typeof c.type === 'string' ? c.type : 'unknown',
          dependencies: [],
          input_mapping: {},
          parameters: c.parameters ?? {},
          level: idx,
        }));
      delete paramsObj[subKey];
    }

    baseNodes.push({
      id,
      type: nodeType,
      dependencies,
      input_mapping: inputMapping,
      run_if: runIf,
      timeout_ms: typeof n.timeout_ms === 'number' ? n.timeout_ms : null,
      max_retries: typeof n.max_retries === 'number' ? n.max_retries : null,
      parameters: paramsObj,
      sub_nodes: subNodes,
    });
  }

  // Compute topological levels via Kahn's algorithm.
  const inDegree = new Map<string, number>();
  const adj = new Map<string, Set<string>>();
  const levelMap = new Map<string, number>();
  for (const n of baseNodes) {
    inDegree.set(n.id, 0);
    adj.set(n.id, new Set());
    levelMap.set(n.id, 0);
  }
  for (const e of edges) {
    const set = adj.get(e.from);
    if (set && !set.has(e.to)) {
      set.add(e.to);
      inDegree.set(e.to, (inDegree.get(e.to) ?? 0) + 1);
    }
  }
  const queue = [...inDegree.entries()]
    .filter(([, deg]) => deg === 0)
    .map(([id]) => id)
    .sort();
  const order: string[] = [];
  while (queue.length > 0) {
    const u = queue.shift()!;
    order.push(u);
    const uLevel = levelMap.get(u) ?? 0;
    const neighbors = [...(adj.get(u) ?? [])].sort();
    for (const v of neighbors) {
      levelMap.set(v, Math.max(levelMap.get(v) ?? 0, uLevel + 1));
      const nextDeg = (inDegree.get(v) ?? 1) - 1;
      inDegree.set(v, nextDeg);
      if (nextDeg === 0) queue.push(v);
    }
  }

  const issues: string[] = [];
  if (baseNodes.length === 0) {
    issues.push('工作流至少需要定义 1 个节点');
  }
  if (order.length !== baseNodes.length) {
    issues.push('检测到节点循环依赖（DAG 必须无环）');
  }

  const nodes: InspectedNode[] = baseNodes.map((n) => ({
    ...n,
    level: levelMap.get(n.id) ?? 0,
  }));
  const maxLevel = nodes.reduce((acc, n) => Math.max(acc, n.level), 0);
  const levels: string[][] =
    order.length > 0 ? Array.from({ length: maxLevel + 1 }, () => []) : [];
  for (const id of order) {
    levels[levelMap.get(id) ?? 0]?.push(id);
  }

  const valid = issues.length === 0;
  return {
    valid,
    name,
    cost_limit_usd: costLimit,
    inputs,
    nodes,
    edges,
    execution_order: order,
    levels,
    diagnostics: {
      valid,
      issues,
      warnings: [],
    },
  };
}
