import React, { useEffect, useState } from 'react';
import { apiFetch, resolveApiUrl } from '../lib/api';
import type { MediaPreviewItem } from '../studio/mediaExtractor';

export interface AuthorizedMediaProps {
  item: MediaPreviewItem;
  apiToken: string;
  tenantId?: string;
  compact?: boolean;
}

function formatBytes(size?: number | null): string {
  if (typeof size !== 'number' || Number.isNaN(size) || size < 0) {
    return '';
  }
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
  return `${(size / (1024 * 1024)).toFixed(2)} MB`;
}

export function AuthorizedMedia({
  item,
  apiToken,
  tenantId,
  compact = false,
}: AuthorizedMediaProps): React.ReactElement {
  const needsAuthFetch = Boolean(apiToken.trim());
  const directSrc = resolveApiUrl(item.apiPath);
  const [blobUrl, setBlobUrl] = useState<string | null>(null);
  const [textPreview, setTextPreview] = useState<string | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [loading, setLoading] = useState<boolean>(false);

  useEffect(() => {
    setLoadError(null);
    setTextPreview(null);

    const wantsSubtitleText = item.kind === 'subtitle';
    if (!needsAuthFetch && !wantsSubtitleText) {
      setBlobUrl(null);
      setLoading(false);
      return undefined;
    }

    const controller = new AbortController();
    let createdObjectUrl: string | null = null;
    setLoading(true);

    apiFetch(item.apiPath, apiToken, { signal: controller.signal }, tenantId)
      .then(async (res) => {
        if (!res.ok) {
          throw new Error(`HTTP ${res.status}`);
        }
        if (wantsSubtitleText) {
          const txt = await res.text();
          setTextPreview(txt.slice(0, 6000));
        } else {
          const blob = await res.blob();
          createdObjectUrl = URL.createObjectURL(blob);
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
  }, [item.apiPath, item.kind, apiToken, tenantId, needsAuthFetch]);

  const effectiveUrl = needsAuthFetch ? blobUrl : directSrc;

  const triggerDownload = async () => {
    if (!needsAuthFetch) {
      const anchor = document.createElement('a');
      anchor.href = directSrc;
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
    image: '幻灯片图像',
    html: '交互课件 HTML',
    audio: '口播音频',
    document: '文档课件',
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

        {!loading && !loadError && item.kind === 'subtitle' && textPreview !== null && (
          <pre className="studio-media-subtitle">{textPreview}</pre>
        )}

        {!loading && !loadError && item.kind === 'document' && (
          <div className="studio-media-doc">
            <p>已生成可编辑文档产物（{item.label}），点击右上角按钮下载或本地打开。</p>
          </div>
        )}
      </div>
    </article>
  );
}
