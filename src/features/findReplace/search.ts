import type { Node as PmNode } from "@tiptap/pm/model";
import type { EditorState, Transaction } from "@tiptap/pm/state";

export interface SearchOptions {
  caseSensitive: boolean;
  wholeWord: boolean;
}

export interface SearchMatch {
  from: number;
  to: number;
  text: string;
}

export const MAX_SEARCH_MATCHES = 10_000;

interface Segment {
  textStart: number;
  textEnd: number;
  docStart: number;
  docEnd: number;
}

interface DocumentText {
  text: string;
  segments: Segment[];
  breaks: number[];
}

function collectDocumentText(doc: PmNode): DocumentText {
  let text = "";
  let pendingBreak = false;
  const segments: Segment[] = [];
  const breaks: number[] = [];

  const flushBreak = () => {
    if (!pendingBreak) {
      return;
    }

    if (text.length > 0) {
      breaks.push(text.length);
      text += "\n";
    }

    pendingBreak = false;
  };

  const walk = (node: PmNode, position: number) => {
    if (node.isText) {
      const value = node.text ?? "";

      if (!value) {
        return;
      }

      flushBreak();

      const textStart = text.length;
      text += value;
      segments.push({
        textStart,
        textEnd: text.length,
        docStart: position,
        docEnd: position + value.length,
      });
      return;
    }

    if (node.isLeaf) {
      pendingBreak = true;
      return;
    }

    let childPosition = position + 1;

    for (let index = 0; index < node.childCount; index += 1) {
      const child = node.child(index);
      walk(child, childPosition);
      childPosition += child.nodeSize;
    }

    pendingBreak = true;
  };

  let position = 0;

  for (let index = 0; index < doc.childCount; index += 1) {
    const child = doc.child(index);
    walk(child, position);
    position += child.nodeSize;
  }

  return { text, segments, breaks };
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

const WORD_CHARACTER = /[\p{L}\p{N}_]/u;

function characterBefore(text: string, index: number): string {
  if (index <= 0) {
    return "";
  }

  const code = text.codePointAt(index - 1) ?? 0;

  if (code >= 0xdc00 && code <= 0xdfff && index >= 2) {
    return text.slice(index - 2, index);
  }

  return text.slice(index - 1, index);
}

function characterAfter(text: string, index: number): string {
  if (index >= text.length) {
    return "";
  }

  const code = text.codePointAt(index) ?? 0;

  return code > 0xffff ? text.slice(index, index + 2) : text.slice(index, index + 1);
}

function isWordCharacter(value: string): boolean {
  return value.length > 0 && WORD_CHARACTER.test(value);
}

function crossesBreak(breaks: number[], start: number, end: number): boolean {
  for (const position of breaks) {
    if (position > start && position < end) {
      return true;
    }
  }

  return false;
}

function findSegment(segments: Segment[], offset: number, atEnd: boolean): Segment | null {
  for (const segment of segments) {
    if (atEnd) {
      if (offset > segment.textStart && offset <= segment.textEnd) {
        return segment;
      }
    } else if (offset >= segment.textStart && offset < segment.textEnd) {
      return segment;
    }
  }

  return null;
}

export function findMatches(
  doc: PmNode,
  query: string,
  options: SearchOptions,
): SearchMatch[] {
  if (!query) {
    return [];
  }

  const { text, segments, breaks } = collectDocumentText(doc);
  const flags = options.caseSensitive ? "gu" : "giu";
  const pattern = new RegExp(escapeRegExp(query), flags);
  const matches: SearchMatch[] = [];

  let result = pattern.exec(text);

  while (result && matches.length < MAX_SEARCH_MATCHES) {
    const start = result.index;
    const end = start + result[0].length;

    if (!crossesBreak(breaks, start, end)) {
      const before = characterBefore(text, start);
      const after = characterAfter(text, end);
      const boundaryOk =
        !options.wholeWord ||
        (!isWordCharacter(before) && !isWordCharacter(after));

      if (boundaryOk) {
        const first = findSegment(segments, start, false);
        const last = findSegment(segments, end, true);

        if (first && last) {
          matches.push({
            from: first.docStart + (start - first.textStart),
            to: last.docStart + (end - last.textStart),
            text: result[0],
          });
        }
      }
    }

    result = pattern.exec(text);
  }

  return matches;
}

export function replaceMatch(
  state: EditorState,
  match: SearchMatch,
  replacement: string,
): Transaction {
  return state.tr.insertText(replacement, match.from, match.to);
}

export function replaceAllMatches(
  state: EditorState,
  query: string,
  replacement: string,
  options: SearchOptions,
): { transaction: Transaction; count: number } {
  const matches = findMatches(state.doc, query, options);
  let transaction = state.tr;

  for (let index = matches.length - 1; index >= 0; index -= 1) {
    const match = matches[index]!;
    transaction = transaction.insertText(replacement, match.from, match.to);
  }

  return { transaction, count: matches.length };
}
