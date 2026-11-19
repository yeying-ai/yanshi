import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { AuthorizedMedia } from '../components/AuthorizedMedia';
import { SpaLink, StudioSubpageHeader } from '../components/StudioSubpageHeader';
import { apiFetch, formatApiError, parseSseChunk, resolveApiUrl } from '../lib/api';
import { formatTime, runFromEnvelope } from '../lib/helpers';
import { navigateTo } from '../lib/router';
import {
  readStorage,
  studioModelOverrideKey,
  tenantKey,
  writeStorage,
} from '../lib/storage';
import {
  ListNotebooksEnvelopeSchema,
  NotebookArtifactsEnvelopeSchema,
  NotebookAskResponseSchema,
  NotebookChunkSchema,
  NotebookDetailEnvelopeSchema,
  NotebookGenerateResponseSchema,
  NotebookSchema,
  NotebookSearchEnvelopeSchema,
  NotebookSourceSchema,
  RunEnvelopeSchema,
  StreamedEventSchema,
  parseJsonResponse,
  type DeliverableCitation,
  type Notebook,
  type NotebookArtifact,
  type NotebookAskResponse,
  type NotebookChunk,
  type NotebookDeliverable,
  type NotebookSearchHit,
  type NotebookSource,
  type StreamedEvent,
} from '../schemas';
import type { MediaPreviewItem, MediaPreviewKind } from '../studio/mediaExtractor';
import { formatSpendUsd, reduceWorkflowEvents } from './WorkflowStudioPage';

export interface NotebookWorkspacePageProps {
  initialNotebookId?: string | null;
  apiToken: string;
  onTokenChange: (token: string) => void;
}

export type GeneratorKind =
  | 'report'
  | 'mindmap'
  | 'audio_overview'
  | 'slides'
  | 'flashcards'
  | 'quiz'
  | 'infographic'
  | 'data_table'
  | 'video_overview'
  | 'illustrated_slides'
  | 'tech_slides_video'
  | 'debate_podcast_show';

export interface GeneratorParamField {
  key: string;
  label: string;
  type: 'text' | 'number' | 'checkbox';
  defaultValue: string | number | boolean;
  min?: number;
  max?: number;
  hint?: string;
}

export interface StudioGeneratorSpec {
  kind: GeneratorKind;
  title: string;
  deliverableBadge: string;
  summary: string;
  fields: GeneratorParamField[];
}

export const STUDIO_GENERATORS: ReadonlyArray<StudioGeneratorSpec> = [
  {
    kind: 'report',
    title: '溯源研究报告',
    deliverableBadge: 'MD · HTML · PDF',
    summary: '规划章节大纲并并行撰写正文，自动附带来源分块引用脚注。',
    fields: [
      {
        key: 'focus',
        label: '报告聚焦主题',
        type: 'text',
        defaultValue: 'the main points, with their supporting evidence',
      },
      {
        key: 'sections',
        label: '章节数量',
        type: 'number',
        defaultValue: 4,
        min: 2,
        max: 10,
      },
      {
        key: 'pdf',
        label: '同时排版 PDF（需本机安装 pandoc 与 typst）',
        type: 'checkbox',
        defaultValue: false,
      },
    ],
  },
  {
    kind: 'mindmap',
    title: '结构化思维导图',
    deliverableBadge: 'SVG · PNG · 交互 HTML',
    summary: '提炼多层级主题树，同时导出矢量图、高清位图与可折叠交互导图页。',
    fields: [
      {
        key: 'focus',
        label: '导图梳理主线',
        type: 'text',
        defaultValue: 'the main ideas and how they relate',
      },
      {
        key: 'depth',
        label: '展开层级深度 (2–4)',
        type: 'number',
        defaultValue: 3,
        min: 2,
        max: 5,
      },
    ],
  },
  {
    kind: 'flashcards',
    title: '翻转记忆闪卡',
    deliverableBadge: '交互 HTML · JSON',
    summary: '按知识点生成一问一答记忆卡片，产出零依赖可点击翻转复习页。',
    fields: [
      {
        key: 'focus',
        label: '闪卡考察重点',
        type: 'text',
        defaultValue: 'the key facts and ideas',
      },
      {
        key: 'count',
        label: '卡片张数 (5–30)',
        type: 'number',
        defaultValue: 12,
        min: 3,
        max: 30,
      },
    ],
  },
  {
    kind: 'quiz',
    title: '互动自测题库',
    deliverableBadge: '自测 HTML · JSON',
    summary: '生成带解析与出处引用的多项选择题，支持页面内即时判分与复盘。',
    fields: [
      {
        key: 'focus',
        label: '测验出题范围',
        type: 'text',
        defaultValue: 'the key facts and ideas',
      },
      {
        key: 'count',
        label: '题目数量 (3–20)',
        type: 'number',
        defaultValue: 8,
        min: 3,
        max: 20,
      },
    ],
  },
  {
    kind: 'infographic',
    title: '核心指标信息长图',
    deliverableBadge: 'SVG · PNG',
    summary: '浓缩关键数字指标、分栏要点与核心结论，渲染出版级视觉信息长图。',
    fields: [
      {
        key: 'focus',
        label: '信息图突出重点',
        type: 'text',
        defaultValue: 'the key numbers and ideas',
      },
    ],
  },
  {
    kind: 'data_table',
    title: '结构化数据提取表',
    deliverableBadge: 'XLSX · CSV · 预览 HTML',
    summary: '从资料原文中抽取对比维度、数值指标与结构化事实，附带逐行出处引用并导出电子表格。',
    fields: [
      {
        key: 'focus',
        label: '提取维度与对比重点',
        type: 'text',
        defaultValue: 'the key entities, metrics, comparisons, and structured facts',
      },
      {
        key: 'max_rows',
        label: '最大提取行数 (5–50)',
        type: 'number',
        defaultValue: 20,
        min: 5,
        max: 50,
      },
    ],
  },
  {
    kind: 'slides',
    title: '演示文稿课件',
    deliverableBadge: 'PPTX · 矢量配图',
    summary: '生成带演讲备注与自动渲染 SVG 示意图的结构化演示幻灯片（需 pandoc）。',
    fields: [
      {
        key: 'focus',
        label: '演示核心论点',
        type: 'text',
        defaultValue: 'the main ideas, for a general audience',
      },
      {
        key: 'slides',
        label: '内容页数',
        type: 'number',
        defaultValue: 6,
        min: 3,
        max: 20,
      },
      {
        key: 'figure_languages',
        label: '配图语言',
        type: 'text',
        defaultValue: 'svg',
      },
    ],
  },
  {
    kind: 'audio_overview',
    title: '双人对话播客概览',
    deliverableBadge: 'MP3 音频',
    summary: '编写主持人与嘉宾对谈台本，并行合成双声部语音并响度标准化拼接（需 TTS + ffmpeg）。',
    fields: [
      {
        key: 'focus',
        label: '对谈讨论焦点',
        type: 'text',
        defaultValue: 'the main ideas, in plain language',
      },
      {
        key: 'minutes',
        label: '预计时长（分钟）',
        type: 'number',
        defaultValue: 3,
        min: 1,
        max: 15,
      },
      {
        key: 'language',
        label: '对谈语言',
        type: 'text',
        defaultValue: 'Chinese',
      },
      {
        key: 'tts_model',
        label: 'TTS 语音模型名',
        type: 'text',
        defaultValue: 'tts-1',
      },
    ],
  },
  {
    kind: 'video_overview',
    title: '多镜头配音视频概览',
    deliverableBadge: 'MP4 · SRT 字幕',
    summary: '编排分镜解说词，并行生成插画画面与旁白语音，合成软字幕 MP4 视频（需生图 + TTS + ffmpeg）。',
    fields: [
      {
        key: 'focus',
        label: '视频讲解主线',
        type: 'text',
        defaultValue: 'the main ideas, in plain language',
      },
      {
        key: 'scene_count',
        label: '分镜数量 (4–10)',
        type: 'number',
        defaultValue: 6,
        min: 2,
        max: 12,
      },
      {
        key: 'language',
        label: '旁白语言',
        type: 'text',
        defaultValue: 'Chinese',
      },
      {
        key: 'image_model',
        label: '文生图模型名',
        type: 'text',
        defaultValue: 'gpt-image-2',
      },
      {
        key: 'tts_model',
        label: 'TTS 语音模型名',
        type: 'text',
        defaultValue: 'tts-1',
      },
    ],
  },
  {
    kind: 'illustrated_slides',
    title: 'AI 插图与架构图幻灯片',
    deliverableBadge: 'HTML · PPTX · PDF',
    summary: '结合笔记本分块规划大纲、编译 D2 架构图与艺术指导插画，输出原生 OOXML 可编辑 PPTX 与 16:9 讲义 PDF。',
    fields: [
      {
        key: 'focus',
        label: '演示切入角度',
        type: 'text',
        defaultValue: '核心原理、架构机制与关键结论',
      },
      {
        key: 'slide_count',
        label: '幻灯片页数 (4–10)',
        type: 'number',
        defaultValue: 6,
        min: 3,
        max: 10,
      },
      {
        key: 'enable_illustrations',
        label: '调用 AI 插画节点生成定制配图',
        type: 'checkbox',
        defaultValue: true,
      },
      {
        key: 'image_model',
        label: '文生图模型名',
        type: 'text',
        defaultValue: 'gemini-3.1-flash-image',
      },
    ],
  },
  {
    kind: 'tech_slides_video',
    title: '1080p 演播级技术精讲视频',
    deliverableBadge: 'MP4 · PPTX · PDF',
    summary: '围绕笔记本资料编排双栏渐进动画幻灯片与视线引导口播稿，合成毫秒级卡点 1080p 讲课视频与同步课件。',
    fields: [
      {
        key: 'focus',
        label: '精讲聚焦主线',
        type: 'text',
        defaultValue: '底层机制、关键数据与工程最佳实践',
      },
      {
        key: 'slide_count',
        label: '正片章节页数 (4–10)',
        type: 'number',
        defaultValue: 6,
        min: 3,
        max: 10,
      },
      {
        key: 'tts_model',
        label: 'TTS 语音模型名',
        type: 'text',
        defaultValue: 'gemini-2.5-flash-preview-tts',
      },
      {
        key: 'image_model',
        label: '文生图模型名',
        type: 'text',
        defaultValue: 'gemini-3.1-flash-image',
      },
    ],
  },
  {
    kind: 'debate_podcast_show',
    title: '三声线多智能体圆桌辩论电台',
    deliverableBadge: 'MP3 · 思辨特刊 PDF',
    summary: '正方开拓派、反方审慎派与金牌主持人基于笔记本资料多轮真刀真枪辩论，混音输出广播级三声线播客与特刊 PDF。',
    fields: [
      {
        key: 'focus',
        label: '圆桌核心辩题',
        type: 'text',
        defaultValue: '围绕笔记本资料中的核心命题与潜在权衡展开正反交锋',
      },
      {
        key: 'turn_count',
        label: '电台对话轮次数 (8–18)',
        type: 'number',
        defaultValue: 12,
        min: 8,
        max: 18,
      },
      {
        key: 'tts_model',
        label: 'TTS 语音模型名',
        type: 'text',
        defaultValue: 'gemini-2.5-flash-preview-tts',
      },
      {
        key: 'enable_cover_art',
        label: '生成单集概念封面插画',
        type: 'checkbox',
        defaultValue: true,
      },
    ],
  },
];

