import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import type { Editor } from "@tiptap/react";
import type { SearchMatch } from "../../features/findReplace/search";

export interface SearchHighlightState {
  matches: SearchMatch[];
  active: number;
}

export const searchHighlightKey = new PluginKey<SearchHighlightState>(
  "searchHighlight",
);

const EMPTY_STATE: SearchHighlightState = {
  matches: [],
  active: -1,
};

export const SearchHighlight = Extension.create({
  name: "searchHighlight",

  addProseMirrorPlugins() {
    return [
      new Plugin<SearchHighlightState>({
        key: searchHighlightKey,
        state: {
          init: () => EMPTY_STATE,
          apply(transaction, value) {
            const next = transaction.getMeta(searchHighlightKey) as
              | SearchHighlightState
              | undefined;

            if (next) {
              return next;
            }

            return transaction.docChanged ? EMPTY_STATE : value;
          },
        },
        props: {
          decorations(state) {
            const current = searchHighlightKey.getState(state);

            if (!current || current.matches.length === 0) {
              return null;
            }

            return DecorationSet.create(
              state.doc,
              current.matches.map((match, index) =>
                Decoration.inline(match.from, match.to, {
                  class:
                    index === current.active
                      ? "search-match search-match--active"
                      : "search-match",
                }),
              ),
            );
          },
        },
      }),
    ];
  },
});

export function setSearchMatches(
  editor: Editor,
  matches: SearchMatch[],
  active: number,
): void {
  editor.view.dispatch(
    editor.state.tr.setMeta(searchHighlightKey, { matches, active }),
  );
}

export function clearSearchMatches(editor: Editor): void {
  setSearchMatches(editor, [], -1);
}
