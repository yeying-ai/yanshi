import audioOverviewYaml from '../../../examples/generators/audio_overview.yml?raw';
import backendProjectPlannerYaml from '../../../examples/generators/backend_project_planner.yml?raw';
import circuitDiagramYaml from '../../../examples/generators/circuit_diagram.yml?raw';
import debatePodcastShowYaml from '../../../examples/generators/debate_podcast_show.yml?raw';
import episodicDramaStudioYaml from '../../../examples/generators/episodic_drama_studio.yml?raw';
import flashcardsYaml from '../../../examples/generators/flashcards.yml?raw';
import illustratedSlidesYaml from '../../../examples/generators/illustrated_slides.yml?raw';
import infographicYaml from '../../../examples/generators/infographic.yml?raw';
import mindmapYaml from '../../../examples/generators/mindmap.yml?raw';
import paperReaderYaml from '../../../examples/generators/paper_reader.yml?raw';
import quizYaml from '../../../examples/generators/quiz.yml?raw';
import repoArchitectureYaml from '../../../examples/generators/repo_architecture.yml?raw';
import reportYaml from '../../../examples/generators/report.yml?raw';
import slidesYaml from '../../../examples/generators/slides.yml?raw';
import techSlidesVideoYaml from '../../../examples/generators/tech_slides_video.yml?raw';
import videoOverviewYaml from '../../../examples/generators/video_overview.yml?raw';
import videoSummarizerYaml from '../../../examples/generators/video_summarizer.yml?raw';
import worldbuildingBibleYaml from '../../../examples/generators/worldbuilding_bible.yml?raw';

export type PresetCategory =
  | '课件与视频'
  | '创意制片与设定'
  | '研究报告与信息图'
  | '导图与工程图'
  | '播客音频'
  | '互动学习'
  | '免 Key 演练'
  | '个人收藏';

export interface CatalogPreset {
  id: string;
  title: string;
  subtitle: string;
  badge: string;
  category: PresetCategory;
  yaml: string;
}

export interface PersonalWorkflowTemplate {
  id: string;
  title: string;
  yaml: string;
  updatedAt: string;
}

interface PersonalTemplateIndexEntry {
  id: string;
  title: string;
  updatedAt: string;
}

export const PERSONAL_TEMPLATES_INDEX_KEY = 'studio.templates.index';
const PERSONAL_TEMPLATE_KEY_PREFIX = 'studio.template.';
const MAX_SAFE_TEMPLATE_BYTES = 15 * 1024;

export function personalTemplateYamlKey(id: string): string {
  return `${PERSONAL_TEMPLATE_KEY_PREFIX}${id}`;
}

export const TECH_SLIDES_VIDEO_RAW_YAML = techSlidesVideoYaml;

