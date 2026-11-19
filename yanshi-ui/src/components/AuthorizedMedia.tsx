import React, { useEffect, useMemo, useState } from 'react';
import { apiFetch, resolveApiUrl } from '../lib/api';
import { appendInlineQuery, type MediaPreviewItem } from '../studio/mediaExtractor';
import { InteractiveSortableTable, parseDelimitedTable } from './NotebookDeliverableViewer';

export interface AuthorizedMediaProps {
  item: MediaPreviewItem;
  apiToken: string;
  tenantId?: string;
  compact?: boolean;
  headerOnly?: boolean;
}

function formatBytes(size?: number | null): string {
  if (typeof size !== 'number' || Number.isNaN(size) || size < 0) {
    return '';
  }
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
  return `${(size / (1024 * 1024)).toFixed(2)} MB`;
}

export function requiresHeaderMediaFetch(apiToken: string, tenantId?: string): boolean {
  if (Boolean(apiToken.trim())) {
    return true;
  }
  const normalizedTenant = (tenantId ?? '').trim();
  return Boolean(normalizedTenant && normalizedTenant !== 'default');
}

export function AuthorizedMedia({
  item,
  apiToken,
  tenantId,
  compact = false,
  headerOnly = false,
}: AuthorizedMediaProps): React.ReactElement {
  const needsHeaderFetch = requiresHeaderMediaFetch(apiToken, tenantId);
  const inlineApiPath = appendInlineQuery(item.apiPath);
  const directInlineSrc = resolveApiUrl(inlineApiPath);
  const directDownloadSrc = resolveApiUrl(item.apiPath);
  const [blobUrl, setBlobUrl] = useState<string | null>(null);
  const [textPreview, setTextPreview] = useState<string | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [loading, setLoading] = useState<boolean>(false);

  const wantsTextBody =
    item.kind === 'subtitle' || item.kind === 'markdown' || item.kind === 'table';

  useEffect(() => {
    setLoadError(null);
    setTextPreview(null);

    if (headerOnly || (!needsHeaderFetch && !wantsTextBody)) {
      setBlobUrl(null);
      setLoading(false);
      return undefined;
    }

    const controller = new AbortController();
    let createdObjectUrl: string | null = null;
    setLoading(true);

    apiFetch(inlineApiPath, apiToken, { signal: controller.signal }, tenantId)
      .then(async (res) => {
        if (!res.ok) {
          throw new Error(`HTTP ${res.status}`);
        }
        if (wantsTextBody) {
          const txt = await res.text();
          setTextPreview(txt.slice(0, 65_536));
        } else {
          const rawBlob = await res.blob();
          const typedBlob =
            item.mimeType && item.mimeType !== rawBlob.type
              ? new Blob([rawBlob], { type: item.mimeType })
              : rawBlob;
          createdObjectUrl = URL.createObjectURL(typedBlob);
          setBlobUrl(createdObjectUrl);
        }
      })
      .catch((err) => {
        if (err instanceof DOMException && err.name === 'AbortError') {
          return;
        }
        setLoadError(err instanceof Error ? err.message : String(err));
      })
      .finally(() => {
        setLoading(false);
      });

    return () => {
      controller.abort();
      if (createdObjectUrl) {
        URL.revokeObjectURL(createdObjectUrl);
      }
    };
  }, [
    inlineApiPath,
    item.kind,
    item.mimeType,
    apiToken,
    tenantId,
    needsHeaderFetch,
    wantsTextBody,
    headerOnly,
  ]);

  const effectiveUrl = needsHeaderFetch ? blobUrl : directInlineSrc;

  const parsedTable = useMemo(() => {
    if (item.kind !== 'table' || textPreview === null) return null;
    const isTsv =
      item.label.toLowerCase().endsWith('.tsv') ||
      (item.mimeType ?? '').toLowerCase().includes('tab-separated');
    return parseDelimitedTable(textPreview, isTsv ? '\t' : ',');
  }, [item.kind, item.label, item.mimeType, textPreview]);

  const triggerDownload = async () => {
    if (!needsHeaderFetch) {
      const anchor = document.createElement('a');
      anchor.href = directDownloadSrc;
      anchor.download = item.label;
      anchor.target = '_blank';
      anchor.rel = 'noopener noreferrer';
      anchor.click();
      return;
    }
    try {
      const res = await apiFetch(item.apiPath, apiToken, {}, tenantId);
      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = item.label;
      anchor.click();
      window.setTimeout(() => URL.revokeObjectURL(url), 5000);
    } catch (err) {
      setLoadError(err instanceof Error ? err.message : String(err));
    }
  };

  const kindLabelMap: Record<MediaPreviewItem['kind'], string> = {
    video: '视频 MP4',
    image: '矢量 / 图像',
    html: '交互页面 HTML',
    pdf: '排版文档 PDF',
    table: '数据表格 CSV/TSV',
    markdown: '文本 / Markdown',
    audio: '口播音频',
    document: '可编辑套件文档',
    subtitle: '字幕时间轴',
  };

  const sizeLabel = formatBytes(item.sizeBytes);

  return (
    <article className={`studio-media-card ${compact ? 'studio-media-card--compact' : ''}`}>
      <header className="studio-media-card__header">
        <div className="studio-media-card__meta">
          <span className={`studio-media-badge studio-media-badge--${item.kind}`}>
            {kindLabelMap[item.kind]}
          </span>
          <strong className="studio-media-card__title" title={item.label}>
            {item.label}
          </strong>
          <small className="studio-media-card__sub">
            #{item.nodeId}
            {sizeLabel ? ` · ${sizeLabel}` : ''}
          </small>
        </div>
        <button
          type="button"
          className="studio-btn-ghost studio-btn-sm"
          onClick={() => void triggerDownload()}
        >
          下载 / 打开
        </button>
      </header>

      {!headerOnly && (
        <div className="studio-media-card__body">
          {loading && <p className="studio-media-status">正在加载预览…</p>}
          {loadError && <p className="studio-media-error">预览加载失败：{loadError}</p>}

          {!loading && !loadError && item.kind === 'image' && effectiveUrl && (
            <img
              className="studio-media-img"
              src={effectiveUrl}
              alt={item.label}
              loading="lazy"
            />
          )}

          {!loading && !loadError && item.kind === 'video' && effectiveUrl && (
            <video className="studio-media-video" src={effectiveUrl} controls preload="metadata" />
          )}

          {!loading && !loadError && item.kind === 'audio' && effectiveUrl && (
            <audio className="studio-media-audio" src={effectiveUrl} controls preload="metadata" />
          )}

          {!loading && !loadError && item.kind === 'html' && effectiveUrl && (
            <iframe
              className="studio-media-iframe"
              src={effectiveUrl}
              title={item.label}
              sandbox="allow-scripts"
            />
          )}

          {!loading && !loadError && item.kind === 'pdf' && effectiveUrl && (
            <iframe className="studio-media-pdf" src={effectiveUrl} title={item.label} />
          )}

          {!loading && !loadError && item.kind === 'table' && parsedTable && (
            <InteractiveSortableTable
              caption={item.label}
              columns={parsedTable.columns}
              rows={parsedTable.rows}
              testId="media-csv-table"
            />
          )}

          {!loading &&
            !loadError &&
            (item.kind === 'subtitle' || item.kind === 'markdown') &&
            textPreview !== null && <pre className="studio-media-subtitle">{textPreview}</pre>}

          {!loading && !loadError && item.kind === 'document' && (
            <div className="studio-media-doc">
              <p>已生成可编辑套件产物（{item.label}），点击右上角按钮下载或本地打开。</p>
            </div>
          )}
        </div>
      )}
    </article>
  );
}
