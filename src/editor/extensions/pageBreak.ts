import { Node } from "@tiptap/core";

export const PageBreak = Node.create({
  name: "pageBreak",
  group: "block",
  atom: true,
  selectable: true,
  draggable: false,

  parseHTML() {
    return [{ tag: "div[data-page-break]" }];
  },

  renderHTML() {
    return [
      "div",
      {
        "data-page-break": "true",
        class: "page-break",
        contenteditable: "false",
      },
    ];
  },
});
