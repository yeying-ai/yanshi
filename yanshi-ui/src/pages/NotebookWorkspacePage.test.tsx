import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { requiresHeaderMediaFetch } from '../components/AuthorizedMedia';
import {
  NotebookDeliverableViewer,
  parseDelimitedTable,
} from '../components/NotebookDeliverableViewer';
import {
  NotebookArtifactsEnvelopeSchema,
  NotebookAskResponseSchema,
  NotebookBatchGenerateResponseSchema,
  NotebookChunkSchema,
  NotebookDetailEnvelopeSchema,
  NotebookExportBundleSchema,
  NotebookImportResponseSchema,
  NotebookMessagesEnvelopeSchema,
  NotebookPlanResponseSchema,
  NotebookSearchEnvelopeSchema,
  NotebookSourceChunksEnvelopeSchema,
} from '../schemas';
import { OFFICIAL_GENERATOR_PRESETS } from '../studio/generatorCatalog';
import { inspectYamlLocally } from '../studio/presets';
import {
  buildAskHistoryPayload,
  buildGeneratorParamsFromDraft,
  dedupeDeliverableCitations,
  encodeBytesToBase64,
  formatAskResultAsNoteText,
  NotebookAskResultCard,
  NotebookDeliverableCitationBar,
  NotebookWorkspacePage,
  notebookArtifactToMediaItem,
  notebookIdFromPath,
  parseCostLimitUsdDraft,
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
    expect(htmlItem.directUrl).toContain('?inline=1');

    const pdfItem = notebookArtifactToMediaItem({
      kind: 'report',
      id: 'art-pdf',
      run_id: 'run-9',
      node_id: 'pdf',
      name: 'report.pdf',
      path_or_url: '/tmp/report.pdf',
      mime_type: 'application/pdf',
      created_at: '2026-11-15T00:00:00Z',
    });
    expect(pdfItem.kind).toBe('pdf');

    const tableItem = notebookArtifactToMediaItem({
      kind: 'data_table',
      id: 'art-csv',
      run_id: 'run-9',
      node_id: 'table_csv',
      name: 'table.csv',
      path_or_url: '/tmp/table.csv',
      mime_type: 'text/csv',
      created_at: '2026-11-15T00:00:00Z',
    });
    expect(tableItem.kind).toBe('table');

    const mdItem = notebookArtifactToMediaItem({
      kind: 'report',
      id: 'art-md',
      run_id: 'run-9',
      node_id: 'md',
      name: 'report.md',
      path_or_url: '/tmp/report.md',
      mime_type: 'text/markdown',
      created_at: '2026-11-15T00:00:00Z',
    });
    expect(mdItem.kind).toBe('markdown');

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
    expect(html).toContain('.png,.jpg,.jpeg,.webp,.gif');
    expect(html).toContain('deepseek-asr');
    expect(html).toContain('data-testid="notebook-ask-card"');
    expect(html).toContain('data-testid="notebook-ask-input"');
    expect(html).toContain('data-testid="notebook-ask-submit"');
    expect(html).toContain('data-testid="notebook-planner-card"');
    expect(html).toContain('data-testid="notebook-plan-input"');
    expect(html).toContain('data-testid="notebook-plan-submit"');
    expect(html).toContain('data-testid="notebook-cost-limit-input"');
    expect(html).toContain('data-testid="import-notebook-file-input"');
    expect(html).toContain('data-testid="import-notebook-submit-btn"');
    expect(html).toContain('data-testid="chunk-lookup-input"');
    expect(html).toContain('data-testid="delete-notebook-dialog"');
    expect(html).toContain('data-testid="chunk-preview-dialog"');
    expect(html).toContain('data-testid="edit-source-dialog"');
    expect(html).not.toContain('role="dialog"');
    expect(html).not.toContain('aria-modal=');
    for (const spec of STUDIO_GENERATORS) {
      expect(html).toContain(`data-testid="generator-card-${spec.kind}"`);
      expect(html).toContain(`data-testid="batch-select-${spec.kind}"`);
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

  it('validates cost_limit_usd drafts and portable export/import Zod schemas', () => {
    expect(parseCostLimitUsdDraft('   ')).toEqual({ ok: true, value: undefined });
    expect(parseCostLimitUsdDraft(' 2.50 ')).toEqual({ ok: true, value: 2.5 });
    expect(parseCostLimitUsdDraft('0').ok).toBe(false);
    expect(parseCostLimitUsdDraft('-1.25').ok).toBe(false);
    expect(parseCostLimitUsdDraft('abc').ok).toBe(false);

    const bundle = NotebookExportBundleSchema.parse({
      version: 1,
      notebook: { id: 'nb-1', title: 'Macrotidal Archive' },
      sources: [{ kind: 'text', title: 'Note', chunks: [] }],
      markdown_brief: '# Macrotidal Archive\n',
    });
    expect(bundle.version).toBe(1);
    expect(bundle.markdown_brief).toContain('Macrotidal Archive');

    const imported = NotebookImportResponseSchema.parse({
      notebook: {
        id: 'nb-2',
        tenant_id: 'default',
        title: 'Macrotidal Archive',
        created_at: '2026-11-19T00:00:00Z',
        updated_at: '2026-11-19T00:00:00Z',
      },
      imported_sources: 1,
      imported_chunks: 3,
    });
    expect(imported.imported_chunks).toBe(3);
  });

  it('validates Studio planner, batch generate, source chunks schemas & draft param builder', () => {
    const reportSpec = STUDIO_GENERATORS.find((g) => g.kind === 'report')!;
    const okDraft = buildGeneratorParamsFromDraft(reportSpec, {
      focus: '  lunar tides  ',
      sections: '3',
      pdf: false,
    });
    expect(okDraft).toEqual({
      ok: true,
      params: { focus: 'lunar tides', sections: 3, pdf: false },
    });

    const badNumberDraft = buildGeneratorParamsFromDraft(reportSpec, {
      focus: 'tides',
      sections: 'not-a-number',
      pdf: true,
    });
    expect(badNumberDraft.ok).toBe(false);

    const srcChunks = NotebookSourceChunksEnvelopeSchema.parse({
      notebook_id: 'nb-1',
      source: {
        id: 'src-1',
        notebook_id: 'nb-1',
        tenant_id: 'default',
        kind: 'file',
        title: 'tides.pdf',
        status: 'ready',
        created_at: '2026-11-19T00:00:00Z',
        updated_at: '2026-11-19T00:00:00Z',
      },
      chunks: [
        {
          chunk_id: 'src-1:0',
          source_id: 'src-1',
          notebook_id: 'nb-1',
          ordinal: 0,
          content: 'Passage zero',
        },
        {
          chunk_id: 'src-1:1',
          source_id: 'src-1',
          notebook_id: 'nb-1',
          ordinal: 1,
          content: 'Passage one',
        },
      ],
    });
    expect(srcChunks.chunks).toHaveLength(2);

    const plan = NotebookPlanResponseSchema.parse({
      notebook_id: 'nb-1',
      goal: 'Study pack',
      model: 'gpt-4o',
      rationale: 'Flashcards plus quiz.',
      items: [
        { kind: 'flashcards', reason: 'Recall', params: { count: 6 } },
        { kind: 'quiz', params: { count: 4 } },
      ],
      used_chunk_count: 2,
    });
    expect(plan.items).toHaveLength(2);

    const batch = NotebookBatchGenerateResponseSchema.parse({
      notebook_id: 'nb-1',
      runs: [
        { run_id: 'r-1', kind: 'flashcards', status: 'queued' },
        { run_id: 'r-2', kind: 'quiz', status: 'queued' },
      ],
      per_run_cost_limit_usd: 1.5,
    });
    expect(batch.runs).toHaveLength(2);
    expect(batch.per_run_cost_limit_usd).toBe(1.5);
  });

  it('requires authenticated fetch for custom tenants and formats persisted Q&A notes & interactive deliverable viewers', () => {
    expect(requiresHeaderMediaFetch('', undefined)).toBe(false);
    expect(requiresHeaderMediaFetch('', 'default')).toBe(false);
    expect(requiresHeaderMediaFetch('', 'tenant-custom')).toBe(true);
    expect(requiresHeaderMediaFetch('tok-123', 'default')).toBe(true);

    const msgEnv = NotebookMessagesEnvelopeSchema.parse({
      notebook_id: 'nb-1',
      messages: [
        {
          id: 'msg-1',
          notebook_id: 'nb-1',
          question: 'What causes spring tides?',
          answer: 'Solar and lunar alignment.',
          model: 'gpt-4o',
          citations: [{ source_id: 'src-1', chunk_id: 'src-1:0', quote: 'Alignment amplifies tides.' }],
          used_chunk_count: 1,
        },
      ],
    });
    expect(msgEnv.messages[0]?.id).toBe('msg-1');

    const noteBody = formatAskResultAsNoteText(
      {
        notebook_id: 'nb-1',
        question: 'What causes spring tides?',
        answer: 'Solar and lunar alignment.',
        model: 'gpt-4o',
        citations: [{ source_id: 'src-1', chunk_id: 'src-1:0', quote: 'Alignment amplifies tides.' }],
        used_chunk_count: 1,
      },
      new Map([['src-1', 'Tidal Notes']]),
    );
    expect(noteBody).toContain('引用来源：');
    expect(noteBody).toContain('[src-1:0] (Tidal Notes)');
    expect(noteBody).toContain('Alignment amplifies tides.');

    const parsedCsv = parseDelimitedTable(
      'Basin,Peak (m),Notes\r\n"Bay of Fundy, CA",16.3,"Resonant ""funnel"" basin"\r\nSevern,15.0,Macrotidal\r\n',
      ',',
    );
    expect(parsedCsv.columns).toEqual(['Basin', 'Peak (m)', 'Notes']);
    expect(parsedCsv.rows).toHaveLength(2);
    expect(parsedCsv.rows[0]?.cells).toEqual([
      'Bay of Fundy, CA',
      '16.3',
      'Resonant "funnel" basin',
    ]);

    const fcHtml = renderToStaticMarkup(
      <NotebookDeliverableViewer
        deliverable={{
          kind: 'flashcards',
          title: 'Tidal Cards',
          content: {
            items: [{ front: 'Peak range in Fundy?', back: '16.3 m', chunk_id: 'src-1:0' }],
          },
          citations: [],
        }}
        onInspectChunk={() => {}}
      />,
    );
    expect(fcHtml).toContain('data-testid="interactive-flashcards"');
    expect(fcHtml).toContain('Peak range in Fundy?');

    const quizHtml = renderToStaticMarkup(
      <NotebookDeliverableViewer
        deliverable={{
          kind: 'quiz',
          title: 'Tidal Quiz',
          content: {
            items: [
              {
                question: 'Which basin reaches 16.3 m?',
                options: ['Bay of Fundy', 'Baltic Sea'],
                answer: 0,
                explanation: 'Half-wave resonance.',
                chunk_id: 'src-1:0',
              },
            ],
          },
          citations: [],
        }}
        onInspectChunk={() => {}}
      />,
    );
    expect(quizHtml).toContain('data-testid="interactive-quiz"');
    expect(quizHtml).toContain('<fieldset');
    expect(quizHtml).toContain('Which basin reaches 16.3 m?');

    const dtHtml = renderToStaticMarkup(
      <NotebookDeliverableViewer
        deliverable={{
          kind: 'data_table',
          title: 'Basin Metrics',
          content: {
            columns: ['Basin', 'Peak (m)'],
            rows: [{ cells: ['Bay of Fundy', '16.3'], chunk_id: 'src-1:0', quote: '16.3 m peak' }],
          },
          citations: [],
        }}
        onInspectChunk={() => {}}
      />,
    );
    expect(dtHtml).toContain('data-testid="interactive-data-table"');
    expect(dtHtml).toContain('scope="col"');
    expect(dtHtml).toContain('Bay of Fundy');

    const audioHtml = renderToStaticMarkup(
      <NotebookDeliverableViewer
        deliverable={{
          kind: 'audio_overview',
          title: 'Tidal Podcast',
          content: {
            loudness_lufs: -16.0,
            script: [
              { speaker: 'host', text: 'Welcome to Tidal Science.', chunk_id: 'src-1:0' },
              { speaker: 'guest', text: 'Bay of Fundy peaks at 16.3 m.', chunk_id: 'src-1:1' },
            ],
          },
          citations: [],
        }}
        onInspectChunk={() => {}}
      />,
    );
    expect(audioHtml).toContain('data-testid="interactive-audio-transcript"');
    expect(audioHtml).toContain('主持人');
    expect(audioHtml).toContain('特邀嘉宾');
    expect(audioHtml).toContain('-16.0 LUFS');

    const videoHtml = renderToStaticMarkup(
      <NotebookDeliverableViewer
        deliverable={{
          kind: 'video_overview',
          title: 'Tidal Explainer',
          content: {
            seconds: 24.5,
            scenes: [
              {
                narration: 'Funnel geometry amplifies incoming Atlantic swells.',
                visual: 'Wide aerial diagram of a funnel-shaped bay',
                motion: true,
                chunk_id: 'src-1:0',
              },
            ],
          },
          citations: [],
        }}
        onInspectChunk={() => {}}
      />,
    );
    expect(videoHtml).toContain('data-testid="interactive-video-storyboard"');
    expect(videoHtml).toContain('镜头 1');
    expect(videoHtml).toContain('动态运镜');
    expect(videoHtml).toContain('24.5s');

    // `debate_podcast_show` (`speaker: moderator | pro | con` + `speaker_label`)
    const debateHtml = renderToStaticMarkup(
      <NotebookDeliverableViewer
        deliverable={{
          kind: 'audio_overview',
          title: 'AI Cinema Debate',
          content: {
            script: [
              { speaker: 'moderator', speaker_label: '林主持', text: '欢迎来到圆桌交锋。' },
              { speaker: 'pro', speaker_label: '正方·陈导', text: '技术平权降低制片门槛。' },
              { speaker: 'con', speaker_label: '反方·周评', text: '批量生成导致审美同质化。' },
            ],
          },
          citations: [],
        }}
        onInspectChunk={() => {}}
      />,
    );
    expect(debateHtml).toContain('林主持');
    expect(debateHtml).toContain('nb-speaker-pill--pro');
    expect(debateHtml).toContain('nb-speaker-pill--con');

    // `tech_slides_video` (`content.slides[]`) & `episodic_drama_studio` (`content.episodes[].scenes[]`)
    const lectureVideoHtml = renderToStaticMarkup(
      <NotebookDeliverableViewer
        deliverable={{
          kind: 'video_overview',
          title: 'Rust Traits Keynote',
          content: {
            seconds: 48.0,
            slides: [
              {
                title: '胖指针内存布局',
                takeaway: 'data_ptr + vtable_ptr 共 16 字节',
                steps: [{ spoken: '请看左侧栈帧中的双指针结构。' }],
              },
            ],
          },
          citations: [],
        }}
        onInspectChunk={() => {}}
      />,
    );
    expect(lectureVideoHtml).toContain('幻灯片 1');
    expect(lectureVideoHtml).toContain('请看左侧栈帧中的双指针结构。');
    expect(lectureVideoHtml).toContain('胖指针内存布局');

    const dramaVideoHtml = renderToStaticMarkup(
      <NotebookDeliverableViewer
        deliverable={{
          kind: 'video_overview',
          title: '记忆当铺',
          content: {
            episodes: [
              {
                episode_number: 1,
                scenes: [
                  {
                    scene_number: 2,
                    location: '雾港地下当铺',
                    speaker_name: '林深',
                    dialogue: '这枚记忆晶片写着三天后的日期。',
                    shot_prompt_en: 'Close-up neon reflection on cracked chip',
                    climax_motion: true,
                  },
                ],
              },
            ],
          },
          citations: [],
        }}
        onInspectChunk={() => {}}
      />,
    );
    expect(dramaVideoHtml).toContain('第 1 集 · 场 2');
    expect(dramaVideoHtml).toContain('林深：这枚记忆晶片写着三天后的日期。');
    expect(dramaVideoHtml).toContain('动态运镜');
  });
});
