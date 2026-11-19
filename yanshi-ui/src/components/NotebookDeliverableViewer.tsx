import React, { useId, useMemo, useState } from 'react';
import type { NotebookDeliverable } from '../schemas';

const TABLE_COLLATOR = new Intl.Collator(undefined, {
  numeric: true,
  sensitivity: 'base',
});

export interface ParsedDelimitedTable {
  columns: string[];
  rows: Array<{
    id: string;
    cells: string[];
    chunkId?: string;
    quote?: string;
  }>;
}

/** Parse RFC-4180 CSV or TSV text into columns and rows without external dependencies. */
export function parseDelimitedTable(raw: string, delimiter: ',' | '\t'): ParsedDelimitedTable {
  const rows: string[][] = [];
  let currentRow: string[] = [];
  let cell = '';
  let inQuotes = false;

  for (let i = 0; i < raw.length; i += 1) {
    const ch = raw[i];
    if (inQuotes) {
      if (ch === '"') {
        if (raw[i + 1] === '"') {
          cell += '"';
          i += 1;
        } else {
          inQuotes = false;
        }
      } else {
        cell += ch;
      }
      continue;
    }
    if (ch === '"') {
      inQuotes = true;
    } else if (ch === delimiter) {
      currentRow.push(cell.trim());
      cell = '';
    } else if (ch === '\r') {
      if (raw[i + 1] === '\n') {
        i += 1;
      }
      currentRow.push(cell.trim());
      cell = '';
      if (currentRow.some((c) => c.length > 0)) {
        rows.push(currentRow);
      }
      currentRow = [];
    } else if (ch === '\n') {
      currentRow.push(cell.trim());
      cell = '';
      if (currentRow.some((c) => c.length > 0)) {
        rows.push(currentRow);
      }
      currentRow = [];
    } else {
      cell += ch;
    }
  }
  if (cell.length > 0 || currentRow.length > 0) {
    currentRow.push(cell.trim());
    if (currentRow.some((c) => c.length > 0)) {
      rows.push(currentRow);
    }
  }

  if (rows.length === 0) {
    return { columns: [], rows: [] };
  }
  const columns = rows[0];
  const bodyRows = rows.slice(1).map((cells, idx) => ({
    id: `row-${idx}`,
    cells: columns.map((_, colIdx) => cells[colIdx] ?? ''),
  }));
  return { columns, rows: bodyRows };
}

export interface InteractiveSortableTableProps {
  caption: string;
  columns: ReadonlyArray<string>;
  rows: ReadonlyArray<{
    id: string;
    cells: ReadonlyArray<string>;
    chunkId?: string;
    quote?: string;
  }>;
  onInspectChunk?: (chunkId: string) => void;
  testId?: string;
}

