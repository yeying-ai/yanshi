import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  applyYamlEditorKeyCommand,
  tokenizeYaml,
  type YamlToken,
} from '../studio/yamlHighlight';

export interface YamlCodeEditorProps {
  value: string;
  onChange: (nextValue: string) => void;
  testId?: string;
  id?: string;
  ariaLabel?: string;
  minHeight?: number;
  rows?: number;
  readOnly?: boolean;
  className?: string;
  onExpandToggle?: () => void;
  isExpanded?: boolean;
}

function tokenClassName(tok: YamlToken): string | undefined {
  switch (tok.kind) {
    case 'plain':
      return undefined;
    case 'schema-key':
      return 'yaml-tok yaml-tok--schema-key';
    case 'key':
      return 'yaml-tok yaml-tok--key';
    case 'node-type':
      return 'yaml-tok yaml-tok--node-type';
    case 'string':
      return 'yaml-tok yaml-tok--string';
    case 'template':
      if (tok.text.startsWith('{%')) return 'yaml-tok yaml-tok--template-ctrl';
      if (tok.text.startsWith('{#')) return 'yaml-tok yaml-tok--comment';
      return 'yaml-tok yaml-tok--template';
    case 'number':
      return 'yaml-tok yaml-tok--number';
    case 'boolean':
      return 'yaml-tok yaml-tok--boolean';
    case 'comment':
      return 'yaml-tok yaml-tok--comment';
    case 'punctuation':
      return 'yaml-tok yaml-tok--punct';
    case 'directive':
      return 'yaml-tok yaml-tok--directive';
    case 'anchor':
      return 'yaml-tok yaml-tok--anchor';
    default:
      return undefined;
  }
}

export function YamlCodeEditor({
  value,
  onChange,
  testId,
  id,
  ariaLabel = 'YAML 编辑器',
  minHeight = 260,
  rows,
  readOnly = false,
  className = '',
  onExpandToggle,
  isExpanded = false,
}: YamlCodeEditorProps): React.ReactElement {
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const backdropRef = useRef<HTMLPreElement | null>(null);
  const gutterRef = useRef<HTMLDivElement | null>(null);

  const [cursorPos, setCursorPos] = useState<{ line: number; col: number }>({
    line: 1,
    col: 1,
  });

  const tokenizedLines = useMemo(() => tokenizeYaml(value), [value]);

  const syncViewportMetrics = useCallback(() => {
    const ta = textareaRef.current;
    const pre = backdropRef.current;
    const gutter = gutterRef.current;
    if (!ta) return;

    if (pre) {
      if (ta.clientWidth > 0) {
        pre.style.width = `${ta.clientWidth}px`;
      }
      if (ta.clientHeight > 0) {
        pre.style.height = `${ta.clientHeight}px`;
      }
      pre.scrollTop = ta.scrollTop;
      pre.scrollLeft = ta.scrollLeft;
    }
    if (gutter) {
      if (ta.clientHeight > 0) {
        gutter.style.height = `${ta.clientHeight}px`;
      }
      gutter.scrollTop = ta.scrollTop;
    }
  }, []);

  const updateCursorInfo = useCallback(() => {
    const ta = textareaRef.current;
    if (!ta) return;
    const idx = ta.selectionStart ?? 0;
    const prefix = ta.value.slice(0, idx);
    const lastNewline = prefix.lastIndexOf('\n');
    const line = lastNewline === -1 ? 1 : prefix.split('\n').length;
    const col = idx - lastNewline;
    setCursorPos((prev) =>
      prev.line === line && prev.col === col ? prev : { line, col },
    );
  }, []);

  useEffect(() => {
    syncViewportMetrics();
  }, [value, syncViewportMetrics]);

  useEffect(() => {
    const ta = textareaRef.current;
    if (!ta || typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(() => {
      syncViewportMetrics();
    });
    observer.observe(ta);
    return () => observer.disconnect();
  }, [syncViewportMetrics]);

  const handleKeyDown = (ev: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (readOnly) return;
    if (ev.nativeEvent.isComposing) return;
    if (ev.key !== 'Tab' && ev.key !== 'Enter') return;
    if (ev.metaKey || ev.ctrlKey || ev.altKey) return;

    const ta = ev.currentTarget;
    const cmd = applyYamlEditorKeyCommand(
      value,
      ta.selectionStart,
      ta.selectionEnd,
      ev.key,
      ev.shiftKey,
    );
    if (!cmd) return;

    ev.preventDefault();
    onChange(cmd.value);
    requestAnimationFrame(() => {
      const el = textareaRef.current;
      if (!el) return;
      el.setSelectionRange(cmd.selectionStart, cmd.selectionEnd);
      syncViewportMetrics();
      updateCursorInfo();
    });
  };

  const computedMinHeight = rows ? Math.max(160, rows * 20 + 24) : minHeight;

  return (
    <div
      className={`yaml-code-editor ${isExpanded ? 'yaml-code-editor--expanded' : ''} ${className}`.trim()}
    >
      <div
        className="yaml-code-editor__body"
        style={isExpanded ? undefined : { minHeight: `${computedMinHeight}px` }}
      >
        <div
          ref={gutterRef}
          className="yaml-code-editor__gutter"
          aria-hidden="true"
        >
          {tokenizedLines.map((_, idx) => {
            const lineNo = idx + 1;
            return (
              <span
                key={lineNo}
                className={
                  cursorPos.line === lineNo ? 'yaml-code-editor__ln active' : 'yaml-code-editor__ln'
                }
              >
                {lineNo}
              </span>
            );
          })}
        </div>

        <div className="yaml-code-editor__stage">
          <pre
            ref={backdropRef}
            className="yaml-code-editor__backdrop"
            aria-hidden="true"
          >
            {tokenizedLines.map((lineTokens, lineIdx) => {
              const isLastLine = lineIdx === tokenizedLines.length - 1;
              return (
                <React.Fragment key={lineIdx}>
                  {lineTokens.map((tok, tokIdx) => {
                    const cls = tokenClassName(tok);
                    return cls ? (
                      <span key={tokIdx} className={cls}>
                        {tok.text}
                      </span>
                    ) : (
                      tok.text
                    );
                  })}
                  {!isLastLine ? '\n' : lineTokens.length === 0 ? ' ' : ''}
                </React.Fragment>
              );
            })}
          </pre>

          <textarea
            ref={textareaRef}
            id={id}
            data-testid={testId}
            aria-label={ariaLabel}
            className="yaml-code-editor__input"
            value={value}
            readOnly={readOnly}
            rows={rows}
            spellCheck={false}
            autoCapitalize="off"
            autoComplete="off"
            autoCorrect="off"
            wrap="off"
            onChange={(ev) => {
              onChange(ev.target.value);
              requestAnimationFrame(() => {
                syncViewportMetrics();
                updateCursorInfo();
              });
            }}
            onScroll={syncViewportMetrics}
            onKeyDown={handleKeyDown}
            onKeyUp={updateCursorInfo}
            onClick={updateCursorInfo}
            onSelect={updateCursorInfo}
          />
        </div>
      </div>

      <div className="yaml-code-editor__footer">
        <span className="yaml-code-editor__meta">
          YAML · 2 Spaces · Ln {cursorPos.line}, Col {cursorPos.col} · 共 {tokenizedLines.length} 行
        </span>
        {onExpandToggle && (
          <button
            type="button"
            className="yaml-code-editor__expand-btn"
            onClick={onExpandToggle}
          >
            {isExpanded ? '⤡ 退出放大' : '⤢ 放大编辑'}
          </button>
        )}
      </div>
    </div>
  );
}