type SourceIngestTab = 'text' | 'url' | 'file' | 'recording';

const SOURCE_INGEST_TABS: ReadonlyArray<{ id: SourceIngestTab; label: string }> = [
  { id: 'text', label: '粘贴文本' },
  { id: 'url', label: '网页 URL' },
  { id: 'file', label: '上传文档' },
  { id: 'recording', label: '上传录音' },
];

/** Extract optional `:id` from `/ui/notebooks/:id`. */
export function notebookIdFromPath(pathname: string): string | null {
  const match = pathname.match(/^\/ui\/notebooks\/([^/]+)\/?$/);
  return match ? decodeURIComponent(match[1]) : null;
}

/** Encode arbitrary binary/UTF-8 bytes into standard Base64 without stack overflow on multi-MB files. */
export function encodeBytesToBase64(bytes: Uint8Array): string {
  const chunkSize = 0x8000;
  const segments: string[] = [];
  for (let offset = 0; offset < bytes.length; offset += chunkSize) {
    const slice = bytes.subarray(offset, offset + chunkSize);
    segments.push(String.fromCharCode(...slice));
  }
  return btoa(segments.join(''));
}

function classifyNotebookArtifactKind(
  name: string,
  mimeType?: string | null,
): MediaPreviewKind {
  const mime = (mimeType ?? '').toLowerCase();
  if (mime.startsWith('image/')) return 'image';
  if (mime.startsWith('audio/')) return 'audio';
  if (mime.startsWith('video/')) return 'video';
  if (mime.startsWith('text/html')) return 'html';
  if (mime.includes('subrip') || mime.startsWith('text/vtt')) return 'subtitle';
  if (mime.includes('pdf') || mime.includes('presentation') || mime.includes('markdown')) {
    return 'document';
  }
  const dot = name.lastIndexOf('.');
  const ext = dot >= 0 ? name.slice(dot + 1).toLowerCase() : '';
  if (['png', 'jpg', 'jpeg', 'webp', 'gif', 'svg'].includes(ext)) return 'image';
  if (['mp3', 'wav', 'ogg', 'm4a'].includes(ext)) return 'audio';
  if (['mp4', 'webm'].includes(ext)) return 'video';
  if (['html', 'htm'].includes(ext)) return 'html';
  if (['srt', 'vtt'].includes(ext)) return 'subtitle';
  return 'document';
}

export function notebookArtifactToMediaItem(artifact: NotebookArtifact): MediaPreviewItem {
  const apiPath = `/v1/runs/${encodeURIComponent(artifact.run_id)}/artifacts/${encodeURIComponent(
    artifact.id,
  )}`;
  return {
    key: `${artifact.kind}:${artifact.id}`,
    nodeId: artifact.node_id,
    label: artifact.name,
    kind: classifyNotebookArtifactKind(artifact.name, artifact.mime_type),
    apiPath,
    directUrl: resolveApiUrl(apiPath),
    mimeType: artifact.mime_type ?? undefined,
    sizeBytes: artifact.size_bytes ?? undefined,
  };
}

/** Deduplicate deliverable citations by `(chunk_id, quote)` while preserving citation order. */
export function dedupeDeliverableCitations(
  citations: ReadonlyArray<DeliverableCitation> | undefined,
): DeliverableCitation[] {
  if (!citations || citations.length === 0) {
    return [];
  }
  const seen = new Set<string>();
  const unique: DeliverableCitation[] = [];
  for (const cit of citations) {
    const chunkId = (cit.chunk_id ?? '').trim();
    const quote = cit.quote.trim();
    if (!quote) continue;
    const key = `${chunkId}::${quote}`;
    if (seen.has(key)) continue;
    seen.add(key);
    unique.push({ ...cit, chunk_id: chunkId, quote });
  }
  return unique;
}

export interface NotebookDeliverableCitationBarProps {
  deliverable: NotebookDeliverable;
  sourceTitleById: ReadonlyMap<string, string>;
  onInspectChunk: (chunkId: string) => void;
}

