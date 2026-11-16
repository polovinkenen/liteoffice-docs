import type { JSONContent } from "@tiptap/core";
import { Schema } from "@tiptap/pm/model";
import { EditorState } from "@tiptap/pm/state";
import { describe, expect, it } from "vitest";
import {
  findMatches,
  replaceAllMatches,
  replaceMatch,
  type SearchOptions,
} from "./search";

const schema = new Schema({
  nodes: {
    doc: { content: "block+" },
    paragraph: { content: "inline*", group: "block" },
    heading: {
      content: "inline*",
      group: "block",
      attrs: { level: { default: 1 } },
    },
    hardBreak: { inline: true, group: "inline", selectable: false },
    image: {
      inline: true,
      group: "inline",
      atom: true,
      attrs: { src: { default: null } },
    },
    text: { group: "inline" },
  },
  marks: {
    bold: {},
    italic: {},
  },
});

const defaultOptions: SearchOptions = {
  caseSensitive: false,
  wholeWord: false,
};

function documentFrom(json: JSONContent) {
  return schema.nodeFromJSON(json);
}

function paragraph(...content: JSONContent[]): JSONContent {
  return { type: "paragraph", content };
}

function text(value: string, marks?: JSONContent["marks"]): JSONContent {
  return { type: "text", text: value, marks };
}

describe("findMatches", () => {
  it("находит все совпадения и возвращает позиции документа", () => {
    const doc = documentFrom({
      type: "doc",
      content: [paragraph(text("кот и кот"))],
    });

    const matches = findMatches(doc, "кот", defaultOptions);

    expect(matches).toHaveLength(2);
    expect(matches[0]).toMatchObject({ from: 1, to: 4, text: "кот" });
    expect(matches[1]).toMatchObject({ from: 7, to: 10, text: "кот" });
  });

  it("учитывает регистр по настройке", () => {
    const doc = documentFrom({
      type: "doc",
      content: [paragraph(text("Привет"))],
    });

    expect(findMatches(doc, "ПРИВЕТ", defaultOptions)).toHaveLength(1);
    expect(
      findMatches(doc, "ПРИВЕТ", {
        ...defaultOptions,
        caseSensitive: true,
      }),
    ).toHaveLength(0);
  });

  it("ищет слово целиком", () => {
    const doc = documentFrom({
      type: "doc",
      content: [paragraph(text("кот котлета кот"))],
    });

    const matches = findMatches(doc, "кот", {
      ...defaultOptions,
      wholeWord: true,
    });

    expect(matches.map((match) => match.text)).toEqual(["кот", "кот"]);
    expect(matches.map((match) => match.from)).toEqual([1, 13]);
  });

  it("находит совпадение через границу марок", () => {
    const doc = documentFrom({
      type: "doc",
      content: [paragraph(text("При", [{ type: "bold" }]), text("вет"))],
    });

    const matches = findMatches(doc, "Привет", defaultOptions);

    expect(matches).toHaveLength(1);
    expect(matches[0]).toMatchObject({ from: 1, to: 7, text: "Привет" });
  });

  it("не ищет через границу абзацев", () => {
    const doc = documentFrom({
      type: "doc",
      content: [paragraph(text("Привет")), paragraph(text("мир"))],
    });

    expect(findMatches(doc, "Приветмир", defaultOptions)).toHaveLength(0);
  });

  it("не ищет через разрыв строки", () => {
    const doc = documentFrom({
      type: "doc",
      content: [
        paragraph(
          text("до"),
          { type: "hardBreak" },
          text("после"),
        ),
      ],
    });

    expect(findMatches(doc, "допосле", defaultOptions)).toHaveLength(0);
    expect(findMatches(doc, "после", defaultOptions)[0]).toMatchObject({
      from: 4,
      to: 9,
    });
  });

  it("возвращает пустой список для пустого запроса", () => {
    const doc = documentFrom({
      type: "doc",
      content: [paragraph(text("текст"))],
    });

    expect(findMatches(doc, "", defaultOptions)).toEqual([]);
  });

  it("игнорирует перекрывающиеся совпадения", () => {
    const doc = documentFrom({
      type: "doc",
      content: [paragraph(text("ааа"))],
    });

    expect(findMatches(doc, "аа", defaultOptions)).toHaveLength(1);
  });

  it("ищет спецсимволы как обычный текст", () => {
    const doc = documentFrom({
      type: "doc",
      content: [paragraph(text("a+b [x] (y)"))],
    });

    expect(findMatches(doc, "a+b", defaultOptions)).toHaveLength(1);
    expect(findMatches(doc, "[x]", defaultOptions)[0]).toMatchObject({
      from: 5,
      to: 8,
    });
  });
});

describe("replace", () => {
  it("заменяет одно совпадение", () => {
    const state = EditorState.create({
      schema,
      doc: documentFrom({
        type: "doc",
        content: [paragraph(text("кот и кот"))],
      }),
    });
    const match = findMatches(state.doc, "кот", defaultOptions)[0]!;

    const transaction = replaceMatch(state, match, "пёс");
    const next = state.apply(transaction);

    expect(next.doc.textContent).toBe("пёс и кот");
  });

  it("заменяет все совпадения в обратном порядке", () => {
    const state = EditorState.create({
      schema,
      doc: documentFrom({
        type: "doc",
        content: [paragraph(text("кот и кот"))],
      }),
    });

    const { transaction, count } = replaceAllMatches(
      state,
      "кот",
      "пёс",
      defaultOptions,
    );
    const next = state.apply(transaction);

    expect(count).toBe(2);
    expect(next.doc.textContent).toBe("пёс и пёс");
  });

  it("не интерпретирует спецсимволы замены как regex", () => {
    const state = EditorState.create({
      schema,
      doc: documentFrom({
        type: "doc",
        content: [paragraph(text("значение"))],
      }),
    });

    const { transaction } = replaceAllMatches(
      state,
      "значение",
      "$& \\1",
      defaultOptions,
    );
    const next = state.apply(transaction);

    expect(next.doc.textContent).toBe("$& \\1");
  });

  it("удаляет совпадения при пустой замене", () => {
    const state = EditorState.create({
      schema,
      doc: documentFrom({
        type: "doc",
        content: [paragraph(text("лишнее"))],
      }),
    });

    const { transaction, count } = replaceAllMatches(
      state,
      "лишнее",
      "",
      defaultOptions,
    );
    const next = state.apply(transaction);

    expect(count).toBe(1);
    expect(next.doc.textContent).toBe("");
  });
});
