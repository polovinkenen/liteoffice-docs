import { Extension } from "@tiptap/core";
import type { EditorState } from "@tiptap/pm/state";

export interface ParagraphFormatAttributes {
  indent: number;
  lineHeight: string | null;
  spaceBefore: number;
  spaceAfter: number;
}

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    paragraphFormat: {
      setParagraphFormat: (
        attributes: Partial<ParagraphFormatAttributes>,
      ) => ReturnType;
      indent: () => ReturnType;
      outdent: () => ReturnType;
    };
  }
}

function blockPositionAt(
  position: EditorState["selection"]["$from"],
): number | null {
  for (let depth = position.depth; depth > 0; depth -= 1) {
    const node = position.node(depth);

    if (node.type.name === "paragraph" || node.type.name === "heading") {
      return position.before(depth);
    }
  }

  return null;
}

function selectionBlockPositions(state: EditorState): number[] {
  const positions = new Set<number>();
  const { from, to, empty } = state.selection;

  if (empty) {
    const position = blockPositionAt(state.selection.$from);

    if (position !== null) {
      positions.add(position);
    }

    return [...positions];
  }

  state.doc.nodesBetween(from, to, (node, position) => {
    if (node.type.name === "paragraph" || node.type.name === "heading") {
      positions.add(position);
    }
  });

  for (const selectionPosition of [state.selection.$from, state.selection.$to]) {
    const position = blockPositionAt(selectionPosition);

    if (position !== null) {
      positions.add(position);
    }
  }

  return [...positions];
}

function numberAttribute(value: unknown): number {
  const number = Number(value);

  return Number.isFinite(number) ? number : 0;
}

export const ParagraphFormat = Extension.create({
  name: "paragraphFormat",

  addGlobalAttributes() {
    return [
      {
        types: ["paragraph", "heading"],
        attributes: {
          indent: {
            default: 0,
            parseHTML: (element) =>
              numberAttribute(element.getAttribute("data-indent")),
            renderHTML: (attributes) => {
              const indent = numberAttribute(attributes.indent);

              if (!indent) {
                return {};
              }

              return {
                "data-indent": String(indent),
                style: `margin-left: ${indent * 2}em`,
              };
            },
          },
          lineHeight: {
            default: null,
            parseHTML: (element) => element.getAttribute("data-line-height"),
            renderHTML: (attributes) => {
              const lineHeight = attributes.lineHeight;

              if (!lineHeight) {
                return {};
              }

              return {
                "data-line-height": String(lineHeight),
                style: `line-height: ${lineHeight}`,
              };
            },
          },
          spaceBefore: {
            default: 0,
            parseHTML: (element) =>
              numberAttribute(element.getAttribute("data-space-before")),
            renderHTML: (attributes) => {
              const spaceBefore = numberAttribute(attributes.spaceBefore);

              if (!spaceBefore) {
                return {};
              }

              return {
                "data-space-before": String(spaceBefore),
                style: `margin-top: ${spaceBefore}pt`,
              };
            },
          },
          spaceAfter: {
            default: 0,
            parseHTML: (element) =>
              numberAttribute(element.getAttribute("data-space-after")),
            renderHTML: (attributes) => {
              const spaceAfter = numberAttribute(attributes.spaceAfter);

              if (!spaceAfter) {
                return {};
              }

              return {
                "data-space-after": String(spaceAfter),
                style: `margin-bottom: ${spaceAfter}pt`,
              };
            },
          },
        },
      },
    ];
  },

  addCommands() {
    return {
      setParagraphFormat:
        (attributes) =>
        ({ state, tr, dispatch }) => {
          const positions = selectionBlockPositions(state);

          if (positions.length === 0) {
            return false;
          }

          if (dispatch) {
            for (const position of positions) {
              const node = tr.doc.nodeAt(position);

              if (node) {
                tr.setNodeMarkup(position, undefined, {
                  ...node.attrs,
                  ...attributes,
                });
              }
            }
          }

          return true;
        },
      indent:
        () =>
        ({ state, tr, dispatch }) => {
          const positions = selectionBlockPositions(state);

          if (positions.length === 0) {
            return false;
          }

          if (dispatch) {
            for (const position of positions) {
              const node = tr.doc.nodeAt(position);

              if (node) {
                const indent = numberAttribute(node.attrs.indent);
                tr.setNodeMarkup(position, undefined, {
                  ...node.attrs,
                  indent: Math.min(indent + 1, 8),
                });
              }
            }
          }

          return true;
        },
      outdent:
        () =>
        ({ state, tr, dispatch }) => {
          const positions = selectionBlockPositions(state);

          if (positions.length === 0) {
            return false;
          }

          if (dispatch) {
            for (const position of positions) {
              const node = tr.doc.nodeAt(position);

              if (node) {
                const indent = numberAttribute(node.attrs.indent);
                tr.setNodeMarkup(position, undefined, {
                  ...node.attrs,
                  indent: Math.max(indent - 1, 0),
                });
              }
            }
          }

          return true;
        },
    };
  },
});