export const OFFICIAL_GENERATOR_PRESETS: CatalogPreset[] = [
  {
    id: 'illustrated_slides',
    title: '主题调研与精美配图 PPT (Illustrated Slides)',
    subtitle: '联网检索 → 规划大纲 → 艺术总监精炼配图 + D2 架构图 → 原生 OOXML 可编辑 PPTX + 16:9 PDF + HTML',
    badge: '日常生产力',
    category: '课件与视频',
    yaml: illustratedSlidesYaml,
  },
  {
    id: 'repo_architecture',
    title: '源码 / GitHub 仓库架构与依赖图 (Repo Architecture)',
    subtitle: '自动扫描代码树、Manifest 与依赖边 → D2 系统分层架构图 + 模块依赖 DAG 图 + 深度报告',
    badge: '日常生产力',
    category: '导图与工程图',
    yaml: repoArchitectureYaml,
  },
  {
    id: 'backend_project_planner',
    title: '多轮对话后端项目全套规划 (Backend Planner)',
    subtitle: '多轮交互澄清需求 → 系统架构图 / 模块图 / UML 类图与时序图 / 流程图 / ER 图 + PG DDL + 导图',
    badge: '日常生产力',
    category: '导图与工程图',
    yaml: backendProjectPlannerYaml,
  },
  {
    id: 'paper_reader',
    title: '学术论文双语精读与实验数据提取 (Paper Reader)',
    subtitle: '支持 PDF 文件或 arXiv 链接 → 双语精译 + 核心思想论点 + 实验数据表格 XLSX/CSV + 导图 + PDF',
    badge: '日常生产力',
    category: '研究报告与信息图',
    yaml: paperReaderYaml,
  },
  {
    id: 'video_summarizer',
    title: '视频观点提炼与字幕导图生成 (Video Summarizer)',
    subtitle: '支持视频 URL 或本地文件 → 原生字幕直提或 FFmpeg+ASR 生成 .srt → 时间轴观点精要 + 思维导图',
    badge: '日常生产力',
    category: '课件与视频',
    yaml: videoSummarizerYaml,
  },
  {
    id: 'tech_slides_video',
    title: '拟真人 PPT 演播级技术精讲视频 (Keynote Presenter Video)',
    subtitle: '多源支撑 → 视线引导口播 + 精美插图/D2 架构图 → 按步配音三模式 1080p 视频 + 原生可编辑 PPTX / 16:9 PDF',
    badge: '旗舰工作流',
    category: '课件与视频',
    yaml: techSlidesVideoYaml,
  },
  {
    id: 'episodic_drama_studio',
    title: '角色一致多集连续剧制片厂 (Episodic Drama Studio)',
    subtitle: '黑板编剧室 → VLM 闭环定妆母图 → 跨集图生图/图生视频锚定角色外观 + 专属声纹配音 → 连续剧成片 MP4 + 设定圣经 PDF',
    badge: '多范式创意',
    category: '创意制片与设定',
    yaml: episodicDramaStudioYaml,
  },
  {
    id: 'debate_podcast_show',
    title: '多智能体三声线圆桌辩论播客 (Debate Podcast Show)',
    subtitle: '正反双方 2 轮对抗辩论 + 主持终审裁决 → 三声线 TTS + EBU R128 响度母带 MP3 + 概念封面 + 思辨特刊 PDF',
    badge: '多范式思辨',
    category: '播客音频',
    yaml: debatePodcastShowYaml,
  },
  {
    id: 'worldbuilding_bible',
    title: '多专家接力原创世界观设定圣经 (Worldbuilding Bible)',
    subtitle: '三专家 Handoff 接力（纪元法则 → 阵营人物 → 分支任务）→ VLM 审片主视觉 + D2 势力/叙事状态图 + 导图 + 精装设定集 PDF',
    badge: '多范式创意',
    category: '创意制片与设定',
    yaml: worldbuildingBibleYaml,
  },
  {
    id: 'slides',
    title: '通用主题演示课件 (Slides)',
    subtitle: '来源资料自动提炼大纲、结构化卡片与矢量图表，直出原生可编辑 OOXML PPTX 与 HTML 放映页',
    badge: '官方生成器',
    category: '课件与视频',
    yaml: slidesYaml,
  },
  {
    id: 'video_overview',
    title: '多镜头视频概览 (Video Overview)',
    subtitle: '自动编排分镜脚本、视觉画面、配音旁白与字幕合成完整 MP4 解说视频',
    badge: '官方生成器',
    category: '课件与视频',
    yaml: videoOverviewYaml,
  },
  {
    id: 'report',
    title: '深度溯源研究报告 (Report)',
    subtitle: '大纲规划 → 多章节并行撰写与原文引用脚注 → 导出 Markdown / HTML / PDF',
    badge: '官方生成器',
    category: '研究报告与信息图',
    yaml: reportYaml,
  },
  {
    id: 'infographic',
    title: '结构化信息长图 (Infographic)',
    subtitle: '提炼核心指标与分栏对比结论，排版渲染高清 SVG 与 PNG 视觉信息长图',
    badge: '官方生成器',
    category: '研究报告与信息图',
    yaml: infographicYaml,
  },
  {
    id: 'mindmap',
    title: '知识全景思维导图 (Mindmap)',
    subtitle: '从网页或长文本抽取多层概念树，输出矢量 SVG、PNG 与可交互缩放 Markmap HTML',
    badge: '官方生成器',
    category: '导图与工程图',
    yaml: mindmapYaml,
  },
  {
    id: 'circuit_diagram',
    title: '工程电路原理图 (Circuit Diagram)',
    subtitle: '元器件清单抽取 + CircuiTikZ 编译闭环自检，输出工程级 SVG 与 PNG 原理图',
    badge: '工程制图',
    category: '导图与工程图',
    yaml: circuitDiagramYaml,
  },
  {
    id: 'audio_overview',
    title: '双人播客对谈 (Audio Overview)',
    subtitle: '主持人与嘉宾自然问答脚本生成 + 多音色并行合成 + FFmpeg 混音导出播客音频',
    badge: '官方生成器',
    category: '播客音频',
    yaml: audioOverviewYaml,
  },
  {
    id: 'flashcards',
    title: '互动翻转记忆闪卡 (Flashcards)',
    subtitle: '围绕核心概念生成问答卡片集与免依赖点击翻转复习 HTML 单页',
    badge: '互动学习',
    category: '互动学习',
    yaml: flashcardsYaml,
  },
  {
    id: 'quiz',
    title: '即时反馈自测题库 (Quiz)',
    subtitle: '自动生成多选题、干扰项与答案解析，输出可交互判分 HTML 测验页',
    badge: '互动学习',
    category: '互动学习',
    yaml: quizYaml,
  },
];

