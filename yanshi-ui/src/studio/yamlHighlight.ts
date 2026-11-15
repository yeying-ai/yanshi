export type YamlTokenKind =
  | 'plain'
  | 'comment'
  | 'directive'
  | 'schema-key'
  | 'key'
  | 'punctuation'
  | 'string'
  | 'template'
  | 'number'
  | 'boolean'
  | 'node-type'
  | 'anchor';

export interface YamlToken {
  kind: YamlTokenKind;
  text: string;
}

export const YANSHI_SCHEMA_KEYS = new Set<string>([
  'name',
  'description',
  'version',
  'inputs',
  'nodes',
  'id',
  'type',
  'dependencies',
  'depends_on',
  'run_if',
  'condition',
  'input_mapping',
  'parameters',
  'retry',
  'timeout_secs',
  'output_format',
  'sub_nodes',
]);

export const YANSHI_NODE_TYPES = new Set<string>([
  'template',
  'llm',
  'agent',
  'skill_agent',
  'map',
  'while',
  'conditional',
  'render_slides',
  'render_video',
  'render_document',
  'tts',
  'image_gen',
  'video_gen',
  'http',
  'file',
  'shell',
  'batch',
  'markmap',
  'arxiv',
  'mcp_tool',
  'rag_retrieve',
  'rag_ingest',
  'code_exec',
]);