export function InteractiveSortableTable({
  caption,
  columns,
  rows,
  onInspectChunk,
  testId = 'interactive-data-table',
}: InteractiveSortableTableProps): React.ReactElement | null {
  const uid = useId();
  const captionId = `${uid}-caption`;
  const filterId = `${uid}-filter`;
  const [filterQuery, setFilterQuery] = useState<string>('');
  const [sortCol, setSortCol] = useState<number | null>(null);
  const [sortDir, setSortDir] = useState<'ascending' | 'descending'>('ascending');
  const [statusAnnouncement, setStatusAnnouncement] = useState<string>('');

  const hasCitations = useMemo(
    () => Boolean(onInspectChunk) && rows.some((r) => Boolean(r.chunkId?.trim())),
    [onInspectChunk, rows],
  );

  const visibleRows = useMemo(() => {
    const q = filterQuery.trim().toLowerCase();
    const filtered = q
      ? rows.filter(
          (r) =>
            r.cells.some((c) => c.toLowerCase().includes(q)) ||
            (r.quote ?? '').toLowerCase().includes(q),
        )
      : [...rows];
    if (sortCol === null || sortCol < 0 || sortCol >= columns.length) {
      return filtered;
    }
    const factor = sortDir === 'ascending' ? 1 : -1;
    return [...filtered].sort((a, b) => {
      const av = (a.cells[sortCol] ?? '').trim();
      const bv = (b.cells[sortCol] ?? '').trim();
      const an = Number(av.replace(/[,%$]/g, ''));
      const bn = Number(bv.replace(/[,%$]/g, ''));
      if (av !== '' && bv !== '' && Number.isFinite(an) && Number.isFinite(bn)) {
        return factor * (an - bn);
      }
      return factor * TABLE_COLLATOR.compare(av, bv);
    });
  }, [rows, filterQuery, sortCol, sortDir, columns.length]);

  if (columns.length === 0) {
    return null;
  }

  const handleSortColumn = (colIndex: number) => {
    const nextDir: 'ascending' | 'descending' =
      sortCol === colIndex && sortDir === 'ascending' ? 'descending' : 'ascending';
    setSortCol(colIndex);
    setSortDir(nextDir);
    setStatusAnnouncement(
      `按 ${columns[colIndex]} ${nextDir === 'ascending' ? '升序' : '降序'}排列，共 ${visibleRows.length} 行`,
    );
  };

  return (
    <div className="nb-interactive-table-block" data-testid={testId}>
      <div className="nb-interactive-toolbar">
        <label htmlFor={filterId} className="nb-sr-only">
          筛选表格行
        </label>
        <input
          id={filterId}
          type="search"
          className="nb-table-filter-input"
          data-testid={`${testId}-filter`}
          placeholder={`筛选 ${rows.length} 行数据…`}
          value={filterQuery}
          onChange={(ev) => setFilterQuery(ev.target.value)}
        />
        <span className="nb-interactive-meta">
          显示 {visibleRows.length} / {rows.length} 行
        </span>
      </div>
      <span role="status" className="nb-sr-only">
        {statusAnnouncement}
      </span>
      <div
        tabIndex={0}
        role="region"
        aria-labelledby={captionId}
        className="nb-table-wrapper"
      >
        <table className="nb-sortable-table">
          <caption id={captionId} className="nb-sr-only">
            {caption}
          </caption>
          <thead>
            <tr>
              {columns.map((col, idx) => (
                <th
                  key={`${col}-${idx}`}
                  scope="col"
                  aria-sort={sortCol === idx ? sortDir : undefined}
                >
                  <button
                    type="button"
                    className="nb-th-sort-btn"
                    onClick={() => handleSortColumn(idx)}
                  >
                    <span>{col}</span>
                    <span className="nb-sort-glyph" aria-hidden="true">
                      {sortCol === idx ? (sortDir === 'ascending' ? '▲' : '▼') : '↕'}
                    </span>
                  </button>
                </th>
              ))}
              {hasCitations && <th scope="col">原文溯源</th>}
            </tr>
          </thead>
          <tbody>
            {visibleRows.map((row) => (
              <tr key={row.id}>
                {columns.map((_, colIdx) => (
                  <td key={`${row.id}-${colIdx}`}>{row.cells[colIdx] ?? ''}</td>
                ))}
                {hasCitations && (
                  <td>
                    {row.chunkId?.trim() && onInspectChunk ? (
                      <button
                        type="button"
                        className="nb-citation-chip"
                        title={row.quote || row.chunkId}
                        onClick={() => onInspectChunk(row.chunkId!.trim())}
                      >
                        [{row.chunkId.split(':').pop() ?? row.chunkId}] 溯源
                      </button>
                    ) : null}
                  </td>
                )}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

interface FlashcardItem {
  front: string;
  back: string;
  chunkId?: string;
  quote?: string;
}

interface QuizQuestionItem {
  question: string;
  options: string[];
  answer: number;
  explanation?: string;
  chunkId?: string;
  quote?: string;
}

function extractFlashcards(deliverable: NotebookDeliverable): FlashcardItem[] {
  const rawItems = deliverable.content?.items;
  if (!Array.isArray(rawItems)) return [];
  const out: FlashcardItem[] = [];
  for (const raw of rawItems) {
    if (!raw || typeof raw !== 'object') continue;
    const obj = raw as Record<string, unknown>;
    const front = typeof obj.front === 'string' ? obj.front.trim() : '';
    const back = typeof obj.back === 'string' ? obj.back.trim() : '';
    if (!front || !back) continue;
    out.push({
      front,
      back,
      chunkId: typeof obj.chunk_id === 'string' ? obj.chunk_id.trim() : undefined,
      quote: typeof obj.quote === 'string' ? obj.quote.trim() : undefined,
    });
  }
  return out;
}

function extractQuizQuestions(deliverable: NotebookDeliverable): QuizQuestionItem[] {
  const rawItems = deliverable.content?.items;
  if (!Array.isArray(rawItems)) return [];
  const out: QuizQuestionItem[] = [];
  for (const raw of rawItems) {
    if (!raw || typeof raw !== 'object') continue;
    const obj = raw as Record<string, unknown>;
    const question = typeof obj.question === 'string' ? obj.question.trim() : '';
    const rawOpts = Array.isArray(obj.options) ? obj.options : [];
    const options = rawOpts
      .filter((o): o is string => typeof o === 'string')
      .map((o) => o.trim());
    const answer = typeof obj.answer === 'number' ? Math.trunc(obj.answer) : -1;
    if (!question || options.length < 2 || answer < 0 || answer >= options.length) continue;
    out.push({
      question,
      options,
      answer,
      explanation: typeof obj.explanation === 'string' ? obj.explanation.trim() : undefined,
      chunkId: typeof obj.chunk_id === 'string' ? obj.chunk_id.trim() : undefined,
      quote: typeof obj.quote === 'string' ? obj.quote.trim() : undefined,
    });
  }
  return out;
}

function extractDeliverableTable(deliverable: NotebookDeliverable): ParsedDelimitedTable | null {
  const content = deliverable.content;
  if (!content || typeof content !== 'object') return null;
  const rawCols = Array.isArray(content.columns) ? content.columns : [];
  const columns = rawCols.filter((c): c is string => typeof c === 'string');
  const rawRows = Array.isArray(content.rows) ? content.rows : [];
  if (columns.length === 0 || rawRows.length === 0) return null;

  const rows = rawRows.flatMap((entry, idx) => {
    if (!entry || typeof entry !== 'object') return [];
    const obj = entry as Record<string, unknown>;
    const cells = Array.isArray(obj.cells)
      ? obj.cells.map((c) => (c === null || c === undefined ? '' : String(c)))
      : [];
    if (cells.length === 0) return [];
    return [
      {
        id: `dt-${idx}`,
        cells: columns.map((_, cIdx) => cells[cIdx] ?? ''),
        chunkId: typeof obj.chunk_id === 'string' ? obj.chunk_id : undefined,
        quote: typeof obj.quote === 'string' ? obj.quote : undefined,
      },
    ];
  });
  return rows.length > 0 ? { columns, rows } : null;
}

export interface AudioScriptTurn {
  speaker: string;
  speakerLabel?: string;
  text: string;
  chunkId?: string;
}

export interface VideoStoryboardScene {
  label?: string;
  narration: string;
  visual?: string;
  motion?: boolean;
  chunkId?: string;
}

export function extractAudioScript(deliverable: NotebookDeliverable): {
  turns: AudioScriptTurn[];
  loudnessLufs?: number;
} {
  const content = deliverable.content;
  if (!content || typeof content !== 'object') {
    return { turns: [] };
  }
  const rawScript = Array.isArray(content.script)
    ? content.script
    : Array.isArray(content.turns)
      ? content.turns
      : [];
  const turns: AudioScriptTurn[] = [];
  for (const entry of rawScript) {
    if (!entry || typeof entry !== 'object') continue;
    const obj = entry as Record<string, unknown>;
    const speaker = typeof obj.speaker === 'string' ? obj.speaker.trim() : '';
    const speakerLabel =
      typeof obj.speaker_label === 'string' && obj.speaker_label.trim()
        ? obj.speaker_label.trim()
        : undefined;
    const text = typeof obj.text === 'string' ? obj.text.trim() : '';
    if (!text) continue;
    turns.push({
      speaker: speaker || 'host',
      speakerLabel,
      text,
      chunkId:
        typeof obj.chunk_id === 'string' && obj.chunk_id.trim() ? obj.chunk_id.trim() : undefined,
    });
  }
  const loudnessLufs =
    typeof content.loudness_lufs === 'number' && Number.isFinite(content.loudness_lufs)
      ? content.loudness_lufs
      : undefined;
  return { turns, loudnessLufs };
}

export function extractVideoScenes(deliverable: NotebookDeliverable): {
  scenes: VideoStoryboardScene[];
  seconds?: number;
} {
  const content = deliverable.content;
  if (!content || typeof content !== 'object') {
    return { scenes: [] };
  }
  const rawScenes = Array.isArray(content.scenes) ? content.scenes : [];
  const scenes: VideoStoryboardScene[] = [];
  for (const entry of rawScenes) {
    if (!entry || typeof entry !== 'object') continue;
    const obj = entry as Record<string, unknown>;
    const narration = typeof obj.narration === 'string' ? obj.narration.trim() : '';
    const visual = typeof obj.visual === 'string' ? obj.visual.trim() : '';
    if (!narration && !visual) continue;
    scenes.push({
      narration,
      visual: visual || undefined,
      motion: obj.motion === true,
      chunkId:
        typeof obj.chunk_id === 'string' && obj.chunk_id.trim() ? obj.chunk_id.trim() : undefined,
    });
  }

  // Fallback 1: `tech_slides_video` stores `content.slides[]` with `steps[].spoken` / `notes` / `title`.
  if (scenes.length === 0 && Array.isArray(content.slides)) {
    content.slides.forEach((entry, idx) => {
      if (!entry || typeof entry !== 'object') return;
      const slide = entry as Record<string, unknown>;
      const title = typeof slide.title === 'string' ? slide.title.trim() : '';
      const steps = Array.isArray(slide.steps) ? slide.steps : [];
      const stepLines = steps
        .map((s) => {
          if (!s || typeof s !== 'object') return '';
          const stepObj = s as Record<string, unknown>;
          if (typeof stepObj.spoken === 'string' && stepObj.spoken.trim()) {
            return stepObj.spoken.trim();
          }
          if (typeof stepObj.caption === 'string' && stepObj.caption.trim()) {
            return stepObj.caption.trim();
          }
          return '';
        })
        .filter(Boolean);
      const notes = typeof slide.notes === 'string' ? slide.notes.trim() : '';
      const narration =
        stepLines.join(' ') ||
        notes ||
        (typeof slide.narration === 'string' ? slide.narration.trim() : '');
      const takeaway = typeof slide.takeaway === 'string' ? slide.takeaway.trim() : '';
      const illustrationPrompt =
        typeof slide.illustration_prompt === 'string' ? slide.illustration_prompt.trim() : '';
      const visualParts = [title, illustrationPrompt || takeaway].filter(Boolean);
      const visual = visualParts.join(' · ');
      if (!narration && !visual) return;
      scenes.push({
        label: `幻灯片 ${idx + 1}`,
        narration,
        visual: visual || undefined,
        chunkId:
          typeof slide.chunk_id === 'string' && slide.chunk_id.trim()
            ? slide.chunk_id.trim()
            : undefined,
      });
    });
  }

  // Fallback 2: `episodic_drama_studio` stores `content.episodes[].scenes[]`.
  if (scenes.length === 0 && Array.isArray(content.episodes)) {
    content.episodes.forEach((epEntry, epIdx) => {
      if (!epEntry || typeof epEntry !== 'object') return;
      const ep = epEntry as Record<string, unknown>;
      const epNum =
        typeof ep.episode_number === 'number' && Number.isFinite(ep.episode_number)
          ? ep.episode_number
          : epIdx + 1;
      const epScenes = Array.isArray(ep.scenes) ? ep.scenes : [];
      epScenes.forEach((scEntry, scIdx) => {
        if (!scEntry || typeof scEntry !== 'object') return;
        const sc = scEntry as Record<string, unknown>;
        const scNum =
          typeof sc.scene_number === 'number' && Number.isFinite(sc.scene_number)
            ? sc.scene_number
            : scIdx + 1;
        const speakerName = typeof sc.speaker_name === 'string' ? sc.speaker_name.trim() : '';
        const dialogue = typeof sc.dialogue === 'string' ? sc.dialogue.trim() : '';
        const fallbackNarration = typeof sc.narration === 'string' ? sc.narration.trim() : '';
        const narration =
          speakerName && dialogue
            ? `${speakerName}：${dialogue}`
            : dialogue || fallbackNarration;
        const location = typeof sc.location === 'string' ? sc.location.trim() : '';
        const shotPrompt = typeof sc.shot_prompt_en === 'string' ? sc.shot_prompt_en.trim() : '';
        const visualParts = [location ? `场景：${location}` : '', shotPrompt].filter(Boolean);
        const visual = visualParts.join(' · ');
        if (!narration && !visual) return;
        scenes.push({
          label: `第 ${epNum} 集 · 场 ${scNum}`,
          narration,
          visual: visual || undefined,
          motion: sc.climax_motion === true || sc.motion === true,
          chunkId:
            typeof sc.chunk_id === 'string' && sc.chunk_id.trim() ? sc.chunk_id.trim() : undefined,
        });
      });
    });
  }

  const seconds =
    typeof content.seconds === 'number' && Number.isFinite(content.seconds)
      ? content.seconds
      : undefined;
  return { scenes, seconds };
}

/**
 * Returns `true` when `NotebookDeliverableViewer` already renders the exact
 * interactive table for `data_table` (`content.columns` + `content.rows` with
 * `[chunk_id]` provenance), avoiding a duplicate raw CSV table right underneath
 * while keeping the file's download header bar intact.
 */
export function hasExclusiveDeliverableView(deliverable: NotebookDeliverable): boolean {
  if (deliverable.kind === 'data_table') {
    return extractDeliverableTable(deliverable) !== null;
  }
  return false;
}

export interface NotebookDeliverableViewerProps {
  deliverable: NotebookDeliverable;
  onInspectChunk: (chunkId: string) => void;
}

export function NotebookDeliverableViewer({
  deliverable,
  onInspectChunk,
}: NotebookDeliverableViewerProps): React.ReactElement | null {
  const flashcards = useMemo(
    () => (deliverable.kind === 'flashcards' ? extractFlashcards(deliverable) : []),
    [deliverable],
  );
  const quizQuestions = useMemo(
    () => (deliverable.kind === 'quiz' ? extractQuizQuestions(deliverable) : []),
    [deliverable],
  );
  const dataTable = useMemo(
    () => (deliverable.kind === 'data_table' ? extractDeliverableTable(deliverable) : null),
    [deliverable],
  );
  const audioScript = useMemo(
    () =>
      deliverable.kind === 'audio_overview' ? extractAudioScript(deliverable) : { turns: [] },
    [deliverable],
  );
  const videoStoryboard = useMemo(
    () =>
      deliverable.kind === 'video_overview' ? extractVideoScenes(deliverable) : { scenes: [] },
    [deliverable],
  );

  // Flashcards interactive state
  const [cardOrder, setCardOrder] = useState<number[]>([]);
  const [cardCursor, setCardCursor] = useState<number>(0);
  const [cardFlipped, setCardFlipped] = useState<boolean>(false);

  // Quiz interactive state
  const [selectedAnswers, setSelectedAnswers] = useState<Record<number, number>>({});
  const quizGroupUid = useId();

  if (deliverable.kind === 'flashcards' && flashcards.length > 0) {
    const indices =
      cardOrder.length === flashcards.length
        ? cardOrder
        : flashcards.map((_, idx) => idx);
    const safeCursor = Math.min(cardCursor, indices.length - 1);
    const activeCard = flashcards[indices[safeCursor]];

    const handleShuffle = () => {
      const next = flashcards.map((_, idx) => idx);
      for (let i = next.length - 1; i > 0; i -= 1) {
        const j = Math.floor(Math.random() * (i + 1));
        const tmp = next[i];
        next[i] = next[j];
        next[j] = tmp;
      }
      setCardOrder(next);
      setCardCursor(0);
      setCardFlipped(false);
    };

    return (
      <div className="nb-study-runner" data-testid="interactive-flashcards">
        <div className="nb-interactive-toolbar">
          <span className="nb-interactive-meta" data-testid="flashcard-progress">
            互动闪卡 · 第 {safeCursor + 1} / {indices.length} 张
          </span>
          <div className="nb-picker-actions">
            <button
              type="button"
              className="studio-btn-secondary studio-btn-sm"
              data-testid="flashcard-prev-btn"
              disabled={safeCursor <= 0}
              onClick={() => {
                setCardCursor((c) => Math.max(0, c - 1));
                setCardFlipped(false);
              }}
            >
              ← 上一张
            </button>
            <button
              type="button"
              className="studio-btn-secondary studio-btn-sm"
              data-testid="flashcard-next-btn"
              disabled={safeCursor >= indices.length - 1}
              onClick={() => {
                setCardCursor((c) => Math.min(indices.length - 1, c + 1));
                setCardFlipped(false);
              }}
            >
              下一张 →
            </button>
            {indices.length > 1 && (
              <button
                type="button"
                className="studio-link-btn"
                data-testid="flashcard-shuffle-btn"
                onClick={handleShuffle}
              >
                随机洗牌
              </button>
            )}
          </div>
        </div>
        <button
          type="button"
          className={`nb-flashcard-surface ${cardFlipped ? 'nb-flashcard-surface--flipped' : ''}`}
          data-testid="flashcard-flip-btn"
          aria-pressed={cardFlipped}
          onClick={() => setCardFlipped((f) => !f)}
        >
          <span className="nb-flashcard-side-tag">
            {cardFlipped ? '背面 · 答案解析（点击翻回）' : '正面 · 核心问题（点击翻转）'}
          </span>
          <span className="nb-flashcard-text">
            {cardFlipped ? activeCard.back : activeCard.front}
          </span>
        </button>
        {activeCard.chunkId && (
          <div className="nb-study-citation-row">
            <button
              type="button"
              className="nb-citation-chip"
              onClick={() => onInspectChunk(activeCard.chunkId!)}
            >
              查看原文分块 [{activeCard.chunkId.split(':').pop() ?? activeCard.chunkId}]
            </button>
          </div>
        )}
      </div>
    );
  }

  if (deliverable.kind === 'quiz' && quizQuestions.length > 0) {
    const answeredCount = Object.keys(selectedAnswers).length;
    const correctCount = Object.entries(selectedAnswers).reduce((acc, [qIdx, choice]) => {
      const q = quizQuestions[Number(qIdx)];
      return q && q.answer === choice ? acc + 1 : acc;
    }, 0);

    return (
      <div className="nb-study-runner" data-testid="interactive-quiz">
        <div className="nb-interactive-toolbar">
          <span className="nb-interactive-meta" data-testid="quiz-score-banner">
            自测得分：{correctCount} / {quizQuestions.length}（已答 {answeredCount} /{' '}
            {quizQuestions.length}）
          </span>
          {answeredCount > 0 && (
            <button
              type="button"
              className="studio-btn-secondary studio-btn-sm"
              data-testid="quiz-reset-btn"
              onClick={() => setSelectedAnswers({})}
            >
              重置测验
            </button>
          )}
        </div>
        <div className="nb-quiz-list">
          {quizQuestions.map((q, qIdx) => {
            const chosen = selectedAnswers[qIdx];
            const isAnswered = typeof chosen === 'number';
            return (
              <fieldset key={`q-${qIdx}`} className="nb-quiz-item">
                <legend className="nb-quiz-question">
                  {qIdx + 1}. {q.question}
                </legend>
                <div className="nb-quiz-options">
                  {q.options.map((opt, optIdx) => {
                    const isSelected = chosen === optIdx;
                    const isRight = optIdx === q.answer;
                    let stateClass = '';
                    if (isAnswered) {
                      if (isRight) stateClass = 'nb-quiz-opt--right';
                      else if (isSelected) stateClass = 'nb-quiz-opt--wrong';
                    }
                    const inputId = `${quizGroupUid}-q${qIdx}-o${optIdx}`;
                    return (
                      <label
                        key={inputId}
                        htmlFor={inputId}
                        className={`nb-quiz-opt ${stateClass}`}
                      >
                        <input
                          id={inputId}
                          type="radio"
                          name={`${quizGroupUid}-q-${qIdx}`}
                          checked={isSelected}
                          onChange={() =>
                            setSelectedAnswers((prev) => ({
                              ...prev,
                              [qIdx]: optIdx,
                            }))
                          }
                        />
                        <span>{opt}</span>
                      </label>
                    );
                  })}
                </div>
                {isAnswered && (q.explanation || q.chunkId) && (
                  <div className="nb-quiz-feedback">
                    {q.explanation && <p>{q.explanation}</p>}
                    {q.chunkId && (
                      <button
                        type="button"
                        className="nb-citation-chip"
                        onClick={() => onInspectChunk(q.chunkId!)}
                      >
                        溯源分块 [{q.chunkId.split(':').pop() ?? q.chunkId}]
                      </button>
                    )}
                  </div>
                )}
              </fieldset>
            );
          })}
        </div>
      </div>
    );
  }

  if (deliverable.kind === 'data_table' && dataTable) {
    return (
      <InteractiveSortableTable
        caption={deliverable.title || '结构化提取数据表'}
        columns={dataTable.columns}
        rows={dataTable.rows}
        onInspectChunk={onInspectChunk}
      />
    );
  }

  if (deliverable.kind === 'audio_overview' && audioScript.turns.length > 0) {
    return (
      <div className="nb-study-runner" data-testid="interactive-audio-transcript">
        <div className="nb-interactive-toolbar">
          <span className="nb-interactive-meta">
            播客逐字稿 · 共 {audioScript.turns.length} 轮对谈
            {typeof audioScript.loudnessLufs === 'number'
              ? ` · ${audioScript.loudnessLufs.toFixed(1)} LUFS`
              : ''}
          </span>
        </div>
        <ol className="nb-transcript-list" role="list">
          {audioScript.turns.map((turn, idx) => {
            const roleKey = turn.speaker.toLowerCase();
            const pillModifier =
              roleKey === 'pro'
                ? 'nb-speaker-pill--pro'
                : roleKey === 'con'
                  ? 'nb-speaker-pill--con'
                  : roleKey === 'guest'
                    ? 'nb-speaker-pill--guest'
                    : 'nb-speaker-pill--host';
            const fallbackLabel =
              roleKey === 'host'
                ? '主持人'
                : roleKey === 'moderator'
                  ? '圆桌主持'
                  : roleKey === 'pro'
                    ? '正方辩手'
                    : roleKey === 'con'
                      ? '反方辩手'
                      : roleKey === 'guest'
                        ? '特邀嘉宾'
                        : turn.speaker;
            const speakerLabel = turn.speakerLabel || fallbackLabel;
            return (
              <li key={`turn-${idx}`} className="nb-transcript-turn">
                <div className="nb-transcript-turn__head">
                  <span className={`nb-speaker-pill ${pillModifier}`}>{speakerLabel}</span>
                  {turn.chunkId && (
                    <button
                      type="button"
                      className="nb-citation-chip"
                      aria-label={`第 ${idx + 1} 轮对谈溯源分块 ${turn.chunkId}`}
                      onClick={() => onInspectChunk(turn.chunkId!)}
                    >
                      [{turn.chunkId.split(':').pop() ?? turn.chunkId}] 溯源
                    </button>
                  )}
                </div>
                <p className="nb-transcript-turn__text">{turn.text}</p>
              </li>
            );
          })}
        </ol>
      </div>
    );
  }

  if (deliverable.kind === 'video_overview' && videoStoryboard.scenes.length > 0) {
    return (
      <div className="nb-study-runner" data-testid="interactive-video-storyboard">
        <div className="nb-interactive-toolbar">
          <span className="nb-interactive-meta">
            视频分镜脚本 · 共 {videoStoryboard.scenes.length} 个镜头
            {typeof videoStoryboard.seconds === 'number'
              ? ` · ${videoStoryboard.seconds.toFixed(1)}s`
              : ''}
          </span>
        </div>
        <ol className="nb-storyboard-list" role="list">
          {videoStoryboard.scenes.map((scene, idx) => (
            <li key={`scene-${idx}`} className="nb-storyboard-card">
              <div className="nb-transcript-turn__head">
                <div className="nb-storyboard-badges">
                  <span className="nb-speaker-pill nb-speaker-pill--host">
                    {scene.label ?? `镜头 ${idx + 1}`}
                  </span>
                  {scene.motion && (
                    <span className="nb-speaker-pill nb-speaker-pill--motion">动态运镜</span>
                  )}
                </div>
                {scene.chunkId && (
                  <button
                    type="button"
                    className="nb-citation-chip"
                    aria-label={`镜头 ${idx + 1} 溯源分块 ${scene.chunkId}`}
                    onClick={() => onInspectChunk(scene.chunkId!)}
                  >
                    [{scene.chunkId.split(':').pop() ?? scene.chunkId}] 溯源
                  </button>
                )}
              </div>
              {scene.narration && <p className="nb-transcript-turn__text">{scene.narration}</p>}
              {scene.visual && (
                <p className="nb-storyboard-visual">
                  <strong>画面提示：</strong>
                  {scene.visual}
                </p>
              )}
            </li>
          ))}
        </ol>
      </div>
    );
  }

  return null;
}