export function buildYamlDownloadFilename(rawName: string | null | undefined): string {
  const trimmed = (rawName ?? '').trim();
  if (!trimmed) {
    return 'workflow.yml';
  }
  if (/^[a-zA-Z0-9_.-]+\.(yml|yaml)$/i.test(trimmed)) {
    return trimmed;
  }
  const slug = trimmed
    .toLowerCase()
    .replace(/\.(yml|yaml)$/i, '')
    .replace(/[^a-z0-9_-]+/g, '_')
    .replace(/^_+|_+$/g, '');
  return `${slug || 'workflow'}.yml`;
}

function makeTemplateSlug(title: string): string {
  const ascii = title
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, '_')
    .replace(/^_+|_+$/g, '')
    .slice(0, 32);
  const suffix = Math.random().toString(36).slice(2, 8);
  return ascii ? `${ascii}_${suffix}` : `tpl_${suffix}`;
}

function utf8ByteLength(text: string): number {
  return new TextEncoder().encode(text).length;
}

export function buildPersonalTemplatesPreferencePatch(
  existing: PersonalWorkflowTemplate[],
  input: { id?: string; title: string; yaml: string },
  nowIso: string = new Date().toISOString(),
): {
  nextTemplates: PersonalWorkflowTemplate[];
  preferencePatch: Record<string, unknown>;
} {
  const title = input.title.trim() || '未命名个人模板';
  const yaml = input.yaml;
  if (!yaml.trim()) {
    throw new Error('工作流 YAML 内容不能为空');
  }
  const payloadBytes = utf8ByteLength(JSON.stringify({ yaml }));
  if (payloadBytes > MAX_SAFE_TEMPLATE_BYTES) {
    throw new Error('单个收藏模板大小不能超过 15 KiB，请导出为本地 .yml 文件保存');
  }

  const id = input.id?.trim() || makeTemplateSlug(title);
  const record: PersonalWorkflowTemplate = {
    id,
    title,
    yaml,
    updatedAt: nowIso,
  };

  const filtered = existing.filter((item) => item.id !== id);
  const nextTemplates = [record, ...filtered].slice(0, 24);
  const indexEntries: PersonalTemplateIndexEntry[] = nextTemplates.map((item) => ({
    id: item.id,
    title: item.title,
    updatedAt: item.updatedAt,
  }));

  return {
    nextTemplates,
    preferencePatch: {
      [PERSONAL_TEMPLATES_INDEX_KEY]: { items: indexEntries },
      [personalTemplateYamlKey(id)]: { yaml },
    },
  };
}

export function removePersonalTemplatePatch(
  existing: PersonalWorkflowTemplate[],
  templateId: string,
): {
  nextTemplates: PersonalWorkflowTemplate[];
  preferencePatch: Record<string, unknown>;
} {
  const nextTemplates = existing.filter((item) => item.id !== templateId);
  const indexEntries: PersonalTemplateIndexEntry[] = nextTemplates.map((item) => ({
    id: item.id,
    title: item.title,
    updatedAt: item.updatedAt,
  }));
  return {
    nextTemplates,
    preferencePatch: {
      [PERSONAL_TEMPLATES_INDEX_KEY]: { items: indexEntries },
      [personalTemplateYamlKey(templateId)]: { yaml: '' },
    },
  };
}

export function restorePersonalTemplatesFromPreferences(
  preferences: Record<string, unknown> | null | undefined,
): PersonalWorkflowTemplate[] {
  if (!preferences || typeof preferences !== 'object') {
    return [];
  }
  const rawIndex = preferences[PERSONAL_TEMPLATES_INDEX_KEY];
  if (!rawIndex || typeof rawIndex !== 'object') {
    return [];
  }
  const items = (rawIndex as { items?: unknown }).items;
  if (!Array.isArray(items)) {
    return [];
  }

  const result: PersonalWorkflowTemplate[] = [];
  for (const raw of items) {
    if (!raw || typeof raw !== 'object') continue;
    const entry = raw as Record<string, unknown>;
    const id = typeof entry.id === 'string' ? entry.id.trim() : '';
    const title = typeof entry.title === 'string' ? entry.title.trim() : '';
    const updatedAt = typeof entry.updatedAt === 'string' ? entry.updatedAt : '';
    if (!id || !title) continue;

    const stored = preferences[personalTemplateYamlKey(id)];
    const yaml =
      stored && typeof stored === 'object' && typeof (stored as { yaml?: unknown }).yaml === 'string'
        ? (stored as { yaml: string }).yaml
        : '';
    if (!yaml.trim()) continue;
    result.push({ id, title, yaml, updatedAt });
  }
  return result;
}
