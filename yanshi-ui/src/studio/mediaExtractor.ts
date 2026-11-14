import { resolveApiUrl } from '../lib/api';
import type { ArtifactSummary } from '../schemas';

export type MediaPreviewKind = 'image' | 'audio' | 'video' | 'html' | 'document' | 'subtitle';

export interface MediaPreviewItem {
  key: string;
  nodeId: string;
  label: string;
  kind: MediaPreviewKind;
  /** Gateway-relative path (`/v1/runs/.../preview?path=...` or `/v1/runs/.../artifacts/...`). */
  apiPath: string;
  /** Resolved URL when direct unauthenticated `<img/video/audio src>` can be used. */
  directUrl: string;
  mimeType?: string;
  sizeBytes?: number | null;
}

const EXT_KIND_MAP: Record<string, MediaPreviewKind> = {
  png: 'image',
  jpg: 'image',
  jpeg: 'image',
  webp: 'image',
  gif: 'image',
  svg: 'image',
  wav: 'audio',
  mp3: 'audio',
  ogg: 'audio',
  m4a: 'audio',
  mp4: 'video',
  webm: 'video',
  html: 'html',
  htm: 'html',
  pdf: 'document',
  pptx: 'document',
  docx: 'document',
  srt: 'subtitle',
  vtt: 'subtitle',
};

function classifyPath(filePath: string, mimeType?: string | null): MediaPreviewKind | null {
  const mime = (mimeType ?? '').toLowerCase();
  if (mime.startsWith('image/')) return 'image';
  if (mime.startsWith('audio/')) return 'audio';
  if (mime.startsWith('video/')) return 'video';
  if (mime.startsWith('text/html')) return 'html';
  if (mime.includes('presentation') || mime.includes('pdf') || mime.includes('wordprocessing')) {
    return 'document';
  }
  if (mime.includes('subrip') || mime.startsWith('text/vtt')) return 'subtitle';

  const clean = filePath.split('?')[0].split('#')[0];
  const dot = clean.lastIndexOf('.');
  if (dot < 0) return null;
  const ext = clean.slice(dot + 1).toLowerCase();
  return EXT_KIND_MAP[ext] ?? null;
}

