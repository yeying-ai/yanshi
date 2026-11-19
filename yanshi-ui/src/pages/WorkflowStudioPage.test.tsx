import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import {
  ResumePlanSchema,
  RunApprovalsEnvelopeSchema,
  type StreamedEvent,
} from '../schemas';
import {
  extractFinalTextSummary,
  formatSpendUsd,
  reduceWorkflowEvents,
  WorkflowStudioPage,
} from './WorkflowStudioPage';

describe('WorkflowStudioPage Phase 4 telemetry & schemas', () => {
  it('formats USD spend across micro-cent, sub-dollar, and dollar scales', () => {
    expect(formatSpendUsd(0)).toBe('$0.00');
    expect(formatSpendUsd(-1)).toBe('$0.00');
    expect(formatSpendUsd(0.0042)).toBe('$0.0042');
    expect(formatSpendUsd(0.125)).toBe('$0.125');
    expect(formatSpendUsd(3.5)).toBe('$3.50');
  });

  it('reduces node.cost, cost.limit_exceeded, and clears errors across checkpoint resume', () => {
    const events: StreamedEvent[] = [
      {
        run_id: 'run-1',
        seq: 0,
        kind: 'workflow.started',
        payload: { workflow_id: 'run-1' },
        ts: '2026-11-15T00:00:00Z',
      },
      {
        run_id: 'run-1',
        seq: 1,
        kind: 'node.completed',
        payload: { node_id: 'draft', duration_ms: 120 },
        ts: '2026-11-15T00:00:01Z',
      },
      {
        run_id: 'run-1',
        seq: 2,
        kind: 'node.cost',
        payload: { node_id: 'draft', cost_usd: 0.0125, total_cost_usd: 0.0125 },
        ts: '2026-11-15T00:00:01Z',
      },
      {
        run_id: 'run-1',
        seq: 3,
        kind: 'cost.limit_exceeded',
        payload: { node_id: 'polish', spent_usd: 0.055, limit_usd: 0.05 },
        ts: '2026-11-15T00:00:02Z',
      },
      {
        run_id: 'run-1',
        seq: 4,
        kind: 'node.failed',
        payload: { node_id: 'polish', error: 'budget exceeded', duration_ms: 45 },
        ts: '2026-11-15T00:00:02Z',
      },
      {
        run_id: 'run-1',
        seq: 5,
        kind: 'workflow.failed',
        payload: { error: 'cost limit exceeded' },
        ts: '2026-11-15T00:00:02Z',
      },
    ];

    const attempt1 = reduceWorkflowEvents(events);
    expect(attempt1.telemetryByNode.draft?.status).toBe('succeeded');
    expect(attempt1.telemetryByNode.draft?.costUsd).toBeCloseTo(0.0125);
    expect(attempt1.totalCostUsd).toBeCloseTo(0.055);
    expect(attempt1.costLimitExceeded).toEqual({
      nodeId: 'polish',
      spentUsd: 0.055,
      limitUsd: 0.05,
    });
    expect(attempt1.workflowError).toBe('cost limit exceeded');

    const resumedEvents: StreamedEvent[] = [
      ...events,
      {
        run_id: 'run-1',
        seq: 6,
        kind: 'workflow.started',
        payload: { workflow_id: 'run-1' },
        ts: '2026-11-15T00:01:00Z',
      },
      {
        run_id: 'run-1',
        seq: 7,
        kind: 'node.started',
        payload: { node_id: 'polish' },
        ts: '2026-11-15T00:01:01Z',
      },
      {
        run_id: 'run-1',
        seq: 8,
        kind: 'node.completed',
        payload: { node_id: 'polish', duration_ms: 88 },
        ts: '2026-11-15T00:01:02Z',
      },
      {
        run_id: 'run-1',
        seq: 9,
        kind: 'node.cost',
        payload: { node_id: 'polish', cost_usd: 0.02, total_cost_usd: 0.075 },
        ts: '2026-11-15T00:01:02Z',
      },
      {
        run_id: 'run-1',
        seq: 10,
        kind: 'workflow.completed',
        payload: { duration_ms: 210 },
        ts: '2026-11-15T00:01:03Z',
      },
    ];

    const attempt2 = reduceWorkflowEvents(resumedEvents);
    expect(attempt2.workflowError).toBeNull();
    expect(attempt2.costLimitExceeded).toBeNull();
    expect(attempt2.telemetryByNode.draft?.status).toBe('succeeded');
    expect(attempt2.telemetryByNode.polish?.status).toBe('succeeded');
    expect(attempt2.telemetryByNode.polish?.costUsd).toBeCloseTo(0.02);
    expect(attempt2.totalCostUsd).toBeCloseTo(0.075);
    expect(attempt2.workflowDurationMs).toBe(210);
  });

  it('validates ResumePlanSchema and RunApprovalsEnvelopeSchema payloads', () => {
    const plan = ResumePlanSchema.parse({
      schema_version: 2,
      workflow_id: 'run-123',
      last_completed_node: 'draft',
      status: 'Failed',
      created_at: '2026-11-15T00:00:00Z',
      tool_calls: [
        {
          node_id: 'agent_step',
          tool_call_id: 'call-1',
          tool: 'http',
          step_index: 1,
          idempotency: 'idempotent',
          decision: 'skip',
          reason: 'cached output available',
          has_recorded_result: true,
        },
      ],
      summary: {
        total: 1,
        to_replay: 0,
        to_skip: 1,
        requires_manual: 0,
      },
      force_replay: false,
    });
    expect(plan.last_completed_node).toBe('draft');
    expect(plan.summary.to_skip).toBe(1);

    const approvals = RunApprovalsEnvelopeSchema.parse({
      approvals: [
        {
          id: 'req-1',
          session_id: 'run-123',
          step_index: 2,
          tool: 'shell',
          risk: 'high',
          reason: 'execute command',
          requested_at: '2026-11-15T00:00:05Z',
        },
      ],
    });
    expect(approvals.approvals).toHaveLength(1);
    expect(approvals.approvals[0]?.tool).toBe('shell');
  });

  it('renders WorkflowStudioPage SSR markup with budget input', () => {
    const html = renderToStaticMarkup(
      <WorkflowStudioPage initialMode="debug" apiToken="" onTokenChange={() => {}} />,
    );
    expect(html).toContain('studio-cost-limit-input');
  });

  it('unwraps FlowValue::Json envelopes in extractFinalTextSummary', () => {
    const summary = extractFinalTextSummary(['normalize', 'assemble_handout'], {
      normalize: { headline: { type: 'json', value: 'Ownership' } },
      assemble_handout: {
        output: { type: 'json', value: '## 课程讲义汇总\n\n核心不变式' },
      },
    });
    expect(summary).toEqual({
      nodeId: 'assemble_handout',
      text: '## 课程讲义汇总\n\n核心不变式',
    });
  });
});