const TERA_EXPR_RE = /(\{\{[\s\S]*?\}\}|\{%[\s\S]*?%\}|\{#[\s\S]*?#\})/g;
const BOOLEAN_OR_NULL_RE = /^(?:true|false|null|~|True|False|NULL)$/;
const NUMBER_RE = /^[+-]?(?:0x[0-9a-fA-F]+|\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)$/;
const BLOCK_SCALAR_HEADER_RE = /^([|>](?:[-+]\d*|\d*[-+]?))(\s*(?:#.*)?)$/;

function pushToken(out: YamlToken[], kind: YamlTokenKind, text: string): void {
  if (!text) return;
  const last = out[out.length - 1];
  if (last && last.kind === kind && kind !== 'template') {
    last.text += text;
  } else {
    out.push({ kind, text });
  }
}

function pushWithTemplates(out: YamlToken[], baseKind: YamlTokenKind, text: string): void {
  if (!text) return;
  if (!text.includes('{')) {
    pushToken(out, baseKind, text);
    return;
  }
  let lastIndex = 0;
  TERA_EXPR_RE.lastIndex = 0;
  let match = TERA_EXPR_RE.exec(text);
  while (match !== null) {
    if (match.index > lastIndex) {
      pushToken(out, baseKind, text.slice(lastIndex, match.index));
    }
    pushToken(out, 'template', match[0]);
    lastIndex = match.index + match[0].length;
    match = TERA_EXPR_RE.exec(text);
  }
  if (lastIndex < text.length) {
    pushToken(out, baseKind, text.slice(lastIndex));
  }
}

function classifyUnquotedScalar(raw: string, activeKey: string | null): YamlTokenKind {
  if (BOOLEAN_OR_NULL_RE.test(raw)) {
    return 'boolean';
  }
  if (NUMBER_RE.test(raw)) {
    return 'number';
  }
  if (activeKey === 'type' && YANSHI_NODE_TYPES.has(raw)) {
    return 'node-type';
  }
  return 'string';
}

function scanFlowOrValue(
  out: YamlToken[],
  segment: string,
  initialKey: string | null,
): { enteredBlockScalar: boolean } {
  // Check for `|` / `>` block scalar header before general scanning.
  const trimmedStart = segment.trimStart();
  const leadingWsLen = segment.length - trimmedStart.length;
  if (leadingWsLen > 0) {
    pushToken(out, 'plain', segment.slice(0, leadingWsLen));
  }
  if (!trimmedStart) {
    return { enteredBlockScalar: false };
  }

  const blockMatch = BLOCK_SCALAR_HEADER_RE.exec(trimmedStart);
  if (blockMatch) {
    pushToken(out, 'punctuation', blockMatch[1]);
    const tail = blockMatch[2];
    if (tail) {
      const commentIdx = tail.indexOf('#');
      if (commentIdx >= 0) {
        pushToken(out, 'plain', tail.slice(0, commentIdx));
        pushToken(out, 'comment', tail.slice(commentIdx));
      } else {
        pushToken(out, 'plain', tail);
      }
    }
    return { enteredBlockScalar: true };
  }

  let i = 0;
  const n = trimmedStart.length;
  let activeKey = initialKey;
  let flowDepth = 0;

  while (i < n) {
    const ch = trimmedStart[i];

    // Whitespace
    if (ch === ' ' || ch === '\t') {
      let end = i + 1;
      while (end < n && (trimmedStart[end] === ' ' || trimmedStart[end] === '\t')) {
        end += 1;
      }
      pushToken(out, 'plain', trimmedStart.slice(i, end));
      i = end;
      continue;
    }

    // Inline comment (valid at start of value or preceded by whitespace)
    if (ch === '#' && (i === 0 || trimmedStart[i - 1] === ' ' || trimmedStart[i - 1] === '\t')) {
      pushToken(out, 'comment', trimmedStart.slice(i));
      break;
    }

    // Tera expression starting right at `{{` / `{%` / `{#`
    if (
      ch === '{' &&
      i + 1 < n &&
      (trimmedStart[i + 1] === '{' || trimmedStart[i + 1] === '%' || trimmedStart[i + 1] === '#')
    ) {
      const closer =
        trimmedStart[i + 1] === '{' ? '}}' : trimmedStart[i + 1] === '%' ? '%}' : '#}';
      const closeIdx = trimmedStart.indexOf(closer, i + 2);
      if (closeIdx >= 0) {
        pushToken(out, 'template', trimmedStart.slice(i, closeIdx + 2));
        i = closeIdx + 2;
        continue;
      }
    }

    // Flow collection punctuation
    if (ch === '[' || ch === ']' || ch === '{' || ch === '}' || ch === ',') {
      if (ch === '{' || ch === '[') flowDepth += 1;
      if ((ch === '}' || ch === ']') && flowDepth > 0) flowDepth -= 1;
      if (ch === ',' || ch === '{') activeKey = null;
      pushToken(out, 'punctuation', ch);
      i += 1;
      continue;
    }

    // Double-quoted string
    if (ch === '"') {
      let end = i + 1;
      while (end < n) {
        if (trimmedStart[end] === '\\') {
          end = Math.min(n, end + 2);
          continue;
        }
        if (trimmedStart[end] === '"') {
          end += 1;
          break;
        }
        end += 1;
      }
      const slice = trimmedStart.slice(i, end);
      // Check if this quoted string is a key inside a flow map `{ "k": v }`
      const afterQuote = trimmedStart.slice(end);
      const colonAfter = /^(\s*):(?=\s|[,}\]]|$)/.exec(afterQuote);
      if (flowDepth > 0 && colonAfter) {
        const bareKey = slice.slice(1, slice.endsWith('"') ? -1 : slice.length);
        pushToken(out, YANSHI_SCHEMA_KEYS.has(bareKey) ? 'schema-key' : 'key', slice);
        pushToken(out, 'plain', colonAfter[1]);
        pushToken(out, 'punctuation', ':');
        activeKey = bareKey;
        i = end + colonAfter[1].length + 1;
      } else {
        pushWithTemplates(out, 'string', slice);
        i = end;
      }
      continue;
    }

    // Single-quoted string
    if (ch === "'") {
      let end = i + 1;
      while (end < n) {
        if (trimmedStart[end] === "'" && trimmedStart[end + 1] === "'") {
          end += 2;
          continue;
        }
        if (trimmedStart[end] === "'") {
          end += 1;
          break;
        }
        end += 1;
      }
      pushWithTemplates(out, 'string', trimmedStart.slice(i, end));
      i = end;
      continue;
    }

    // Anchor / alias / tag (`&base`, `*base`, `!env`)
    if (ch === '&' || ch === '*' || ch === '!') {
      let end = i + 1;
      while (
        end < n &&
        trimmedStart[end] !== ' ' &&
        trimmedStart[end] !== '\t' &&
        trimmedStart[end] !== ',' &&
        trimmedStart[end] !== ']' &&
        trimmedStart[end] !== '}'
      ) {
        end += 1;
      }
      pushToken(out, 'anchor', trimmedStart.slice(i, end));
      i = end;
      continue;
    }

    // Inside flow mapping `{ type: string, maxLength: 80 }`, detect unquoted key
    if (flowDepth > 0) {
      const flowKeyMatch = /^([A-Za-z0-9_.-]+)(\s*):(?=\s|[,}\]]|$)/.exec(trimmedStart.slice(i));
      if (flowKeyMatch) {
        const keyName = flowKeyMatch[1];
        pushToken(out, YANSHI_SCHEMA_KEYS.has(keyName) ? 'schema-key' : 'key', keyName);
        pushToken(out, 'plain', flowKeyMatch[2]);
        pushToken(out, 'punctuation', ':');
        activeKey = keyName;
        i += flowKeyMatch[1].length + flowKeyMatch[2].length + 1;
        continue;
      }
    }

    // Unquoted scalar chunk (up to flow delimiter if in flow mode, or inline ` #` comment)
    let end = i;
    while (end < n) {
      const c = trimmedStart[end];
      if (c === '#' && (trimmedStart[end - 1] === ' ' || trimmedStart[end - 1] === '\t')) {
        break;
      }
      if (flowDepth > 0 && (c === ',' || c === ']' || c === '}')) {
        break;
      }
      end += 1;
    }

    // Trim trailing spaces off the unquoted scalar so trailing spaces before `#` stay `'plain'`
    let scalarEnd = end;
    while (
      scalarEnd > i &&
      (trimmedStart[scalarEnd - 1] === ' ' || trimmedStart[scalarEnd - 1] === '\t')
    ) {
      scalarEnd -= 1;
    }

    const rawScalar = trimmedStart.slice(i, scalarEnd);
    const kind = classifyUnquotedScalar(rawScalar, activeKey);
    pushWithTemplates(out, kind, rawScalar);
    if (end > scalarEnd) {
      pushToken(out, 'plain', trimmedStart.slice(scalarEnd, end));
    }
    activeKey = null;
    i = end;
  }

  return { enteredBlockScalar: false };
}

