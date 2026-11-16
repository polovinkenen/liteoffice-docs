import { useCallback, useEffect, useMemo, useState } from "react";
import { useEditorState } from "@tiptap/react";
import type { Editor } from "@tiptap/react";
import {
  clearSearchMatches,
  setSearchMatches,
} from "../../editor/extensions/searchHighlight";
import { ru } from "../../i18n/ru";
import {
  findMatches,
  replaceAllMatches,
  replaceMatch,
  type SearchMatch,
} from "./search";

const EMPTY_MATCHES: SearchMatch[] = [];

export interface FindReplaceController {
  open: boolean;
  query: string;
  replacement: string;
  caseSensitive: boolean;
  wholeWord: boolean;
  matches: SearchMatch[];
  activeIndex: number;
  status: string | null;
  openPanel: () => void;
  close: () => void;
  setQuery: (value: string) => void;
  setReplacement: (value: string) => void;
  setCaseSensitive: (value: boolean) => void;
  setWholeWord: (value: boolean) => void;
  next: () => void;
  previous: () => void;
  replaceCurrent: () => void;
  replaceAll: () => void;
}

export function useFindReplace(editor: Editor | null): FindReplaceController {
  const [open, setOpen] = useState(false);
  const [query, setQueryState] = useState("");
  const [replacement, setReplacementState] = useState("");
  const [caseSensitive, setCaseSensitiveState] = useState(false);
  const [wholeWord, setWholeWordState] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);
  const [status, setStatus] = useState<string | null>(null);

  const doc =
    useEditorState({
      editor,
      selector: ({ editor: current }) => current?.state.doc ?? null,
      equalityFn: (left, right) => left === right,
    }) ?? null;

  const matches = useMemo(() => {
    if (!open || !doc) {
      return EMPTY_MATCHES;
    }

    return findMatches(doc, query, { caseSensitive, wholeWord });
  }, [caseSensitive, doc, open, query, wholeWord]);

  const currentIndex =
    matches.length === 0
      ? -1
      : activeIndex < 0
        ? -1
        : Math.min(activeIndex, matches.length - 1);

  const setQuery = useCallback((value: string) => {
    setQueryState(value);
    setActiveIndex(-1);
    setStatus(null);
  }, []);

  const setReplacement = useCallback((value: string) => {
    setReplacementState(value);
    setStatus(null);
  }, []);

  const setCaseSensitive = useCallback((value: boolean) => {
    setCaseSensitiveState(value);
    setActiveIndex(-1);
    setStatus(null);
  }, []);

  const setWholeWord = useCallback((value: boolean) => {
    setWholeWordState(value);
    setActiveIndex(-1);
    setStatus(null);
  }, []);

  const close = useCallback(() => {
    setOpen(false);
    setStatus(null);

    if (editor) {
      clearSearchMatches(editor);
      editor.commands.focus();
    }
  }, [editor]);

  const openPanel = useCallback(() => {
    setOpen(true);
    setActiveIndex(-1);
    setStatus(null);

    requestAnimationFrame(() => {
      const input = document.querySelector<HTMLInputElement>(
        ".find-bar__field input",
      );

      input?.focus();
      input?.select();
    });

    if (!editor) {
      return;
    }

    const { from, to } = editor.state.selection;

    if (from !== to) {
      const selected = editor.state.doc.textBetween(from, to, " ", " ").trim();

      if (selected && selected.length <= 200) {
        setQueryState((current) => current || selected);
        setActiveIndex(-1);
      }
    }
  }, [editor]);

  const goTo = useCallback(
    (index: number) => {
      if (!editor || index < 0 || index >= matches.length) {
        return;
      }

      const match = matches[index]!;
      editor
        .chain()
        .setTextSelection({ from: match.from, to: match.to })
        .scrollIntoView()
        .run();
      setActiveIndex(index);
    },
    [editor, matches],
  );

  const next = useCallback(() => {
    if (matches.length === 0) {
      return;
    }

    goTo(currentIndex < 0 ? 0 : (currentIndex + 1) % matches.length);
  }, [currentIndex, goTo, matches.length]);

  const previous = useCallback(() => {
    if (matches.length === 0) {
      return;
    }

    goTo(
      currentIndex < 0
        ? matches.length - 1
        : (currentIndex - 1 + matches.length) % matches.length,
    );
  }, [currentIndex, goTo, matches.length]);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      const key = event.key.toLowerCase();

      if (
        (event.metaKey || event.ctrlKey) &&
        !event.altKey &&
        key === "f"
      ) {
        event.preventDefault();
        openPanel();
        return;
      }

      if (!open) {
        return;
      }

      if (event.key === "Escape") {
        event.preventDefault();
        close();
        return;
      }

      if (
        event.key !== "Enter" ||
        !(event.target instanceof HTMLElement) ||
        !event.target.closest(".find-bar")
      ) {
        return;
      }

      event.preventDefault();

      if (event.shiftKey) {
        previous();
      } else {
        next();
      }
    };

    window.addEventListener("keydown", handler);

    return () => {
      window.removeEventListener("keydown", handler);
    };
  }, [close, next, open, openPanel, previous]);

  useEffect(() => {
    if (!editor) {
      return;
    }

    if (!open || matches.length === 0) {
      clearSearchMatches(editor);
      return;
    }

    setSearchMatches(editor, matches, currentIndex);
  }, [currentIndex, editor, matches, open]);

  const replaceCurrent = useCallback(() => {
    if (!editor || matches.length === 0) {
      return;
    }

    const index = currentIndex < 0 ? 0 : currentIndex;
    const match = matches[index]!;
    editor.view.dispatch(replaceMatch(editor.state, match, replacement));
    setStatus(ru.find.replaced(1));
    setActiveIndex(index);
  }, [currentIndex, editor, matches, replacement]);

  const replaceAll = useCallback(() => {
    if (!editor) {
      return;
    }

    const { transaction, count } = replaceAllMatches(
      editor.state,
      query,
      replacement,
      { caseSensitive, wholeWord },
    );

    if (count === 0) {
      return;
    }

    editor.view.dispatch(transaction);
    setStatus(ru.find.replaced(count));
    setActiveIndex(-1);
  }, [caseSensitive, editor, query, replacement, wholeWord]);

  return {
    open,
    query,
    replacement,
    caseSensitive,
    wholeWord,
    matches,
    activeIndex: currentIndex,
    status,
    openPanel,
    close,
    setQuery,
    setReplacement,
    setCaseSensitive,
    setWholeWord,
    next,
    previous,
    replaceCurrent,
    replaceAll,
  };
}