export function NotebookDeliverableCitationBar({
  deliverable,
  sourceTitleById,
  onInspectChunk,
}: NotebookDeliverableCitationBarProps): React.ReactElement {
  const citations = dedupeDeliverableCitations(deliverable.citations);
  return (
    <div className="nb-deliverable-bar" data-testid="deliverable-citation-bar">
      {deliverable.title.trim() && (
        <h4 className="nb-deliverable-title">{deliverable.title}</h4>
      )}
      {citations.length > 0 && (
        <ul className="nb-citation-chips" role="list" aria-label="交付物原文引用">
          {citations.map((cit, index) => {
            const sourceLabel = sourceTitleById.get(cit.source_id) ?? '资料引用';
            const snippet =
              cit.quote.length > 56 ? `${cit.quote.slice(0, 56)}…` : cit.quote;
            const hasChunk = cit.chunk_id.length > 0;
            return (
              <li key={`${cit.chunk_id || 'quote'}-${index}`}>
                {hasChunk ? (
                  <button
                    type="button"
                    className="nb-citation-chip"
                    data-testid={`citation-chip-${cit.chunk_id}`}
                    aria-label={`查看引用 #${index + 1} 原文分块 ${cit.chunk_id}：${sourceLabel}`}
                    onClick={() => onInspectChunk(cit.chunk_id)}
                  >
                    <span className="nb-citation-chip__badge">
                      [{index + 1}] {sourceLabel}
                    </span>
                    <span className="nb-citation-chip__quote">“{snippet}”</span>
                  </button>
                ) : (
                  <span className="nb-citation-chip nb-citation-chip--static">
                    <span className="nb-citation-chip__badge">
                      [{index + 1}] {sourceLabel}
                    </span>
                    <span className="nb-citation-chip__quote">“{snippet}”</span>
                  </span>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

/**
 * Compute the explicit `source_ids` filter for search / ask / generate.
 * Returns `undefined` when every ready source is checked so newly-ingested
 * sources participate automatically without sending redundant filters.
 */
export function resolveScopedSourceIds(
  sources: ReadonlyArray<Pick<NotebookSource, 'id' | 'status'>>,
  excludedIds: ReadonlySet<string>,
): string[] | undefined {
  if (excludedIds.size === 0) {
    return undefined;
  }
  const readySources = sources.filter((s) => s.status === 'ready');
  const selectedIds = readySources.filter((s) => !excludedIds.has(s.id)).map((s) => s.id);
  if (selectedIds.length === readySources.length) {
    return undefined;
  }
  return selectedIds;
}

export interface NotebookAskTurnPayload {
  question: string;
  answer: string;
}

/**
 * Normalize prior Q&A turns into a bounded `history` payload (`<= 8` turns)
 * for multi-turn follow-up grounding on `POST /v1/notebooks/{id}/ask`.
 */
export function buildAskHistoryPayload(
  turns: ReadonlyArray<Pick<NotebookAskResponse, 'question' | 'answer'>>,
  maxTurns = 8,
): NotebookAskTurnPayload[] {
  const cleaned = turns
    .map((t) => ({
      question: t.question.trim(),
      answer: t.answer.trim(),
    }))
    .filter((t) => t.question.length > 0 || t.answer.length > 0);
  const skip = Math.max(0, cleaned.length - Math.max(1, maxTurns));
  return cleaned.slice(skip);
}

export interface NotebookAskResultCardProps {
  result: NotebookAskResponse;
  turnIndex?: number;
  sourceTitleById: ReadonlyMap<string, string>;
  onInspectChunk: (chunkId: string) => void;
  onUseAsFocus?: (answerText: string) => void;
  onSaveAsSource?: (result: NotebookAskResponse) => void;
  saveBusy?: boolean;
}

export function NotebookAskResultCard({
  result,
  turnIndex,
  sourceTitleById,
  onInspectChunk,
  onUseAsFocus,
  onSaveAsSource,
  saveBusy = false,
}: NotebookAskResultCardProps): React.ReactElement {
  const turnPrefix = typeof turnIndex === 'number' ? `#${turnIndex + 1} · ` : '';
  return (
    <div className="nb-ask-answer" data-testid="notebook-ask-result">
      <div className="nb-ask-answer__meta">
        <strong>
          {turnPrefix}Q: {result.question}
        </strong>
        <span>
          {result.model} · 检索 {result.used_chunk_count} 段原文
        </span>
      </div>
      <p className="nb-ask-answer__body">{result.answer}</p>
      {result.citations.length > 0 && (
        <NotebookDeliverableCitationBar
          deliverable={{
            kind: 'ask',
            title: '',
            citations: result.citations,
          }}
          sourceTitleById={sourceTitleById}
          onInspectChunk={onInspectChunk}
        />
      )}
      <div className="nb-ask-answer__actions">
        {onUseAsFocus && (
          <button
            type="button"
            className="studio-link-btn"
            onClick={() => onUseAsFocus(result.answer)}
          >
            设为当前生成器焦点
          </button>
        )}
        {onSaveAsSource && (
          <button
            type="button"
            className="studio-link-btn"
            data-testid="save-ask-as-source-btn"
            disabled={saveBusy}
            onClick={() => onSaveAsSource(result)}
          >
            {saveBusy ? '保存中…' : '存为笔记来源'}
          </button>
        )}
      </div>
    </div>
  );
}

function buildInitialGeneratorDrafts(): Record<GeneratorKind, Record<string, string | boolean>> {
  const out = {} as Record<GeneratorKind, Record<string, string | boolean>>;
  for (const spec of STUDIO_GENERATORS) {
    const entry: Record<string, string | boolean> = {};
    for (const field of spec.fields) {
      entry[field.key] =
        typeof field.defaultValue === 'boolean'
          ? field.defaultValue
          : String(field.defaultValue);
    }
    out[spec.kind] = entry;
  }
  return out;
}

function sourceStatusLabel(status: string): string {
  switch (status) {
    case 'ready':
      return '已就绪';
    case 'ingesting':
      return '解析分块中…';
    case 'pending':
      return '排队入库…';
    case 'failed':
      return '入库失败';
    default:
      return status;
  }
}

export function NotebookWorkspacePage({
  initialNotebookId = null,
  apiToken,
  onTokenChange,
}: NotebookWorkspacePageProps): React.ReactElement {
  const [tenantId, setTenantId] = useState<string>(() => readStorage(tenantKey, 'default'));
  const [modelName, setModelName] = useState<string>(() =>
    readStorage(studioModelOverrideKey, ''),
  );

  const [notebooks, setNotebooks] = useState<Notebook[]>([]);
  const [selectedNotebookId, setSelectedNotebookId] = useState<string>(initialNotebookId ?? '');
  const [activeNotebook, setActiveNotebook] = useState<Notebook | null>(null);
  const [sources, setSources] = useState<NotebookSource[]>([]);
  const [artifacts, setArtifacts] = useState<NotebookArtifact[]>([]);

  const [newNotebookTitle, setNewNotebookTitle] = useState<string>('');
  const [createNotebookBusy, setCreateNotebookBusy] = useState<boolean>(false);
  const [renamingNotebook, setRenamingNotebook] = useState<boolean>(false);
  const [renameTitleDraft, setRenameTitleDraft] = useState<string>('');
  const [renameNotebookBusy, setRenameNotebookBusy] = useState<boolean>(false);

  // Source ingestion state
  const [ingestTab, setIngestTab] = useState<SourceIngestTab>('text');
  const [sourceTitleDraft, setSourceTitleDraft] = useState<string>('');
  const [sourceTextDraft, setSourceTextDraft] = useState<string>('');
  const [sourceUrlDraft, setSourceUrlDraft] = useState<string>('');
  const [selectedUploadFile, setSelectedUploadFile] = useState<File | null>(null);
  const [selectedRecordingFile, setSelectedRecordingFile] = useState<File | null>(null);
  const [recordingModelDraft, setRecordingModelDraft] = useState<string>('gemini-2.5-flash-asr');
  const [recordingLangDraft, setRecordingLangDraft] = useState<string>('');
  const [ingestBusy, setIngestBusy] = useState<boolean>(false);
  const sourceTabRefs = useRef<Record<SourceIngestTab, HTMLButtonElement | null>>({
    text: null,
    url: null,
    file: null,
    recording: null,
  });

  // BM25 search, source scoping, grounded Q&A & citation chunk lookup state
  const [excludedSourceIds, setExcludedSourceIds] = useState<string[]>([]);
  const [askQuestionDraft, setAskQuestionDraft] = useState<string>('');
  const [askBusy, setAskBusy] = useState<boolean>(false);
  const [askThread, setAskThread] = useState<NotebookAskResponse[]>([]);
  const [saveAskSourceBusy, setSaveAskSourceBusy] = useState<boolean>(false);
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [searchBusy, setSearchBusy] = useState<boolean>(false);
  const [searchHits, setSearchHits] = useState<NotebookSearchHit[] | null>(null);
  const [chunkLookupId, setChunkLookupId] = useState<string>('');
  const [chunkLookupBusy, setChunkLookupBusy] = useState<boolean>(false);
  const [inspectedChunk, setInspectedChunk] = useState<NotebookChunk | null>(null);
  const [deletingSourceId, setDeletingSourceId] = useState<string | null>(null);
  const [deletingArtifactId, setDeletingArtifactId] = useState<string | null>(null);
  const [deleteNotebookBusy, setDeleteNotebookBusy] = useState<boolean>(false);
  const deleteDialogRef = useRef<HTMLDialogElement | null>(null);
  const chunkDialogRef = useRef<HTMLDialogElement | null>(null);

  // Studio generator state
  const [expandedKind, setExpandedKind] = useState<GeneratorKind>('flashcards');
  const [paramDrafts, setParamDrafts] = useState<
    Record<GeneratorKind, Record<string, string | boolean>>
  >(() => buildInitialGeneratorDrafts());
  const [generateBusyKind, setGenerateBusyKind] = useState<GeneratorKind | null>(null);

  // Active generator run live tracking
  const [activeGenRunId, setActiveGenRunId] = useState<string>('');
  const [activeGenKind, setActiveGenKind] = useState<GeneratorKind | null>(null);
  const [activeGenStatus, setActiveGenStatus] = useState<string>('');
  const [activeGenEvents, setActiveGenEvents] = useState<StreamedEvent[]>([]);
  const runStreamAbortRef = useRef<AbortController | null>(null);

  // Gallery filter & feedback banners
  const [artifactFilter, setArtifactFilter] = useState<'all' | GeneratorKind>('all');
  const [errorBanner, setErrorBanner] = useState<string | null>(null);
  const [noticeBanner, setNoticeBanner] = useState<string | null>(null);

  useEffect(() => {
    if (initialNotebookId && initialNotebookId !== selectedNotebookId) {
      setSelectedNotebookId(initialNotebookId);
    }
  }, [initialNotebookId, selectedNotebookId]);

  useEffect(() => {
    writeStorage(tenantKey, tenantId);
  }, [tenantId]);

  useEffect(() => {
    writeStorage(studioModelOverrideKey, modelName);
  }, [modelName]);

  const loadNotebooksList = useCallback(
    async (preferId?: string) => {
      try {
        const res = await apiFetch('/v1/notebooks', apiToken, {}, tenantId);
        if (!res.ok) {
          if (res.status === 401) {
            setErrorBanner(
              '🔐 当前网关已启用访问鉴权（HTTP 401）：请在右上角「Token」输入框填入服务端 YANSHI_API_TOKEN。',
            );
            return;
          }
          const text = await res.text();
          throw new Error(`加载笔记本列表失败 (${res.status}): ${text}`);
        }
        setErrorBanner((prev) => (prev && prev.includes('HTTP 401') ? null : prev));
        const env = await parseJsonResponse(
          ListNotebooksEnvelopeSchema,
          res,
          'GET /v1/notebooks',
        );
        setNotebooks(env.notebooks);
        setSelectedNotebookId((prev) => {
          const target = preferId || prev;
          if (target && env.notebooks.some((n) => n.id === target)) {
            return target;
          }
          return env.notebooks[0]?.id ?? '';
        });
      } catch (err) {
        setErrorBanner(formatApiError(err));
      }
    },
    [apiToken, tenantId],
  );

  const loadNotebookDetailAndArtifacts = useCallback(
    async (notebookId: string) => {
      if (!notebookId) {
        setActiveNotebook(null);
        setSources([]);
        setArtifacts([]);
        return;
      }
      try {
        const [detailRes, artRes] = await Promise.all([
          apiFetch(`/v1/notebooks/${encodeURIComponent(notebookId)}`, apiToken, {}, tenantId),
          apiFetch(
            `/v1/notebooks/${encodeURIComponent(notebookId)}/artifacts`,
            apiToken,
            {},
            tenantId,
          ),
        ]);
        if (!detailRes.ok) {
          if (detailRes.status === 404) {
            setActiveNotebook(null);
            setSources([]);
            setArtifacts([]);
            return;
          }
          const text = await detailRes.text();
          throw new Error(`读取笔记本详情失败 (${detailRes.status}): ${text}`);
        }
        const detail = await parseJsonResponse(
          NotebookDetailEnvelopeSchema,
          detailRes,
          'GET /v1/notebooks/:id',
        );
        setActiveNotebook(detail.notebook);
        setSources(detail.sources);

        if (artRes.ok) {
          const artEnv = await parseJsonResponse(
            NotebookArtifactsEnvelopeSchema,
            artRes,
            'GET /v1/notebooks/:id/artifacts',
          );
          setArtifacts(artEnv.artifacts);
        }
      } catch (err) {
        setErrorBanner(formatApiError(err));
      }
    },
    [apiToken, tenantId],
  );

  useEffect(() => {
    void loadNotebooksList(initialNotebookId ?? undefined);
  }, [loadNotebooksList, initialNotebookId]);

  useEffect(() => {
    setExcludedSourceIds([]);
    setAskThread([]);
    setRenamingNotebook(false);
    setSearchHits(null);
    setInspectedChunk(null);
    void loadNotebookDetailAndArtifacts(selectedNotebookId);
  }, [selectedNotebookId, loadNotebookDetailAndArtifacts]);

  // Poll notebook sources while any source is still `pending` or `ingesting`.
  const hasIngestingSource = useMemo(
    () => sources.some((s) => s.status === 'pending' || s.status === 'ingesting'),
    [sources],
  );

  useEffect(() => {
    if (!selectedNotebookId || !hasIngestingSource) {
      return undefined;
    }
    const timer = window.setInterval(() => {
      void loadNotebookDetailAndArtifacts(selectedNotebookId);
    }, 1200);
    return () => window.clearInterval(timer);
  }, [selectedNotebookId, hasIngestingSource, loadNotebookDetailAndArtifacts]);

  // Stream live generator run events via SSE and refresh artifacts upon completion.
  useEffect(() => {
    if (!activeGenRunId) {
      return undefined;
    }
    let cancelled = false;
    const abortCtrl = new AbortController();
    runStreamAbortRef.current?.abort();
    runStreamAbortRef.current = abortCtrl;

    const watchGeneratorRun = async () => {
      try {
        const res = await apiFetch(
          `/v1/runs/${encodeURIComponent(activeGenRunId)}/events`,
          apiToken,
          {
            headers: { Accept: 'text/event-stream' },
            signal: abortCtrl.signal,
          },
          tenantId,
        );
        if (res.ok && res.body) {
          const reader = res.body.getReader();
          const decoder = new TextDecoder();
          let carry = '';
          while (!cancelled) {
            const { value, done } = await reader.read();
            if (done) break;
            carry += decoder.decode(value, { stream: true });
            const parsed = parseSseChunk(carry, StreamedEventSchema, 'notebook generator SSE');
            carry = parsed.rest;
            for (const ev of parsed.events) {
              setActiveGenEvents((prev) =>
                prev.some((e) => e.seq === ev.seq) ? prev : [...prev, ev],
              );
            }
          }
        }

        // Confirm terminal run status and refresh notebook artifacts.
        if (!cancelled) {
          const runRes = await apiFetch(
            `/v1/runs/${encodeURIComponent(activeGenRunId)}`,
            apiToken,
            {},
            tenantId,
          );
          if (runRes.ok) {
            const runEnv = await parseJsonResponse(RunEnvelopeSchema, runRes, 'GET generator run');
            const rec = runFromEnvelope(runEnv);
            setActiveGenStatus(rec.status);
            if (rec.status === 'failed' && rec.error) {
              setErrorBanner(`生成任务失败：${rec.error}`);
            } else if (rec.status === 'succeeded') {
              setNoticeBanner('成品已生成并自动挂载至右侧画廊。');
            }
          }
          if (selectedNotebookId) {
            await loadNotebookDetailAndArtifacts(selectedNotebookId);
          }
        }
      } catch (err) {
        if (cancelled || (err instanceof DOMException && err.name === 'AbortError')) {
          return;
        }
        setErrorBanner(formatApiError(err));
      }
    };

    void watchGeneratorRun();

    return () => {
      cancelled = true;
      abortCtrl.abort();
    };
  }, [activeGenRunId, apiToken, tenantId, selectedNotebookId, loadNotebookDetailAndArtifacts]);

  const readySourceCount = useMemo(
    () => sources.filter((s) => s.status === 'ready').length,
    [sources],
  );

  const excludedSourceSet = useMemo(() => new Set(excludedSourceIds), [excludedSourceIds]);

  const selectedReadyCount = useMemo(
    () => sources.filter((s) => s.status === 'ready' && !excludedSourceSet.has(s.id)).length,
    [sources, excludedSourceSet],
  );

  const scopedSourceIds = useMemo(
    () => resolveScopedSourceIds(sources, excludedSourceSet),
    [sources, excludedSourceSet],
  );

  const sourceTitleById = useMemo(() => {
    const map = new Map<string, string>();
    for (const s of sources) {
      map.set(s.id, s.title);
    }
    return map;
  }, [sources]);

  const reducedGenRun = useMemo(
    () => reduceWorkflowEvents(activeGenEvents),
    [activeGenEvents],
  );

  const handleSelectNotebook = (nextId: string) => {
    setSelectedNotebookId(nextId);
    if (nextId) {
      navigateTo(`/ui/notebooks/${encodeURIComponent(nextId)}`);
    } else {
      navigateTo('/ui/notebooks');
    }
  };

  const openModalDialog = (dialog: HTMLDialogElement | null) => {
    if (!dialog) return;
    if (typeof dialog.showModal === 'function') {
      if (!dialog.open) {
        dialog.showModal();
      }
    } else {
      dialog.setAttribute('open', '');
    }
  };

  const closeModalDialog = (dialog: HTMLDialogElement | null) => {
    if (!dialog) return;
    dialog.returnValue = '';
    if (typeof dialog.close === 'function' && dialog.open) {
      dialog.close();
    } else {
      dialog.removeAttribute('open');
    }
  };

  const handleConfirmDeleteNotebook = async () => {
    if (!selectedNotebookId) return;
    const deletedTitle = activeNotebook?.title ?? selectedNotebookId;
    setDeleteNotebookBusy(true);
    setErrorBanner(null);
    try {
      const res = await apiFetch(
        `/v1/notebooks/${encodeURIComponent(selectedNotebookId)}`,
        apiToken,
        { method: 'DELETE' },
        tenantId,
      );
      if (!res.ok) {
        const text = await res.text();
        throw new Error(`删除笔记本失败 (${res.status}): ${text}`);
      }
      closeModalDialog(deleteDialogRef.current);
      setSearchHits(null);
      setInspectedChunk(null);
      setActiveGenRunId('');
      const remaining = notebooks.filter((n) => n.id !== selectedNotebookId);
      setNotebooks(remaining);
      const nextId = remaining[0]?.id ?? '';
      handleSelectNotebook(nextId);
      if (!nextId) {
        setActiveNotebook(null);
        setSources([]);
        setArtifacts([]);
      }
      setNoticeBanner(`已删除笔记本「${deletedTitle}」及其关联来源索引。`);
    } catch (err) {
      setErrorBanner(formatApiError(err));
    } finally {
      setDeleteNotebookBusy(false);
    }
  };

  const handleDeleteSource = async (source: NotebookSource) => {
    if (!selectedNotebookId) return;
    setDeletingSourceId(source.id);
    setErrorBanner(null);
    try {
      const res = await apiFetch(
        `/v1/notebooks/${encodeURIComponent(selectedNotebookId)}/sources/${encodeURIComponent(
          source.id,
        )}`,
        apiToken,
        { method: 'DELETE' },
        tenantId,
      );
      if (!res.ok) {
        const text = await res.text();
        throw new Error(`移除来源失败 (${res.status}): ${text}`);
      }
      setSources((prev) => prev.filter((item) => item.id !== source.id));
      setExcludedSourceIds((prev) => prev.filter((id) => id !== source.id));
      setSearchHits((prev) =>
        prev ? prev.filter((hit) => hit.source_id !== source.id) : null,
      );
      if (inspectedChunk?.source_id === source.id) {
        setInspectedChunk(null);
        closeModalDialog(chunkDialogRef.current);
      }
      setNoticeBanner(`已移除来源「${source.title}」，BM25 索引缓存已即时刷新。`);
    } catch (err) {
      setErrorBanner(formatApiError(err));
    } finally {
      setDeletingSourceId(null);
    }
  };

  const handleInspectChunk = async (rawChunkId: string) => {
    if (!selectedNotebookId) return;
    const trimmed = rawChunkId.trim();
    if (!trimmed) {
      setErrorBanner('请输入要溯源的引用分块 ID（例如 source_id:0）。');
      return;
    }
    setChunkLookupBusy(true);
    setErrorBanner(null);
    try {
      const res = await apiFetch(
        `/v1/notebooks/${encodeURIComponent(selectedNotebookId)}/chunks/${encodeURIComponent(
          trimmed,
        )}`,
        apiToken,
        {},
        tenantId,
      );
      if (!res.ok) {
        const text = await res.text();
        throw new Error(`查询分块 ${trimmed} 失败 (${res.status}): ${text}`);
      }
      const chunk = await parseJsonResponse(
        NotebookChunkSchema,
        res,
        'GET /v1/notebooks/:id/chunks/:chunk_id',
      );
      setInspectedChunk(chunk);
      openModalDialog(chunkDialogRef.current);
    } catch (err) {
      setErrorBanner(formatApiError(err));
    } finally {
      setChunkLookupBusy(false);
    }
  };

  const handleCreateNotebook = async (ev: React.FormEvent<HTMLFormElement>) => {
    ev.preventDefault();
    const trimmed = newNotebookTitle.trim();
    if (!trimmed) {
      setErrorBanner('请输入笔记本标题后再创建。');
      return;
    }
    setCreateNotebookBusy(true);
    setErrorBanner(null);
    try {
      const res = await apiFetch(
        '/v1/notebooks',
        apiToken,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ title: trimmed }),
        },
        tenantId,
      );
      if (!res.ok) {
        const text = await res.text();
        throw new Error(`创建笔记本失败 (${res.status}): ${text}`);
      }
      const created = await parseJsonResponse(NotebookSchema, res, 'POST /v1/notebooks');
      setNewNotebookTitle('');
      await loadNotebooksList(created.id);
      handleSelectNotebook(created.id);
      setNoticeBanner(`已创建笔记本「${created.title}」，请添加资料来源。`);
    } catch (err) {
      setErrorBanner(formatApiError(err));
    } finally {
      setCreateNotebookBusy(false);
    }
  };

  const handleRenameNotebook = async (ev: React.FormEvent<HTMLFormElement>) => {
    ev.preventDefault();
    if (!selectedNotebookId) return;
    const trimmed = renameTitleDraft.trim();
    if (!trimmed) {
      setErrorBanner('笔记本标题不能为空。');
      return;
    }
    setRenameNotebookBusy(true);
    setErrorBanner(null);
    try {
      const res = await apiFetch(
        `/v1/notebooks/${encodeURIComponent(selectedNotebookId)}`,
        apiToken,
        {
          method: 'PATCH',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ title: trimmed }),
        },
        tenantId,
      );
      if (!res.ok) {
        const text = await res.text();
        throw new Error(`重命名笔记本失败 (${res.status}): ${text}`);
      }
      const updated = await parseJsonResponse(NotebookSchema, res, 'PATCH /v1/notebooks/:id');
      setActiveNotebook(updated);
      setNotebooks((prev) => prev.map((nb) => (nb.id === updated.id ? updated : nb)));
      setRenamingNotebook(false);
      setNoticeBanner(`已将笔记本重命名为「${updated.title}」。`);
    } catch (err) {
      setErrorBanner(formatApiError(err));
    } finally {
      setRenameNotebookBusy(false);
    }
  };

  const handleDeleteArtifact = async (art: NotebookArtifact) => {
    if (!selectedNotebookId) return;
    setDeletingArtifactId(art.id);
    setErrorBanner(null);
    try {
      const res = await apiFetch(
        `/v1/notebooks/${encodeURIComponent(selectedNotebookId)}/artifacts/${encodeURIComponent(
          art.id,
        )}`,
        apiToken,
        { method: 'DELETE' },
        tenantId,
      );
      if (!res.ok) {
        const text = await res.text();
        throw new Error(`移除成品失败 (${res.status}): ${text}`);
      }
      setArtifacts((prev) => prev.filter((item) => item.id !== art.id));
      setNoticeBanner(`已从笔记本移除成品「${art.name}」。`);
    } catch (err) {
      setErrorBanner(formatApiError(err));
    } finally {
      setDeletingArtifactId(null);
    }
  };

  const handleIngestSource = async (ev: React.FormEvent<HTMLFormElement>) => {
    ev.preventDefault();
    if (!selectedNotebookId) {
      setErrorBanner('请先创建或选择一个笔记本。');
      return;
    }
    setIngestBusy(true);
    setErrorBanner(null);
    try {
      let bodyPayload: Record<string, unknown>;
      if (ingestTab === 'text') {
        if (!sourceTextDraft.trim()) {
          throw new Error('粘贴的资料正文不能为空。');
        }
        bodyPayload = {
          kind: 'text',
          text: sourceTextDraft,
          ...(sourceTitleDraft.trim() ? { title: sourceTitleDraft.trim() } : {}),
        };
      } else if (ingestTab === 'url') {
        const urlTrimmed = sourceUrlDraft.trim();
        if (!/^https?:\/\//i.test(urlTrimmed)) {
          throw new Error('来源 URL 必须以 http:// 或 https:// 开头。');
        }
        bodyPayload = {
          kind: 'url',
          url: urlTrimmed,
          ...(sourceTitleDraft.trim() ? { title: sourceTitleDraft.trim() } : {}),
        };
      } else if (ingestTab === 'file') {
        if (!selectedUploadFile) {
          throw new Error(
            '请选择一个 .pdf、.docx、.xlsx、.pptx、.csv、.tsv、.json、.txt、.md 或 .html 文档文件。',
          );
        }
        const buffer = await selectedUploadFile.arrayBuffer();
        const contentBase64 = encodeBytesToBase64(new Uint8Array(buffer));
        bodyPayload = {
          kind: 'file',
          filename: selectedUploadFile.name,
          content_base64: contentBase64,
          ...(sourceTitleDraft.trim() ? { title: sourceTitleDraft.trim() } : {}),
        };
      } else {
        if (!selectedRecordingFile) {
          throw new Error('请选择一个 .mp3、.wav、.m4a、.ogg 或 .webm 录音文件。');
        }
        const buffer = await selectedRecordingFile.arrayBuffer();
        const contentBase64 = encodeBytesToBase64(new Uint8Array(buffer));
        bodyPayload = {
          kind: 'recording',
          filename: selectedRecordingFile.name,
          content_base64: contentBase64,
          ...(sourceTitleDraft.trim() ? { title: sourceTitleDraft.trim() } : {}),
          ...(recordingModelDraft.trim() ? { model: recordingModelDraft.trim() } : {}),
          ...(recordingLangDraft.trim() ? { language: recordingLangDraft.trim() } : {}),
        };
      }

      const res = await apiFetch(
        `/v1/notebooks/${encodeURIComponent(selectedNotebookId)}/sources`,
        apiToken,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(bodyPayload),
        },
        tenantId,
      );
      if (!res.ok) {
        const text = await res.text();
        throw new Error(`添加来源失败 (${res.status}): ${text}`);
      }
      const created = await parseJsonResponse(
        NotebookSourceSchema,
        res,
        'POST /v1/notebooks/:id/sources',
      );
      setSources((prev) => [...prev, created]);
      setSourceTitleDraft('');
      if (ingestTab === 'text') setSourceTextDraft('');
      if (ingestTab === 'url') setSourceUrlDraft('');
      if (ingestTab === 'file') setSelectedUploadFile(null);
      if (ingestTab === 'recording') setSelectedRecordingFile(null);
      setNoticeBanner(`来源「${created.title}」已提交后台分块与索引构建。`);
    } catch (err) {
      setErrorBanner(formatApiError(err));
    } finally {
      setIngestBusy(false);
    }
  };

  const toggleSourceIncluded = (sourceId: string, included: boolean) => {
    setExcludedSourceIds((prev) => {
      if (included) {
        return prev.filter((id) => id !== sourceId);
      }
      return prev.includes(sourceId) ? prev : [...prev, sourceId];
    });
  };

  const handleSearchNotebook = async (ev: React.FormEvent<HTMLFormElement>) => {
    ev.preventDefault();
    if (!selectedNotebookId) return;
    const q = searchQuery.trim();
    if (!q) {
      setSearchHits(null);
      return;
    }
    if (scopedSourceIds && scopedSourceIds.length === 0) {
      setErrorBanner('请至少勾选一份「已就绪」资料来源后再执行检索。');
      return;
    }
    setSearchBusy(true);
    setErrorBanner(null);
    try {
      const scopeParam =
        scopedSourceIds && scopedSourceIds.length > 0
          ? `&source_ids=${encodeURIComponent(scopedSourceIds.join(','))}`
          : '';
      const res = await apiFetch(
        `/v1/notebooks/${encodeURIComponent(selectedNotebookId)}/search?q=${encodeURIComponent(
          q,
        )}&top_k=6${scopeParam}`,
        apiToken,
        {},
        tenantId,
      );
      if (!res.ok) {
        const text = await res.text();
        throw new Error(`BM25 检索失败 (${res.status}): ${text}`);
      }
      const env = await parseJsonResponse(
        NotebookSearchEnvelopeSchema,
        res,
        'GET /v1/notebooks/:id/search',
      );
      setSearchHits(env.chunks);
    } catch (err) {
      setErrorBanner(formatApiError(err));
    } finally {
      setSearchBusy(false);
    }
  };

  const handleAskNotebook = async (ev: React.FormEvent<HTMLFormElement>) => {
    ev.preventDefault();
    if (!selectedNotebookId) {
      setErrorBanner('请先创建或选择一个笔记本。');
      return;
    }
    const question = askQuestionDraft.trim();
    if (!question) {
      setErrorBanner('请输入要向当前笔记本提问的问题。');
      return;
    }
    if (selectedReadyCount === 0) {
      setErrorBanner('请至少勾选一份「已就绪」资料来源参与源接地问答。');
      return;
    }
    setAskBusy(true);
    setErrorBanner(null);
    try {
      const historyPayload = buildAskHistoryPayload(askThread);
      const body: Record<string, unknown> = { question };
      if (historyPayload.length > 0) {
        body.history = historyPayload;
      }
      if (modelName.trim()) {
        body.model = modelName.trim();
      }
      if (scopedSourceIds) {
        body.source_ids = scopedSourceIds;
      }
      const res = await apiFetch(
        `/v1/notebooks/${encodeURIComponent(selectedNotebookId)}/ask`,
        apiToken,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body),
        },
        tenantId,
      );
      if (!res.ok) {
        const text = await res.text();
        throw new Error(`笔记本问答失败 (${res.status}): ${text}`);
      }
      const parsed = await parseJsonResponse(
        NotebookAskResponseSchema,
        res,
        'POST /v1/notebooks/:id/ask',
      );
      setAskThread((prev) => [...prev, parsed]);
      setAskQuestionDraft('');
    } catch (err) {
      setErrorBanner(formatApiError(err));
    } finally {
      setAskBusy(false);
    }
  };

  const handleSaveAnswerAsSource = async (result: NotebookAskResponse) => {
    if (!selectedNotebookId) return;
    setSaveAskSourceBusy(true);
    setErrorBanner(null);
    try {
      const shortQ = result.question.trim().slice(0, 40);
      const res = await apiFetch(
        `/v1/notebooks/${encodeURIComponent(selectedNotebookId)}/sources`,
        apiToken,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            kind: 'text',
            title: `问答摘录：${shortQ}`,
            text: `问题：${result.question}\n\n回答：\n${result.answer}`,
          }),
        },
        tenantId,
      );
      if (!res.ok) {
        const text = await res.text();
        throw new Error(`保存问答为笔记失败 (${res.status}): ${text}`);
      }
      const created = await parseJsonResponse(
        NotebookSourceSchema,
        res,
        'POST /v1/notebooks/:id/sources',
      );
      setSources((prev) => [...prev, created]);
      setNoticeBanner(`已将回答保存为笔记来源「${created.title}」。`);
    } catch (err) {
      setErrorBanner(formatApiError(err));
    } finally {
      setSaveAskSourceBusy(false);
    }
  };

  const applyHitAsGeneratorFocus = (snippet: string) => {
    const compact = snippet.replace(/\s+/g, ' ').trim().slice(0, 140);
    setParamDrafts((prev) => ({
      ...prev,
      [expandedKind]: {
        ...prev[expandedKind],
        focus: compact,
      },
    }));
    setNoticeBanner(`已将该段落设为「${expandedKind}」生成器的聚焦主题。`);
  };

  const updateGeneratorField = (
    kind: GeneratorKind,
    fieldKey: string,
    value: string | boolean,
  ) => {
    setParamDrafts((prev) => ({
      ...prev,
      [kind]: {
        ...prev[kind],
        [fieldKey]: value,
      },
    }));
  };

  const handleLaunchGenerator = async (spec: StudioGeneratorSpec) => {
    if (!selectedNotebookId) {
      setErrorBanner('请先创建或选择一个笔记本。');
      return;
    }
    if (readySourceCount === 0) {
      setErrorBanner('当前笔记本尚无「已就绪」资料分块，请先添加并等待资料入库完成。');
      return;
    }
    if (selectedReadyCount === 0) {
      setErrorBanner('请至少在左侧勾选一份「已就绪」资料来源参与生成。');
      return;
    }
    const rawDraft = paramDrafts[spec.kind] ?? {};
    const params: Record<string, unknown> = {};
    for (const field of spec.fields) {
      const val = rawDraft[field.key];
      if (field.type === 'checkbox') {
        params[field.key] = Boolean(val);
      } else if (field.type === 'number') {
        const parsed = Number(String(val ?? field.defaultValue).trim());
        if (!Number.isFinite(parsed)) {
          setErrorBanner(`参数「${field.label}」必须是有效数字。`);
          return;
        }
        params[field.key] = parsed;
      } else {
        const strVal = String(val ?? field.defaultValue).trim();
        if (strVal) {
          params[field.key] = strVal;
        }
      }
    }

    setGenerateBusyKind(spec.kind);
    setErrorBanner(null);
    setNoticeBanner(null);
    try {
      const body: Record<string, unknown> = {
        kind: spec.kind,
        params,
      };
      if (modelName.trim()) {
        body.model = modelName.trim();
      }
      if (scopedSourceIds) {
        body.source_ids = scopedSourceIds;
      }
      const res = await apiFetch(
        `/v1/notebooks/${encodeURIComponent(selectedNotebookId)}/generate`,
        apiToken,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body),
        },
        tenantId,
      );
      if (!res.ok) {
        const text = await res.text();
        throw new Error(`启动生成器失败 (${res.status}): ${text}`);
      }
      const gen = await parseJsonResponse(
        NotebookGenerateResponseSchema,
        res,
        'POST /v1/notebooks/:id/generate',
      );
      setActiveGenEvents([]);
      setActiveGenKind(spec.kind);
      setActiveGenStatus(gen.status ?? 'running');
      setActiveGenRunId(gen.run_id);
    } catch (err) {
      setErrorBanner(formatApiError(err));
    } finally {
      setGenerateBusyKind(null);
    }
  };

  const filteredArtifacts = useMemo(() => {
    const reversed = [...artifacts].reverse();
    if (artifactFilter === 'all') return reversed;
    return reversed.filter((a) => a.kind === artifactFilter);
  }, [artifacts, artifactFilter]);

  const completedGenNodes = useMemo(
    () =>
      Object.values(reducedGenRun.telemetryByNode).filter(
        (t) => t.status === 'succeeded' || t.status === 'skipped',
      ).length,
    [reducedGenRun.telemetryByNode],
  );

  return (
    <div className="nb-shell" data-testid="notebook-workspace-page">
      <StudioSubpageHeader
        eyebrow="Yanshi Content Studio"
        title="多源内容笔记本工作台"
        subtitle="汇集网页、PDF/DOCX/XLSX/PPTX/CSV 文档、笔记与语音录音构建本地 BM25 知识索引，支持勾选来源多轮追问溯源与 12 套多模态生成器"
        activeSection="notebooks"
        actions={
          <div className="nb-header-controls">
            <label className="nb-inline-control" htmlFor="nb-tenant-input">
              <span>租户</span>
              <input
                id="nb-tenant-input"
                name="tenant"
                type="text"
                autoCorrect="off"
                autoCapitalize="none"
                spellCheck={false}
                value={tenantId}
                onChange={(ev) => setTenantId(ev.target.value)}
              />
            </label>
            <label className="nb-inline-control" htmlFor="nb-token-input">
              <span>Token</span>
              <input
                id="nb-token-input"
                name="token"
                type="password"
                autoComplete="off"
                placeholder="可选 Bearer"
                value={apiToken}
                onChange={(ev) => onTokenChange(ev.target.value)}
              />
            </label>
          </div>
        }
      />

      {errorBanner && (
        <div className="studio-alert studio-alert--error" role="alert">
          <span>{errorBanner}</span>
          <button type="button" onClick={() => setErrorBanner(null)}>
            关闭
          </button>
        </div>
      )}

      {noticeBanner && (
        <div className="studio-alert studio-alert--info" role="status">
          <span>{noticeBanner}</span>
          <button type="button" onClick={() => setNoticeBanner(null)}>
            知道了
          </button>
        </div>
      )}

      <main className="nb-workbench">
        {/* Column 1: Notebook Shelf, Source Ingestion & BM25 Search */}
        <aside className="nb-column nb-column--sources" aria-label="笔记本与资料来源库">
          <section className="nb-card">
            <div className="nb-card__head">
              <h2 id="nb-picker-heading">当前笔记本</h2>
              <span>{notebooks.length} 本</span>
            </div>

            {notebooks.length > 0 && (
              <div className="nb-field">
                <div className="nb-picker-label-row">
                  <label htmlFor="nb-select">切换已有笔记本</label>
                  {activeNotebook && (
                    <div className="nb-picker-actions">
                      <button
                        type="button"
                        className="studio-link-btn"
                        data-testid="open-rename-notebook-btn"
                        aria-expanded={renamingNotebook}
                        aria-controls="nb-rename-form"
                        onClick={() => {
                          setRenameTitleDraft(activeNotebook.title);
                          setRenamingNotebook((prev) => !prev);
                        }}
                      >
                        {renamingNotebook ? '收起重命名' : '重命名'}
                      </button>
                      <button
                        type="button"
                        className="studio-link-btn nb-danger-link"
                        data-testid="open-delete-notebook-btn"
                        aria-label={`删除笔记本 ${activeNotebook.title}`}
                        disabled={deleteNotebookBusy}
                        onClick={() => openModalDialog(deleteDialogRef.current)}
                      >
                        删除笔记本
                      </button>
                    </div>
                  )}
                </div>
                <select
                  id="nb-select"
                  name="notebook_id"
                  data-testid="notebook-selector"
                  value={selectedNotebookId}
                  onChange={(ev) => handleSelectNotebook(ev.target.value)}
                >
                  {notebooks.map((nb) => (
                    <option key={nb.id} value={nb.id}>
                      {nb.title}
                    </option>
                  ))}
                </select>
              </div>
            )}

            {activeNotebook && renamingNotebook && (
              <form
                id="nb-rename-form"
                className="nb-create-form"
                method="POST"
                action={`/v1/notebooks/${encodeURIComponent(activeNotebook.id)}`}
                aria-label="重命名当前笔记本"
                onSubmit={(ev) => void handleRenameNotebook(ev)}
              >
                <label htmlFor="nb-rename-title">重命名当前笔记本</label>
                <div className="nb-create-form__row">
                  <input
                    id="nb-rename-title"
                    name="title"
                    type="text"
                    enterKeyHint="done"
                    required
                    data-testid="rename-notebook-input"
                    value={renameTitleDraft}
                    onChange={(ev) => setRenameTitleDraft(ev.target.value)}
                  />
                  <button
                    type="submit"
                    className="studio-btn-primary studio-btn-sm"
                    data-testid="save-rename-notebook-btn"
                    disabled={renameNotebookBusy}
                  >
                    {renameNotebookBusy ? '保存中…' : '保存'}
                  </button>
                  <button
                    type="button"
                    className="studio-btn-secondary studio-btn-sm"
                    disabled={renameNotebookBusy}
                    onClick={() => setRenamingNotebook(false)}
                  >
                    取消
                  </button>
                </div>
              </form>
            )}

            <form
              className="nb-create-form"
              method="POST"
              action="/v1/notebooks"
              aria-labelledby="nb-picker-heading"
              onSubmit={(ev) => void handleCreateNotebook(ev)}
            >
              <label htmlFor="nb-new-title">新建主题笔记本</label>
              <div className="nb-create-form__row">
                <input
                  id="nb-new-title"
                  name="title"
                  type="text"
                  data-testid="new-notebook-title-input"
                  placeholder="例如：Rust 异步运行时调研"
                  value={newNotebookTitle}
                  onChange={(ev) => setNewNotebookTitle(ev.target.value)}
                />
                <button
                  type="submit"
                  className="studio-btn-primary studio-btn-sm"
                  data-testid="create-notebook-btn"
                  disabled={createNotebookBusy}
                >
                  {createNotebookBusy ? '创建中…' : '新建'}
                </button>
              </div>
            </form>
          </section>

          <section className="nb-card">
            <div className="nb-card__head">
              <h2 id="nb-ingest-heading">添加资料来源</h2>
              <span>
                {readySourceCount}/{sources.length} 已就绪
              </span>
            </div>

            <div
              className="nb-tabs"
              role="tablist"
              aria-label="资料导入方式"
              onKeyDown={(ev) => {
                const idx = SOURCE_INGEST_TABS.findIndex((t) => t.id === ingestTab);
                if (idx < 0) return;
                let nextIdx = -1;
                if (ev.key === 'ArrowRight') {
                  nextIdx = (idx + 1) % SOURCE_INGEST_TABS.length;
                } else if (ev.key === 'ArrowLeft') {
                  nextIdx = (idx - 1 + SOURCE_INGEST_TABS.length) % SOURCE_INGEST_TABS.length;
                } else if (ev.key === 'Home') {
                  nextIdx = 0;
                } else if (ev.key === 'End') {
                  nextIdx = SOURCE_INGEST_TABS.length - 1;
                }
                if (nextIdx >= 0) {
                  ev.preventDefault();
                  const nextTab = SOURCE_INGEST_TABS[nextIdx]!.id;
                  setIngestTab(nextTab);
                  const btn = sourceTabRefs.current[nextTab];
                  btn?.focus();
                  btn?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
                }
              }}
            >
              {SOURCE_INGEST_TABS.map((tab) => {
                const selected = ingestTab === tab.id;
                return (
                  <button
                    key={tab.id}
                    ref={(el) => {
                      sourceTabRefs.current[tab.id] = el;
                    }}
                    type="button"
                    role="tab"
                    id={`nb-tab-${tab.id}`}
                    tabIndex={selected ? 0 : -1}
                    aria-selected={selected}
                    onClick={(ev) => {
                      setIngestTab(tab.id);
                      ev.currentTarget.scrollIntoView({ block: 'nearest', inline: 'nearest' });
                    }}
                  >
                    {tab.label}
                  </button>
                );
              })}
            </div>

            <form
              className="nb-ingest-form"
              method="POST"
              action="/v1/notebooks/sources"
              aria-labelledby="nb-ingest-heading"
              onSubmit={(ev) => void handleIngestSource(ev)}
            >
              <div role="tabpanel" aria-labelledby="nb-tab-text" hidden={ingestTab !== 'text'}>
                <div className="nb-tabpanel-inner">
                  <div className="nb-field">
                    <label htmlFor="nb-src-text-title">来源标题（可选）</label>
                    <input
                      id="nb-src-text-title"
                      name="text_title"
                      type="text"
                      placeholder="留空自动截取首句作为标题"
                      value={sourceTitleDraft}
                      onChange={(ev) => setSourceTitleDraft(ev.target.value)}
                    />
                  </div>
                  <div className="nb-field">
                    <label htmlFor="nb-src-text-body">资料正文（自动按段落切块）</label>
                    <textarea
                      id="nb-src-text-body"
                      name="text"
                      rows={5}
                      data-testid="source-text-input"
                      placeholder="粘贴论文摘录、会议纪要、架构说明或课堂笔记…"
                      value={sourceTextDraft}
                      onChange={(ev) => setSourceTextDraft(ev.target.value)}
                    />
                  </div>
                </div>
              </div>

              <div role="tabpanel" aria-labelledby="nb-tab-url" hidden={ingestTab !== 'url'}>
                <div className="nb-tabpanel-inner">
                  <div className="nb-field">
                    <label htmlFor="nb-src-url">网页链接（内置 SSRF 安全抓取与正文提取）</label>
                    <input
                      id="nb-src-url"
                      name="url"
                      type="url"
                      autoCorrect="off"
                      autoCapitalize="none"
                      spellCheck={false}
                      placeholder="https://example.org/article"
                      value={sourceUrlDraft}
                      onChange={(ev) => setSourceUrlDraft(ev.target.value)}
                    />
                  </div>
                  <div className="nb-field">
                    <label htmlFor="nb-src-url-title">备注标题（可选）</label>
                    <input
                      id="nb-src-url-title"
                      name="url_title"
                      type="text"
                      placeholder="留空默认使用 URL"
                      value={sourceTitleDraft}
                      onChange={(ev) => setSourceTitleDraft(ev.target.value)}
                    />
                  </div>
                </div>
              </div>

              <div role="tabpanel" aria-labelledby="nb-tab-file" hidden={ingestTab !== 'file'}>
                <div className="nb-tabpanel-inner">
                  <div className="nb-field">
                    <label htmlFor="nb-src-file">
                      选择本地文档、表格或课件（支持 .pdf / .docx / .xlsx / .pptx / .csv / .tsv / .json / .md / .txt / .html，≤10MB）
                    </label>
                    <input
                      id="nb-src-file"
                      name="file"
                      type="file"
                      accept=".pdf,.docx,.xlsx,.pptx,.csv,.tsv,.json,.txt,.md,.markdown,.html,.htm,application/pdf,application/vnd.openxmlformats-officedocument.wordprocessingml.document,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet,application/vnd.openxmlformats-officedocument.presentationml.presentation,text/csv,text/tab-separated-values,application/json,text/plain,text/markdown,text/html"
                      onChange={(ev) => setSelectedUploadFile(ev.target.files?.[0] ?? null)}
                    />
                  </div>
                </div>
              </div>

              <div
                role="tabpanel"
                aria-labelledby="nb-tab-recording"
                hidden={ingestTab !== 'recording'}
              >
                <div className="nb-tabpanel-inner">
                  <div className="nb-field">
                    <label htmlFor="nb-src-recording">
                      选择会议或访谈录音（支持 .mp3 / .wav / .m4a / .ogg / .webm，≤10MB）
                    </label>
                    <input
                      id="nb-src-recording"
                      name="recording"
                      type="file"
                      accept=".mp3,.wav,.m4a,.ogg,.flac,.webm,.aac,audio/*"
                      onChange={(ev) => setSelectedRecordingFile(ev.target.files?.[0] ?? null)}
                    />
                  </div>
                  <div className="nb-field">
                    <label htmlFor="nb-src-asr-model">ASR 语音转写模型（可选）</label>
                    <input
                      id="nb-src-asr-model"
                      name="asr_model"
                      type="text"
                      autoCorrect="off"
                      autoCapitalize="none"
                      spellCheck={false}
                      placeholder="例如 gemini-2.5-flash-asr / whisper-1（留空读 YANSHI_ASR_MODEL）"
                      value={recordingModelDraft}
                      onChange={(ev) => setRecordingModelDraft(ev.target.value)}
                    />
                  </div>
                  <div className="nb-field">
                    <label htmlFor="nb-src-asr-lang">语种提示（可选，如 zh / en）</label>
                    <input
                      id="nb-src-asr-lang"
                      name="asr_language"
                      type="text"
                      autoCorrect="off"
                      autoCapitalize="none"
                      spellCheck={false}
                      placeholder="留空自动识别语种"
                      value={recordingLangDraft}
                      onChange={(ev) => setRecordingLangDraft(ev.target.value)}
                    />
                  </div>
                </div>
              </div>

              <button
                type="submit"
                className="studio-btn-primary"
                data-testid="submit-source-btn"
                disabled={ingestBusy || !selectedNotebookId}
              >
                {ingestBusy ? '正在提交入库…' : '+ 加入当前笔记本'}
              </button>
            </form>

            <div className="nb-source-list-wrap">
              {sources.length === 0 ? (
                <p className="nb-empty-hint">
                  {activeNotebook
                    ? '尚未添加资料。导入文本、网页或文档/录音文件后即可检索、问答和生成成品。'
                    : '请先在上方新建一个主题笔记本。'}
                </p>
              ) : (
                <>
                  <div className="nb-source-scope-bar" data-testid="source-scope-bar">
                    <span>
                      参与范围：已选 {selectedReadyCount}/{readySourceCount} 份就绪资料
                    </span>
                    <div className="nb-source-scope-bar__actions">
                      {excludedSourceIds.length > 0 ? (
                        <button
                          type="button"
                          className="studio-link-btn"
                          onClick={() => setExcludedSourceIds([])}
                        >
                          全选来源
                        </button>
                      ) : (
                        sources.length > 1 && (
                          <button
                            type="button"
                            className="studio-link-btn"
                            onClick={() => setExcludedSourceIds(sources.map((s) => s.id))}
                          >
                            取消全选
                          </button>
                        )
                      )}
                    </div>
                  </div>
                  <ul className="nb-source-list" role="list" aria-label="已添加的资料来源">
                    {sources.map((src) => {
                      const included = !excludedSourceSet.has(src.id);
                      const checkId = `nb-src-check-${src.id}`;
                      return (
                        <li
                          key={src.id}
                          className={`nb-source-item ${included ? '' : 'nb-source-item--excluded'}`}
                        >
                          <div className="nb-source-item__top">
                            <label htmlFor={checkId} className="nb-source-check-label">
                              <input
                                id={checkId}
                                name={`source_scope_${src.id}`}
                                type="checkbox"
                                className="nb-source-checkbox"
                                data-testid={`source-scope-checkbox-${src.id}`}
                                checked={included}
                                onChange={(ev) => toggleSourceIncluded(src.id, ev.target.checked)}
                              />
                              <span>{src.title}</span>
                            </label>
                            <div className="nb-source-item__actions">
                              <span className="nb-source-status" data-status={src.status}>
                                {sourceStatusLabel(src.status)}
                              </span>
                              <button
                                type="button"
                                className="studio-link-btn nb-danger-link"
                                data-testid={`delete-source-${src.id}`}
                                aria-label={`移除 ${src.title}`}
                                disabled={deletingSourceId === src.id}
                                onClick={() => void handleDeleteSource(src)}
                              >
                                {deletingSourceId === src.id ? '移除中…' : '移除'}
                              </button>
                            </div>
                          </div>
                          <div className="nb-source-item__meta">
                            <span>{src.kind.toUpperCase()}</span>
                            <span>{formatTime(src.created_at)}</span>
                            {src.sha256 ? <code>#{src.sha256.slice(0, 8)}</code> : null}
                            {src.status === 'ready' && (
                              <button
                                type="button"
                                className="studio-link-btn"
                                aria-label={`查看首段 ${src.title}`}
                                onClick={() => void handleInspectChunk(`${src.id}:0`)}
                              >
                                查看首段
                              </button>
                            )}
                          </div>
                          {src.error ? <p className="nb-source-item__error">{src.error}</p> : null}
                        </li>
                      );
                    })}
                  </ul>
                </>
              )}
            </div>
          </section>

          <section className="nb-card">
            <div className="nb-card__head">
              <h2 id="nb-search-heading">BM25 段落溯源检索</h2>
            </div>
            <search>
              <form
                className="nb-search-form"
                method="GET"
                action="/v1/notebooks/search"
                aria-labelledby="nb-search-heading"
                onSubmit={(ev) => void handleSearchNotebook(ev)}
              >
                <label htmlFor="nb-search-q" className="nb-sr-only">
                  检索笔记本原文分块
                </label>
                <input
                  id="nb-search-q"
                  name="q"
                  type="search"
                  data-testid="notebook-search-input"
                  autoCorrect="off"
                  autoCapitalize="none"
                  spellCheck={false}
                  placeholder="输入关键词检索原文分块…"
                  value={searchQuery}
                  onChange={(ev) => setSearchQuery(ev.target.value)}
                />
                <button
                  type="submit"
                  className="studio-btn-secondary studio-btn-sm"
                  data-testid="notebook-search-submit"
                  disabled={searchBusy || !selectedNotebookId}
                >
                  {searchBusy ? '检索中…' : '检索'}
                </button>
              </form>
            </search>

            {searchHits !== null && (
              <div className="nb-search-results">
                {searchHits.length === 0 ? (
                  <p className="nb-empty-hint">未找到匹配分块，请尝试调整关键词。</p>
                ) : (
                  <ul className="nb-hit-list" role="list">
                    {searchHits.map((hit) => (
                      <li key={hit.chunk_id} className="nb-hit-card">
                        <div className="nb-hit-card__head">
                          <span>{sourceTitleById.get(hit.source_id) ?? '资料分块'}</span>
                          <code>BM25 {hit.score.toFixed(2)}</code>
                        </div>
                        <p>{hit.content}</p>
                        <div className="nb-hit-card__foot">
                          <button
                            type="button"
                            className="studio-link-btn"
                            aria-label={`查看分块 ${hit.chunk_id}`}
                            onClick={() => void handleInspectChunk(hit.chunk_id)}
                          >
                            查看分块 {hit.chunk_id.slice(-8)}
                          </button>
                          <button
                            type="button"
                            className="studio-link-btn"
                            onClick={() => applyHitAsGeneratorFocus(hit.content)}
                          >
                            设为生成焦点
                          </button>
                        </div>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            )}

            <form
              className="nb-chunk-lookup-form"
              onSubmit={(ev) => {
                ev.preventDefault();
                void handleInspectChunk(chunkLookupId);
              }}
            >
              <label htmlFor="nb-chunk-id-input">按引用分块 ID 溯源原文</label>
              <div className="nb-search-form">
                <input
                  id="nb-chunk-id-input"
                  name="chunk_id"
                  type="text"
                  data-testid="chunk-lookup-input"
                  autoCorrect="off"
                  autoCapitalize="none"
                  spellCheck={false}
                  placeholder="例如 550e8400-...:0"
                  value={chunkLookupId}
                  onChange={(ev) => setChunkLookupId(ev.target.value)}
                />
                <button
                  type="submit"
                  className="studio-btn-secondary studio-btn-sm"
                  data-testid="chunk-lookup-btn"
                  disabled={chunkLookupBusy || !selectedNotebookId}
                >
                  {chunkLookupBusy ? '定位中…' : '溯源'}
                </button>
              </div>
            </form>
          </section>
        </aside>

        {/* Column 2: Grounded Q&A, 12 Content Studio Generators & Live Run Tracker */}
        <section className="nb-column nb-column--studio" aria-label="Content Studio 生成器矩阵">
          <div className="nb-studio-bar">
            <div>
              <h2>Content Studio 生成器工作台</h2>
              <p>
                {activeNotebook
                  ? `基于「${activeNotebook.title}」已勾选的 ${selectedReadyCount}/${readySourceCount} 份就绪资料，提问溯源或生成结构化交付物`
                  : '创建并选择笔记本后即可向资料提问或一键生成多模态成品'}
              </p>
            </div>
            <div className="nb-model-box">
              <label htmlFor="nb-model-input">生成模型 (model)</label>
              <input
                id="nb-model-input"
                name="model"
                type="text"
                data-testid="notebook-model-input"
                autoCorrect="off"
                autoCapitalize="none"
                spellCheck={false}
                placeholder="例如 gpt-4o / kimi-k2.6（留空读 YANSHI_GENERATE_MODEL）"
                value={modelName}
                onChange={(ev) => setModelName(ev.target.value)}
              />
            </div>
          </div>

          <section
            className="nb-card nb-ask-card"
            data-testid="notebook-ask-card"
            aria-labelledby="nb-ask-heading"
          >
            <div className="nb-card__head">
              <h3 id="nb-ask-heading">向笔记本提问（源接地多轮问答）</h3>
              <div className="nb-picker-actions">
                <span>
                  {scopedSourceIds
                    ? `限定 ${selectedReadyCount} 份已选来源`
                    : `覆盖全部 ${readySourceCount} 份就绪来源`}
                  {askThread.length > 0 ? ` · 对话 ${askThread.length} 轮` : ''}
                </span>
                {askThread.length > 0 && (
                  <button
                    type="button"
                    className="studio-link-btn"
                    data-testid="clear-ask-thread-btn"
                    onClick={() => setAskThread([])}
                  >
                    清空对话
                  </button>
                )}
              </div>
            </div>
            <form
              className="nb-ask-form"
              method="POST"
              action="/v1/notebooks/ask"
              aria-labelledby="nb-ask-heading"
              onSubmit={(ev) => void handleAskNotebook(ev)}
            >
              <label htmlFor="nb-ask-input" className="nb-sr-only">
                针对已勾选笔记本资料输入问题或追问
              </label>
              <input
                id="nb-ask-input"
                name="question"
                type="text"
                enterKeyHint="send"
                data-testid="notebook-ask-input"
                placeholder={
                  askThread.length > 0
                    ? '继续追问上一轮回答（例如：那它的内存开销与权衡是什么？）…'
                    : '针对已选资料提问（例如：各方案的核心指标与权衡差异是什么？）…'
                }
                value={askQuestionDraft}
                onChange={(ev) => setAskQuestionDraft(ev.target.value)}
              />
              <button
                type="submit"
                className="studio-btn-primary studio-btn-sm"
                data-testid="notebook-ask-submit"
                disabled={askBusy || !selectedNotebookId || selectedReadyCount === 0}
              >
                {askBusy ? '溯源推理中…' : askThread.length > 0 ? '继续追问' : '提问溯源'}
              </button>
            </form>
            {askThread.length > 0 && (
              <ol className="nb-ask-thread" role="list" data-testid="notebook-ask-thread">
                {askThread.map((turn, idx) => (
                  <li key={`${turn.question}-${idx}`}>
                    <NotebookAskResultCard
                      result={turn}
                      turnIndex={idx}
                      sourceTitleById={sourceTitleById}
                      onInspectChunk={(chunkId) => void handleInspectChunk(chunkId)}
                      onUseAsFocus={(answerText) => applyHitAsGeneratorFocus(answerText)}
                      onSaveAsSource={(res) => void handleSaveAnswerAsSource(res)}
                      saveBusy={saveAskSourceBusy}
                    />
                  </li>
                ))}
              </ol>
            )}
          </section>

          {activeGenRunId && (
            <div className="nb-live-run-ribbon" role="status" data-testid="notebook-live-run">
              <div className="nb-live-run-ribbon__info">
                <span
                  className={`nb-run-dot ${
                    ['succeeded', 'failed', 'cancelled'].includes(activeGenStatus)
                      ? 'nb-run-dot--done'
                      : 'nb-run-dot--running'
                  }`}
                />
                <strong>
                  {activeGenKind ?? 'workflow'} · 状态：{activeGenStatus || 'running'}
                </strong>
                <span>已完成 {completedGenNodes} 个节点</span>
                {reducedGenRun.totalCostUsd > 0 && (
                  <span className="studio-spend-pill">
                    💰 {formatSpendUsd(reducedGenRun.totalCostUsd)}
                  </span>
                )}
              </div>
              <div className="nb-live-run-ribbon__actions">
                <SpaLink
                  className="studio-btn-secondary studio-btn-sm"
                  to={`/ui/studio?run_id=${encodeURIComponent(activeGenRunId)}${
                    activeGenKind ? `&preset=${encodeURIComponent(activeGenKind)}` : ''
                  }`}
                >
                  在 Studio 拓扑台中查看 / 续跑
                </SpaLink>
              </div>
            </div>
          )}

          <div className="nb-generator-grid">
            {STUDIO_GENERATORS.map((spec) => {
              const isExpanded = expandedKind === spec.kind;
              const isBusy = generateBusyKind === spec.kind;
              const draft = paramDrafts[spec.kind] ?? {};
              return (
                <article
                  key={spec.kind}
                  className={`nb-gen-card ${isExpanded ? 'nb-gen-card--expanded' : ''}`}
                  data-testid={`generator-card-${spec.kind}`}
                >
                  <div className="nb-gen-card__head">
                    <div>
                      <span className="nb-gen-badge">{spec.deliverableBadge}</span>
                      <h3>{spec.title}</h3>
                    </div>
                    <button
                      type="button"
                      className="studio-link-btn"
                      aria-expanded={isExpanded}
                      onClick={() => setExpandedKind(spec.kind)}
                    >
                      {isExpanded ? '参数已展开' : '调整参数'}
                    </button>
                  </div>

                  <p className="nb-gen-card__desc">{spec.summary}</p>

                  {isExpanded && (
                    <div className="nb-gen-params">
                      {spec.fields.map((field) => {
                        const controlId = `nb-gen-${spec.kind}-${field.key}`;
                        const currentVal = draft[field.key] ?? field.defaultValue;
                        if (field.type === 'checkbox') {
                          return (
                            <div key={field.key} className="nb-gen-checkbox">
                              <input
                                id={controlId}
                                name={field.key}
                                type="checkbox"
                                checked={Boolean(currentVal)}
                                onChange={(ev) =>
                                  updateGeneratorField(spec.kind, field.key, ev.target.checked)
                                }
                              />
                              <label htmlFor={controlId}>{field.label}</label>
                            </div>
                          );
                        }
                        return (
                          <div key={field.key} className="nb-field">
                            <label htmlFor={controlId}>{field.label}</label>
                            <input
                              id={controlId}
                              name={field.key}
                              type={field.type === 'number' ? 'number' : 'text'}
                              min={field.min}
                              max={field.max}
                              value={String(currentVal)}
                              onChange={(ev) =>
                                updateGeneratorField(spec.kind, field.key, ev.target.value)
                              }
                            />
                          </div>
                        );
                      })}
                    </div>
                  )}

                  <div className="nb-gen-card__foot">
                    <SpaLink
                      className="studio-link-btn"
                      to={`/ui/studio?preset=${encodeURIComponent(spec.kind)}`}
                    >
                      打开 YAML 编排源码
                    </SpaLink>
                    <button
                      type="button"
                      className="studio-btn-primary studio-btn-sm"
                      data-testid={`launch-generator-${spec.kind}`}
                      disabled={isBusy || !selectedNotebookId || selectedReadyCount === 0}
                      onClick={() => void handleLaunchGenerator(spec)}
                    >
                      {isBusy ? '提交中…' : '✨ 生成成品'}
                    </button>
                  </div>
                </article>
              );
            })}
          </div>
        </section>

        {/* Column 3: Aggregated Multimodal Deliverable Gallery */}
        <aside className="nb-column nb-column--gallery" aria-label="笔记本多模态成品画廊">
          <div className="nb-card nb-card--gallery">
            <div className="nb-card__head">
              <h2>成品画廊 ({filteredArtifacts.length})</h2>
              <button
                type="button"
                className="studio-link-btn"
                disabled={!selectedNotebookId}
                onClick={() => void loadNotebookDetailAndArtifacts(selectedNotebookId)}
              >
                刷新
              </button>
            </div>

            <div className="nb-gallery-filters" role="group" aria-label="按生成类型筛选成品">
              <button
                type="button"
                className={artifactFilter === 'all' ? 'active' : ''}
                onClick={() => setArtifactFilter('all')}
              >
                全部 ({artifacts.length})
              </button>
              {STUDIO_GENERATORS.map((g) => {
                const count = artifacts.filter((a) => a.kind === g.kind).length;
                if (count === 0 && artifactFilter !== g.kind) return null;
                return (
                  <button
                    key={g.kind}
                    type="button"
                    className={artifactFilter === g.kind ? 'active' : ''}
                    onClick={() => setArtifactFilter(g.kind)}
                  >
                    {g.title} ({count})
                  </button>
                );
              })}
            </div>

            <div className="nb-gallery-list">
              {filteredArtifacts.length === 0 ? (
                <div className="nb-empty-gallery">
                  <h3>尚无已生成的交付物</h3>
                  <p>
                    在左侧导入至少一份资料并等待状态变为「已就绪」，即可在中间点击任意生成器产出闪卡、导图、报告、课件或音视频。
                  </p>
                </div>
              ) : (
                (() => {
                  const renderedDeliverableRuns = new Set<string>();
                  return filteredArtifacts.map((art) => {
                    const mediaItem = notebookArtifactToMediaItem(art);
                    const showDeliverableBar =
                      Boolean(art.deliverable) && !renderedDeliverableRuns.has(art.run_id);
                    if (showDeliverableBar) {
                      renderedDeliverableRuns.add(art.run_id);
                    }
                    return (
                      <div key={art.id} className="nb-artifact-block">
                        <div className="nb-artifact-block__meta">
                          <span className="nb-gen-badge">{art.kind}</span>
                          <small>{formatTime(art.created_at)}</small>
                          <div className="nb-picker-actions">
                            <SpaLink
                              className="studio-link-btn"
                              to={`/ui/studio?run_id=${encodeURIComponent(
                                art.run_id,
                              )}&preset=${encodeURIComponent(art.kind)}`}
                            >
                              查看运行 #{art.run_id.slice(0, 8)}
                            </SpaLink>
                            <button
                              type="button"
                              className="studio-link-btn nb-danger-link"
                              data-testid={`delete-artifact-${art.id}`}
                              aria-label={`移除成品 ${art.name}`}
                              disabled={deletingArtifactId === art.id}
                              onClick={() => void handleDeleteArtifact(art)}
                            >
                              {deletingArtifactId === art.id ? '移除中…' : '移除'}
                            </button>
                          </div>
                        </div>
                        {showDeliverableBar && art.deliverable && (
                          <NotebookDeliverableCitationBar
                            deliverable={art.deliverable}
                            sourceTitleById={sourceTitleById}
                            onInspectChunk={(chunkId) => void handleInspectChunk(chunkId)}
                          />
                        )}
                        <AuthorizedMedia
                          item={mediaItem}
                          apiToken={apiToken}
                          tenantId={tenantId}
                        />
                      </div>
                    );
                  });
                })()
              )}
            </div>
          </div>
        </aside>
      </main>

      <dialog
        ref={deleteDialogRef}
        className="nb-dialog"
        data-testid="delete-notebook-dialog"
        aria-labelledby="nb-delete-dialog-title"
        onClose={(ev) => {
          ev.currentTarget.returnValue = '';
        }}
      >
        <h2 id="nb-delete-dialog-title">确认删除笔记本？</h2>
        <p>
          将永久移除笔记本「{activeNotebook?.title ?? ''}」及其名下全部 {sources.length}{' '}
          份资料分块与成品挂载记录，此操作不可撤销。
        </p>
        <div className="nb-dialog__actions">
          <form method="dialog">
            <button type="submit" value="cancel" className="studio-btn-secondary studio-btn-sm" autoFocus>
              取消
            </button>
          </form>
          <button
            type="button"
            className="studio-btn-primary studio-btn-sm nb-btn-danger"
            data-testid="confirm-delete-notebook-btn"
            disabled={deleteNotebookBusy}
            onClick={() => void handleConfirmDeleteNotebook()}
          >
            {deleteNotebookBusy ? '删除中…' : '确认删除'}
          </button>
        </div>
      </dialog>

      <dialog
        ref={chunkDialogRef}
        className="nb-dialog nb-dialog--wide"
        data-testid="chunk-preview-dialog"
        aria-labelledby="nb-chunk-dialog-title"
        onClose={(ev) => {
          ev.currentTarget.returnValue = '';
        }}
      >
        <div className="nb-dialog__head">
          <h2 id="nb-chunk-dialog-title">
            引用分块原文 {inspectedChunk ? `· #${inspectedChunk.ordinal}` : ''}
          </h2>
          <form method="dialog">
            <button type="submit" value="close" className="studio-btn-secondary studio-btn-sm" autoFocus>
              关闭
            </button>
          </form>
        </div>
        {inspectedChunk && (
          <div className="nb-chunk-preview">
            <div className="nb-source-item__meta">
              <span>{sourceTitleById.get(inspectedChunk.source_id) ?? '所属来源'}</span>
              <code>{inspectedChunk.chunk_id}</code>
              {typeof inspectedChunk.metadata?.start_line === 'number' &&
                typeof inspectedChunk.metadata?.end_line === 'number' && (
                  <span>
                    行号 {inspectedChunk.metadata.start_line}–{inspectedChunk.metadata.end_line}
                  </span>
                )}
            </div>
            <pre className="nb-chunk-preview__body">{inspectedChunk.content}</pre>
          </div>
        )}
      </dialog>
    </div>
  );
}
