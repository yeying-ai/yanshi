import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import {
  NotebookArtifactsEnvelopeSchema,
  NotebookAskResponseSchema,
  NotebookChunkSchema,
  NotebookDetailEnvelopeSchema,
  NotebookSearchEnvelopeSchema,
} from '../schemas';
import { OFFICIAL_GENERATOR_PRESETS } from '../studio/generatorCatalog';
import { inspectYamlLocally } from '../studio/presets';
import {
  buildAskHistoryPayload,
  dedupeDeliverableCitations,
  encodeBytesToBase64,
  NotebookAskResultCard,
  NotebookDeliverableCitationBar,
  NotebookWorkspacePage,
  notebookArtifactToMediaItem,
  notebookIdFromPath,
  resolveScopedSourceIds,
  STUDIO_GENERATORS,
} from './NotebookWorkspacePage';

describe('NotebookWorkspacePage helpers, generator contract & SSR', () => {
  it('extracts notebook IDs from /ui/notebooks/:id paths', () => {
    expect(notebookIdFromPath('/ui/notebooks')).toBeNull();
    expect(notebookIdFromPath('/ui/notebooks/')).toBeNull();
    expect(notebookIdFromPath('/ui/notebooks/550e8400-e29b-41d4-a716-446655440000')).toBe(
      '550e8400-e29b-41d4-a716-446655440000',
    );
    expect(notebookIdFromPath('/ui/notebooks/nb%20demo/')).toBe('nb demo');
  });

  it('encodes UTF-8 and multi-chunk binary buffers to standard Base64', () => {
    const utf8 = new TextEncoder().encode('潮汐与月球引力 Tides');
    const b64 = encodeBytesToBase64(utf8);
    const decoded = Uint8Array.from(atob(b64), (ch) => ch.charCodeAt(0));
    expect(new TextDecoder().decode(decoded)).toBe('潮汐与月球引力 Tides');

    // >64 KB payload crosses multiple 0x8000 chunks without RangeError
    const large = new Uint8Array(70_000);
    for (let i = 0; i < large.length; i += 1) {
      large[i] = i % 251;
    }
    const largeB64 = encodeBytesToBase64(large);
    expect(atob(largeB64).length).toBe(70_000);
  });

  it('maps NotebookArtifact records into AuthorizedMedia preview items', () => {
    const htmlItem = notebookArtifactToMediaItem({
      kind: 'flashcards',
      id: 'art-1',
      run_id: 'run-9',
      node_id: 'page',
      name: 'flashcards.html',
      path_or_url: '/tmp/flashcards.html',
      mime_type: 'text/html',
      created_at: '2026-11-15T00:00:00Z',
      size_bytes: 4096,
    });
    expect(htmlItem.kind).toBe('html');
    expect(htmlItem.apiPath).toBe('/v1/runs/run-9/artifacts/art-1');

    const audioItem = notebookArtifactToMediaItem({
      kind: 'audio_overview',
      id: 'art-2',
      run_id: 'run-10',
      node_id: 'render',
      name: 'overview.mp3',
      path_or_url: '/tmp/overview.mp3',
      mime_type: null,
      created_at: '2026-11-15T00:00:00Z',
    });
    expect(audioItem.kind).toBe('audio');
  });

  it('keeps all 12 Studio generator field keys aligned with compiled-in YAML inputs', () => {
    expect(STUDIO_GENERATORS.map((g) => g.kind).sort()).toEqual([
      'audio_overview',
      'data_table',
      'debate_podcast_show',
      'flashcards',
      'illustrated_slides',
      'infographic',
      'mindmap',
      'quiz',
      'report',
      'slides',
      'tech_slides_video',
      'video_overview',
    ]);

    for (const spec of STUDIO_GENERATORS) {
      const preset = OFFICIAL_GENERATOR_PRESETS.find((p) => p.id === spec.kind);
      expect(preset, `preset for ${spec.kind}`).toBeDefined();
      const inspected = inspectYamlLocally(preset!.yaml);
      const declaredInputs = new Set(inspected.inputs.map((inp) => inp.name));
      expect(declaredInputs.has('chunks')).toBe(true);
      for (const field of spec.fields) {
        expect(
          declaredInputs.has(field.key),
          `generator '${spec.kind}' field '${field.key}' must exist in YAML inputs`,
        ).toBe(true);
      }
    }
  });

  it('validates notebook detail, BM25 search, and artifacts response schemas', () => {
    const detail = NotebookDetailEnvelopeSchema.parse({
      notebook: {
        id: 'nb-1',
        tenant_id: 'default',
        title: 'Ocean Tides',
        created_at: '2026-11-15T00:00:00Z',
        updated_at: '2026-11-15T00:00:00Z',
      },
      sources: [
        {
          id: 'src-1',
          notebook_id: 'nb-1',
          tenant_id: 'default',
          kind: 'text',
          title: 'Lunar Pull',
          uri: null,
          status: 'ready',
          error: null,
          sha256: 'abc12345',
          created_at: '2026-11-15T00:00:01Z',
          updated_at: '2026-11-15T00:00:02Z',
        },
      ],
    });
    expect(detail.sources[0]?.status).toBe('ready');

    const search = NotebookSearchEnvelopeSchema.parse({
      chunks: [
        {
          source_id: 'src-1',
          chunk_id: 'src-1:0',
          content: 'Spring tides occur during new and full moons.',
          score: 3.42,
        },
      ],
    });
    expect(search.chunks[0]?.score).toBeCloseTo(3.42);

    const arts = NotebookArtifactsEnvelopeSchema.parse({
      notebook_id: 'nb-1',
      artifacts: [
        {
          kind: 'mindmap',
          deliverable: {
            kind: 'mindmap',
            title: 'Lunar Tidal Map',
            citations: [
              {
                source_id: 'src-1',
                chunk_id: 'src-1:0',
                quote: 'Spring tides occur during new and full moons.',
              },
            ],
          },
          id: 'art-1',
          run_id: 'run-1',
          node_id: 'map_svg',
          name: 'mindmap.svg',
          path_or_url: '/tmp/mindmap.svg',
          mime_type: 'image/svg+xml',
          created_at: '2026-11-15T00:00:10Z',
          size_bytes: 2048,
        },
      ],
    });
    expect(arts.artifacts).toHaveLength(1);
    expect(arts.artifacts[0]?.deliverable?.title).toBe('Lunar Tidal Map');
    expect(arts.artifacts[0]?.deliverable?.citations[0]?.chunk_id).toBe('src-1:0');

    const chunk = NotebookChunkSchema.parse({
      chunk_id: 'src-1:0',
      source_id: 'src-1',
      notebook_id: 'nb-1',
      ordinal: 0,
      content: 'Perigean king tides peak when the moon is closest.',
      metadata: { start_line: 1, end_line: 4 },
    });
    expect(chunk.ordinal).toBe(0);
    expect(chunk.metadata?.start_line).toBe(1);
  });

  it('renders 3-column NotebookWorkspacePage SSR markup with PDF/Recording tabs, grounded Ask card, chunk lookup, native dialogs, and 12 generators', () => {
    const html = renderToStaticMarkup(
      <NotebookWorkspacePage apiToken="" onTokenChange={() => {}} />,
    );
    expect(html).toContain('data-testid="notebook-workspace-page"');
    expect(html).toContain('<search>');
    expect(html).toContain('role="tablist"');
    expect(html).toContain('id="nb-tab-recording"');
    expect(html).toContain('accept=".pdf,.docx,.xlsx,.pptx,.csv,');
    expect(html).toContain('data-testid="notebook-ask-card"');
    expect(html).toContain('data-testid="notebook-ask-input"');
    expect(html).toContain('data-testid="notebook-ask-submit"');
    expect(html).toContain('data-testid="chunk-lookup-input"');
    expect(html).toContain('data-testid="delete-notebook-dialog"');
    expect(html).toContain('data-testid="chunk-preview-dialog"');
    expect(html).not.toContain('role="dialog"');
    expect(html).not.toContain('aria-modal=');
    for (const spec of STUDIO_GENERATORS) {
      expect(html).toContain(`data-testid="generator-card-${spec.kind}"`);
    }
  });

  it('computes scoped source_ids and renders grounded NotebookAskResultCard with citation chips', () => {
    const sampleSources = [
      {
        id: 'src-a',
        notebook_id: 'nb-1',
        kind: 'file',
        title: 'Basins CSV',
        status: 'ready',
        created_at: '2026-11-19T00:00:00Z',
        updated_at: '2026-11-19T00:00:00Z',
      },
      {
        id: 'src-b',
        notebook_id: 'nb-1',
        kind: 'file',
        title: 'Amphidromic DOCX',
        status: 'ready',
        created_at: '2026-11-19T00:00:01Z',
        updated_at: '2026-11-19T00:00:01Z',
      },
      {
        id: 'src-pending',
        notebook_id: 'nb-1',
        kind: 'url',
        title: 'Pending URL',
        status: 'ingesting',
        created_at: '2026-11-19T00:00:02Z',
        updated_at: '2026-11-19T00:00:02Z',
      },
    ];
    expect(resolveScopedSourceIds(sampleSources, new Set())).toBeUndefined();
    expect(resolveScopedSourceIds(sampleSources, new Set(['src-pending']))).toBeUndefined();
    expect(resolveScopedSourceIds(sampleSources, new Set(['src-b']))).toEqual(['src-a']);
    expect(resolveScopedSourceIds(sampleSources, new Set(['src-a', 'src-b']))).toEqual([]);

    const askParsed = NotebookAskResponseSchema.parse({
      notebook_id: 'nb-1',
      question: 'Bay of Fundy 峰值潮差是多少？',
      answer: 'Bay of Fundy 的峰值潮差达到 16.3 米。',
      model: 'gpt-4o',
      citations: [
        {
          source_id: 'src-a',
          chunk_id: 'src-a:0',
          quote: 'basin: BayOfFundy | peak_m: 16.3',
        },
      ],
      used_chunk_count: 2,
    });

    const askHtml = renderToStaticMarkup(
      <NotebookAskResultCard
        result={askParsed}
        turnIndex={1}
        sourceTitleById={new Map([['src-a', 'Basins CSV']])}
        onInspectChunk={() => {}}
        onUseAsFocus={() => {}}
        onSaveAsSource={() => {}}
      />,
    );
    expect(askHtml).toContain('data-testid="notebook-ask-result"');
    expect(askHtml).toContain('#2 · Q:');
    expect(askHtml).toContain('16.3 米');
    expect(askHtml).toContain('data-testid="citation-chip-src-a:0"');
    expect(askHtml).toContain('data-testid="save-ask-as-source-btn"');
    expect(askHtml).toContain('Basins CSV');

    const manyTurns = Array.from({ length: 10 }, (_, idx) => ({
      question: `  Q${idx + 1} `,
      answer: `  A${idx + 1} `,
    }));
    const historyPayload = buildAskHistoryPayload(manyTurns);
    expect(historyPayload).toHaveLength(8);
    expect(historyPayload[0]).toEqual({ question: 'Q3', answer: 'A3' });
    expect(historyPayload[7]).toEqual({ question: 'Q10', answer: 'A10' });
  });

  it('deduplicates deliverable citations and renders accessible chunk-lookup buttons', () => {
    const deduped = dedupeDeliverableCitations([
      { source_id: 'src-1', chunk_id: 'src-1:0', quote: 'Spring tides peak at new moon.' },
      { source_id: 'src-1', chunk_id: 'src-1:0', quote: 'Spring tides peak at new moon.' },
      { source_id: 'src-2', chunk_id: 'src-2:1', quote: 'Neap tides occur at quarter moon.' },
    ]);
    expect(deduped).toHaveLength(2);

    const titles = new Map([
      ['src-1', 'Lunar Mechanics'],
      ['src-2', 'Coastal Survey'],
    ]);
    const html = renderToStaticMarkup(
      <NotebookDeliverableCitationBar
        deliverable={{
          kind: 'data_table',
          title: 'Tidal Amplitude Table',
          citations: deduped,
        }}
        sourceTitleById={titles}
        onInspectChunk={() => {}}
      />,
    );
    expect(html).toContain('Tidal Amplitude Table');
    expect(html).toContain('data-testid="citation-chip-src-1:0"');
    expect(html).toContain('data-testid="citation-chip-src-2:1"');
    expect(html).toContain('Lunar Mechanics');
    expect(html).toContain('type="button"');
  });
});