/**
 * Lossless line-by-line YAML + Yanshi Tera expression tokenizer.
 * Guarantees `reconstructFromTokens(tokenizeYaml(src)) === src` for every input string.
 */
export function tokenizeYaml(source: string): YamlToken[][] {
  const rawLines = source.split('\n');
  const result: YamlToken[][] = [];

  let blockParentIndent: number | null = null;
  let blockContentIndent: number | null = null;

  for (const rawLine of rawLines) {
    const hasCr = rawLine.endsWith('\r');
    const line = hasCr ? rawLine.slice(0, -1) : rawLine;
    const tokens: YamlToken[] = [];

    let indent = 0;
    while (indent < line.length && line[indent] === ' ') {
      indent += 1;
    }
    const restAfterIndent = line.slice(indent);

    // 1. Check active multiline `|` / `>` block scalar state
    if (blockParentIndent !== null) {
      if (restAfterIndent.trim().length === 0) {
        pushToken(tokens, 'plain', line);
        if (hasCr) pushToken(tokens, 'plain', '\r');
        result.push(tokens);
        continue;
      }
      if (blockContentIndent === null && indent > blockParentIndent) {
        blockContentIndent = indent;
      }
      if (blockContentIndent !== null && indent >= blockContentIndent) {
        pushToken(tokens, 'plain', line.slice(0, indent));
        pushWithTemplates(tokens, 'string', restAfterIndent);
        if (hasCr) pushToken(tokens, 'plain', '\r');
        result.push(tokens);
        continue;
      }
      // Outdented back to or above parent key -> leave block scalar mode
      blockParentIndent = null;
      blockContentIndent = null;
    }

    // 2. Empty / whitespace-only line
    if (restAfterIndent.length === 0) {
      pushToken(tokens, 'plain', line);
      if (hasCr) pushToken(tokens, 'plain', '\r');
      result.push(tokens);
      continue;
    }

    // 3. Document directives (`---`, `...`, `%YAML`) at column 0
    if (
      indent === 0 &&
      (/^(?:---|\.\.\.)(?=\s|$)/.test(line) || line.startsWith('%YAML') || line.startsWith('%TAG'))
    ) {
      const commentIdx = line.indexOf(' #');
      if (commentIdx >= 0) {
        pushToken(tokens, 'directive', line.slice(0, commentIdx));
        pushToken(tokens, 'plain', ' ');
        pushToken(tokens, 'comment', line.slice(commentIdx + 1));
      } else {
        pushToken(tokens, 'directive', line);
      }
      if (hasCr) pushToken(tokens, 'plain', '\r');
      result.push(tokens);
      continue;
    }

    // 4. Emit leading spaces
    if (indent > 0) {
      pushToken(tokens, 'plain', line.slice(0, indent));
    }

    // 5. Full-line comment
    if (restAfterIndent.startsWith('#')) {
      pushToken(tokens, 'comment', restAfterIndent);
      if (hasCr) pushToken(tokens, 'plain', '\r');
      result.push(tokens);
      continue;
    }

    // 6. Optional sequence bullet prefix (`- ` or `-\n`), supporting nested `- - item`
    let cursor = indent;
    while (
      cursor < line.length &&
      line[cursor] === '-' &&
      (cursor + 1 === line.length || line[cursor + 1] === ' ' || line[cursor + 1] === '\t')
    ) {
      pushToken(tokens, 'punctuation', '-');
      cursor += 1;
      let wsEnd = cursor;
      while (wsEnd < line.length && (line[wsEnd] === ' ' || line[wsEnd] === '\t')) {
        wsEnd += 1;
      }
      if (wsEnd > cursor) {
        pushToken(tokens, 'plain', line.slice(cursor, wsEnd));
        cursor = wsEnd;
      }
    }

    const effectiveKeyIndent = indent;
    const remainder = line.slice(cursor);
    if (remainder.length === 0) {
      if (hasCr) pushToken(tokens, 'plain', '\r');
      result.push(tokens);
      continue;
    }

    if (remainder.startsWith('#')) {
      pushToken(tokens, 'comment', remainder);
      if (hasCr) pushToken(tokens, 'plain', '\r');
      result.push(tokens);
      continue;
    }

    // 7. Mapping key (`foo: ...` or `"foo": ...`)
    const unquotedKeyMatch = /^([A-Za-z0-9_.-]+)(\s*):(?=\s|$)/.exec(remainder);
    const quotedKeyMatch = !unquotedKeyMatch
      ? /^("(?:\\.|[^"\\])*"|'(?:''|[^'])*')(\s*):(?=\s|$)/.exec(remainder)
      : null;

    const keyMatch = unquotedKeyMatch ?? quotedKeyMatch;
    if (keyMatch) {
      const rawKeyToken = keyMatch[1];
      const bareKey =
        rawKeyToken.startsWith('"') || rawKeyToken.startsWith("'")
          ? rawKeyToken.slice(1, -1)
          : rawKeyToken;
      pushToken(tokens, YANSHI_SCHEMA_KEYS.has(bareKey) ? 'schema-key' : 'key', rawKeyToken);
      pushToken(tokens, 'plain', keyMatch[2]);
      pushToken(tokens, 'punctuation', ':');

      const afterColon = remainder.slice(rawKeyToken.length + keyMatch[2].length + 1);
      const { enteredBlockScalar } = scanFlowOrValue(tokens, afterColon, bareKey);
      if (enteredBlockScalar) {
        blockParentIndent = effectiveKeyIndent;
        blockContentIndent = null;
      }
    } else {
      const { enteredBlockScalar } = scanFlowOrValue(tokens, remainder, null);
      if (enteredBlockScalar) {
        blockParentIndent = effectiveKeyIndent;
        blockContentIndent = null;
      }
    }

    if (hasCr) pushToken(tokens, 'plain', '\r');
    result.push(tokens);
  }

  return result;
}