function looksLikeRunFilePath(str: string): boolean {
  if (!str || str.length > 1024 || str.includes('\n')) return false;
  if (/^https?:\/\//i.test(str)) return false;
  // Paths emitted by Yanshi nodes live under `<run_dir>/artifacts/...` or carry media extensions.
  if (str.includes('/artifacts/') || str.startsWith('artifacts/')) {
    return classifyPath(str) !== null;
  }
  if ((str.startsWith('/') || str.includes('/')) && classifyPath(str) !== null) {
    return true;
  }
  return false;
}

function basenameOf(filePath: string): string {
  const parts = filePath.replace(/\\/g, '/').split('/').filter(Boolean);
  return parts[parts.length - 1] ?? filePath;
}

export function buildPreviewApiPath(runId: string, filePath: string): string {
  return `/v1/runs/${encodeURIComponent(runId)}/preview?path=${encodeURIComponent(filePath)}`;
}

/**
 * Walk a node's captured `output` JSON recursively and surface every
 * previewable media/document file (`FlowValue::File` objects, `render_slides`
 * frame arrays, `map` nested TTS WAVs, `render_video` MP4s, etc.).
 */
export function extractNodeMediaPreviews(
  runId: string,
  nodeId: string,
  output: unknown,
): MediaPreviewItem[] {
  if (!runId || output === null || output === undefined) {
    return [];
  }

  const seenPaths = new Set<string>();
  const items: MediaPreviewItem[] = [];

  const pushCandidate = (filePath: string, hintLabel: string, mimeType?: string | null) => {
    const trimmed = filePath.trim();
    if (!trimmed || seenPaths.has(trimmed)) return;
    const kind = classifyPath(trimmed, mimeType);
    if (!kind) return;
    seenPaths.add(trimmed);
    const apiPath = buildPreviewApiPath(runId, trimmed);
    items.push({
      key: `${nodeId}:${trimmed}`,
      nodeId,
      label: hintLabel || basenameOf(trimmed),
      kind,
      apiPath,
      directUrl: resolveApiUrl(apiPath),
      mimeType: mimeType ?? undefined,
    });
  };

  const walk = (val: unknown, breadcrumb: string, depth: number) => {
    if (depth > 8 || val === null || val === undefined) return;
    if (typeof val === 'string') {
      if (looksLikeRunFilePath(val)) {
        pushCandidate(val, `${basenameOf(val)}`);
      }
      return;
    }
    if (Array.isArray(val)) {
      val.forEach((entry, idx) => walk(entry, `${breadcrumb}[${idx}]`, depth + 1));
      return;
    }
    if (typeof val === 'object') {
      const obj = val as Record<string, unknown>;
      if (typeof obj.path === 'string') {
        const mime = typeof obj.mime_type === 'string' ? obj.mime_type : null;
        if (looksLikeRunFilePath(obj.path) || mime) {
          pushCandidate(obj.path, basenameOf(obj.path), mime);
        }
      }
      for (const [k, v] of Object.entries(obj)) {
        walk(v, breadcrumb ? `${breadcrumb}.${k}` : k, depth + 1);
      }
    }
  };

  walk(output, '', 0);

  // Order: video first, then html/images, audio, documents, subtitles.
  const priority: Record<MediaPreviewKind, number> = {
    video: 0,
    image: 1,
    html: 2,
    audio: 3,
    document: 4,
    subtitle: 5,
  };
  items.sort((a, b) => priority[a.kind] - priority[b.kind] || a.key.localeCompare(b.key));
  return items;
}

/**
 * Merge registered run-end `/v1/runs/{id}/artifacts` with live captured node
 * outputs so the Release Mode Deliverable Showcase works both mid-run and
 * on historical runs reloaded later.
 */
export function collectRunDeliverables(
  runId: string,
  artifacts: ArtifactSummary[],
  nodeOutputs: Record<string, unknown>,
): MediaPreviewItem[] {
  const results: MediaPreviewItem[] = [];
  const seenNames = new Set<string>();

  for (const art of artifacts) {
    const kind = classifyPath(art.name, art.mime_type) ?? 'document';
    // Prefer inline `/preview` for HTML/images/video/audio when we also have path,
    // otherwise use `/artifacts/{id}` which streams the exact registered bytes.
    const apiPath = `/v1/runs/${encodeURIComponent(runId)}/artifacts/${encodeURIComponent(art.id)}`;
    seenNames.add(`${art.node_id}:${art.name}`);
    results.push({
      key: `artifact:${art.id}`,
      nodeId: art.node_id,
      label: art.name,
      kind,
      apiPath,
      directUrl: resolveApiUrl(apiPath),
      mimeType: art.mime_type ?? undefined,
      sizeBytes: art.size_bytes,
    });
  }

  for (const [nodeId, output] of Object.entries(nodeOutputs)) {
    for (const item of extractNodeMediaPreviews(runId, nodeId, output)) {
      const dedupKey = `${nodeId}:${item.label}`;
      if (seenNames.has(dedupKey)) {
        // Upgrade HTML/image/video items to inline `/preview` URL so browsers
        // render them inline instead of triggering `Content-Disposition: attachment`.
        const existingIdx = results.findIndex(
          (r) => r.nodeId === nodeId && r.label === item.label,
        );
        if (existingIdx >= 0 && ['video', 'image', 'html', 'audio'].includes(item.kind)) {
          results[existingIdx] = {
            ...results[existingIdx],
            apiPath: item.apiPath,
            directUrl: item.directUrl,
          };
        }
        continue;
      }
      seenNames.add(dedupKey);
      results.push(item);
    }
  }

  const priority: Record<MediaPreviewKind, number> = {
    video: 0,
    html: 1,
    document: 2,
    image: 3,
    audio: 4,
    subtitle: 5,
  };
  results.sort((a, b) => priority[a.kind] - priority[b.kind] || a.label.localeCompare(b.label));
  return results;
}
