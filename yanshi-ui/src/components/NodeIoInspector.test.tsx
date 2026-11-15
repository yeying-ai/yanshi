import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import type { InspectedNode } from '../schemas';
import {
  NodeIoInspector,
  parseDebugOverrideObject,
  unwrapCapturedFlowJson,
} from './NodeIoInspector';

const NODE: InspectedNode = {
  id: 'summarize',
  type: 'llm',
  dependencies: ['fetch'],
  input_mapping: { text: '{{ nodes.fetch.outputs.output }}' },
  parameters: { prompt: '{{ text }}' },
  level: 1,
};

describe('NodeIoInspector debug overrides', () => {
  it('unwraps tagged json FlowValue envelopes while preserving file/url envelopes', () => {
    const raw = {
      text: { type: 'json', value: 'Hello World' },
      count: { type: 'json', value: 3 },
      slide: { type: 'file', path: '/tmp/slide.png', mime_type: 'image/png' },
    };
    expect(unwrapCapturedFlowJson(raw)).toEqual({
      text: 'Hello World',
      count: 3,
      slide: { type: 'file', path: '/tmp/slide.png', mime_type: 'image/png' },
    });
  });

  it('validates debug override JSON objects and rejects arrays or syntax errors', () => {
    expect(parseDebugOverrideObject('{"output": "ok"}')).toEqual({
      value: { output: 'ok' },
      error: null,
    });
    expect(parseDebugOverrideObject('[1, 2, 3]').error).toContain('JSON 对象');
    expect(parseDebugOverrideObject('{bad').error).toContain('JSON 解析失败');
  });

  it('renders pause callout and injection toggle when paused before or after node', () => {
    const htmlBefore = renderToStaticMarkup(
      <NodeIoInspector
        runId="run-123"
        apiToken=""
        tenantId="default"
        node={NODE}
        capturedInput={{ text: { type: 'json', value: 'draft' } }}
        capturedOutput={undefined}
        pausedAt={{ node_id: 'summarize', phase: 'before_node', next_nodes: ['summarize'] }}
        hasBreakpoint
        onToggleBreakpoint={() => {}}
        onStepDebug={() => {}}
        onContinueDebug={() => {}}
        debugActionBusy={false}
      />,
    );
    expect(htmlBefore).toContain('已在执行前断点挂起：summarize');
    expect(htmlBefore).toContain('data-testid="toggle-debug-override-btn"');

    const htmlAfter = renderToStaticMarkup(
      <NodeIoInspector
        runId="run-123"
        apiToken=""
        tenantId="default"
        node={NODE}
        capturedInput={{ text: { type: 'json', value: 'draft' } }}
        capturedOutput={{ output: { type: 'json', value: 'done' } }}
        pausedAt={{ node_id: 'summarize', phase: 'after_node', next_nodes: ['render'] }}
        hasBreakpoint={false}
        onToggleBreakpoint={() => {}}
        onStepDebug={() => {}}
        onContinueDebug={() => {}}
        debugActionBusy={false}
      />,
    );
    expect(htmlAfter).toContain('本步执行完毕并暂停：summarize');
    expect(htmlAfter).toContain('data-testid="toggle-debug-override-btn"');
  });
});
