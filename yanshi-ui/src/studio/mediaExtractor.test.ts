import { describe, expect, it } from 'vitest';
import {
  appendInlineQuery,
  classifyMediaPath,
  collectRunDeliverables,
  extractNodeMediaPreviews,
} from './mediaExtractor';
import { inspectYamlLocally, TECH_SLIDES_VIDEO_YAML } from './presets';

describe('extractNodeMediaPreviews', () => {
  it('extracts nested slide PNGs, HTML player, Map TTS WAVs, and MP4 outputs', () => {
    const runId = '11111111-2222-3333-4444-555555555555';
    const slidesOutput = {
      slides_html: '/tmp/runs/1111/artifacts/slides/deck.html',
      slides_frames: [
        {
          notes: 'Slide 1 notes',
          frames: [
            '/tmp/runs/1111/artifacts/slides/slide_01_step_01.png',
            '/tmp/runs/1111/artifacts/slides/slide_01_step_02.png',
          ],
        },
      ],
    };

    const items = extractNodeMediaPreviews(runId, 'slides', slidesOutput);
    expect(items.map((i) => i.kind)).toEqual(['image', 'image', 'html']);
    expect(items[0].apiPath).toContain('/v1/runs/11111111-2222-3333-4444-555555555555/preview?path=');

    const mapTtsOutput = {
      results: [
        {
          say: {
            Ok: {
              say_output: {
                type: 'file',
                path: '/tmp/runs/1111/artifacts/say/clip_0.wav',
                mime_type: 'audio/wav',
              },
            },
          },
        },
      ],
    };
    const audioItems = extractNodeMediaPreviews(runId, 'narrate', mapTtsOutput);
    expect(audioItems).toHaveLength(1);
    expect(audioItems[0].kind).toBe('audio');
    expect(audioItems[0].label).toBe('clip_0.wav');
  });

  it('deduplicates registered artifacts with inline preview paths', () => {
    const runId = 'aaaa-bbbb';
    const merged = collectRunDeliverables(
      runId,
      [
        {
          id: 'art-1',
          node_id: 'film',
          name: 'lecture.mp4',
          mime_type: 'video/mp4',
          size_bytes: 4096,
        },
      ],
      {
        film: {
          film_output: '/runs/aaaa-bbbb/artifacts/film/lecture.mp4',
        },
      },
    );
    expect(merged).toHaveLength(1);
    expect(merged[0].kind).toBe('video');
    expect(merged[0].apiPath).toContain('/preview?path=');
  });

  it('classifies pdf, csv/tsv tables, markdown/sql/json, and appends inline=1 to artifact URLs', () => {
    expect(classifyMediaPath('report.pdf', 'application/pdf')).toBe('pdf');
    expect(classifyMediaPath('table.csv', 'text/csv; charset=utf-8')).toBe('table');
    expect(classifyMediaPath('metrics.tsv')).toBe('table');
    expect(classifyMediaPath('dossier.md', 'text/markdown')).toBe('markdown');
    expect(classifyMediaPath('schema.sql')).toBe('markdown');
    expect(classifyMediaPath('deck.pptx')).toBe('document');
    expect(classifyMediaPath('sheet.xlsx')).toBe('document');
    expect(appendInlineQuery('/v1/runs/r1/artifacts/a1')).toBe(
      '/v1/runs/r1/artifacts/a1?inline=1',
    );
    expect(appendInlineQuery('/v1/runs/r1/preview?path=1.svg')).toBe(
      '/v1/runs/r1/preview?path=1.svg',
    );
  });
});

describe('inspectYamlLocally', () => {
  it('computes levels, edges, and sub_nodes for tech_slides_video preset', () => {
    const res = inspectYamlLocally(TECH_SLIDES_VIDEO_YAML);
    expect(res.valid).toBe(true);
    expect(res.nodes.length).toBe(20);
    expect(res.nodes.some((n) => n.id === 'illustrate' && n.type === 'illustration')).toBe(true);
    expect(res.inputs.some((i) => i.name === 'topic')).toBe(true);
    expect(res.inputs.some((i) => i.name === 'presenter_mode')).toBe(true);
    expect(res.edges.some((e) => e.kind === 'condition' && e.to === 'arch_figure')).toBe(true);
    const narrate = res.nodes.find((n) => n.id === 'narrate');
    expect(narrate?.sub_nodes?.[0]?.id).toBe('say');
  });
});