export function reconstructFromTokens(lines: YamlToken[][]): string {
  return lines.map((line) => line.map((tok) => tok.text).join('')).join('\n');
}

export interface EditorSelectionResult {
  value: string;
  selectionStart: number;
  selectionEnd: number;
}

/**
 * Pure 2-space YAML indentation & Enter auto-indent command handler.
 */
export function applyYamlEditorKeyCommand(
  source: string,
  selectionStart: number,
  selectionEnd: number,
  key: 'Tab' | 'Enter',
  shiftKey: boolean,
): EditorSelectionResult | null {
  if (key === 'Tab') {
    const hasMultilineSelection =
      selectionStart !== selectionEnd &&
      source.slice(selectionStart, selectionEnd).includes('\n');

    if (!shiftKey && !hasMultilineSelection) {
      const next =
        source.slice(0, selectionStart) + '  ' + source.slice(selectionEnd);
      return {
        value: next,
        selectionStart: selectionStart + 2,
        selectionEnd: selectionStart + 2,
      };
    }

    // Indent or outdent all covered lines by up to 2 spaces.
    const lineStart = source.lastIndexOf('\n', Math.max(0, selectionStart - 1)) + 1;
    const effectiveEnd =
      selectionEnd > selectionStart && source[selectionEnd - 1] === '\n'
        ? selectionEnd - 1
        : selectionEnd;
    const nextNewline = source.indexOf('\n', effectiveEnd);
    const blockEnd = nextNewline === -1 ? source.length : nextNewline;

    const block = source.slice(lineStart, blockEnd);
    const lines = block.split('\n');
    let firstLineDelta = 0;
    let totalDelta = 0;

    const updatedLines = lines.map((ln, idx) => {
      if (!shiftKey) {
        if (idx === 0) firstLineDelta = 2;
        totalDelta += 2;
        return '  ' + ln;
      }
      const remove = ln.startsWith('  ') ? 2 : ln.startsWith(' ') ? 1 : 0;
      if (idx === 0) firstLineDelta = -remove;
      totalDelta -= remove;
      return ln.slice(remove);
    });

    const next =
      source.slice(0, lineStart) + updatedLines.join('\n') + source.slice(blockEnd);
    const newStart = Math.max(lineStart, selectionStart + firstLineDelta);
    const newEnd =
      selectionStart === selectionEnd
        ? newStart
        : Math.max(newStart, selectionEnd + totalDelta);

    return {
      value: next,
      selectionStart: newStart,
      selectionEnd: newEnd,
    };
  }

  if (key === 'Enter' && !shiftKey) {
    const lineStart = source.lastIndexOf('\n', Math.max(0, selectionStart - 1)) + 1;
    const beforeCursorOnLine = source.slice(lineStart, selectionStart);
    const indentMatch = /^( *)/.exec(beforeCursorOnLine);
    const currentIndent = indentMatch ? indentMatch[1] : '';
    const trimmedBefore = beforeCursorOnLine.replace(/#.*$/, '').trimEnd();
    const extraIndent = trimmedBefore.endsWith(':') ? '  ' : '';
    const insertion = '\n' + currentIndent + extraIndent;

    const next =
      source.slice(0, selectionStart) + insertion + source.slice(selectionEnd);
    const nextPos = selectionStart + insertion.length;
    return {
      value: next,
      selectionStart: nextPos,
      selectionEnd: nextPos,
    };
  }

  return null;
}
