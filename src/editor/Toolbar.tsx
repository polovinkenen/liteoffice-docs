import { useEffect, useRef, useState, type ReactNode } from "react";
import { useEditorState } from "@tiptap/react";
import type { Editor } from "@tiptap/react";
import { ru } from "../i18n/ru";
import {
  BLOCK_STYLES,
  FONT_FAMILIES,
  FONT_SIZES,
  LINE_HEIGHTS,
  getBlockAttributes,
  getBlockStyle,
  getSelectionFontState,
  normalizeLink,
} from "./formatting";
import {
  AlignCenterIcon,
  AlignJustifyIcon,
  AlignLeftIcon,
  AlignRightIcon,
  HighlightIcon,
  IndentIcon,
  LinkIcon,
  OutdentIcon,
  TaskListIcon,
} from "./icons";

interface ToolbarProps {
  editor: Editor | null;
}

interface ToolButtonProps {
  label: string;
  onClick: () => void;
  active?: boolean;
  disabled?: boolean;
  children: ReactNode;
}

function ToolButton({
  label,
  onClick,
  active = false,
  disabled = false,
  children,
}: ToolButtonProps) {
  return (
    <button
      type="button"
      className={`tool-button${active ? " tool-button--active" : ""}`}
      title={label}
      aria-label={label}
      aria-pressed={active}
      disabled={disabled}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

export default function Toolbar({ editor }: ToolbarProps) {
  const [linkOpen, setLinkOpen] = useState(false);
  const [linkValue, setLinkValue] = useState("");
  const [paragraphOpen, setParagraphOpen] = useState(false);
  const [indent, setIndent] = useState("0");
  const [spaceBefore, setSpaceBefore] = useState("0");
  const [spaceAfter, setSpaceAfter] = useState("0");
  const selectionRef = useRef<{ from: number; to: number } | null>(null);

  const captureSelection = () => {
    if (!editor) {
      return;
    }

    const { from, to } = editor.state.selection;
    selectionRef.current = { from, to };
  };

  const applyToSelection = (
    action: (chain: ReturnType<Editor["chain"]>) => void,
  ) => {
    if (!editor) {
      return;
    }

    const chain = editor.chain().focus();

    if (selectionRef.current) {
      chain.setTextSelection(selectionRef.current);
    }

    action(chain);
    chain.run();
    selectionRef.current = null;
  };

  const snapshot = useEditorState({
    editor,
    selector: ({ editor: current }) => {
      if (!current) {
        return null;
      }

      const textStyle = current.getAttributes("textStyle");
      const block = current.isActive("heading")
        ? current.getAttributes("heading")
        : current.getAttributes("paragraph");
      const font = getSelectionFontState(current);

      return {
        canUndo: current.can().chain().undo().run(),
        canRedo: current.can().chain().redo().run(),
        bold: current.isActive("bold"),
        italic: current.isActive("italic"),
        underline: current.isActive("underline"),
        strike: current.isActive("strike"),
        highlight: current.isActive("highlight"),
        link: current.isActive("link"),
        bulletList: current.isActive("bulletList"),
        orderedList: current.isActive("orderedList"),
        taskList: current.isActive("taskList"),
        blockquote: current.isActive("blockquote"),
        codeBlock: current.isActive("codeBlock"),
        blockStyle: getBlockStyle(current),
        fontFamily: font.value,
        fontMixed: font.mixed,
        fontSize:
          typeof textStyle.fontSize === "string" ? textStyle.fontSize : "",
        color:
          typeof textStyle.color === "string"
            ? textStyle.color
            : "#000000",
        alignment:
          typeof block.textAlign === "string" ? block.textAlign : "left",
        lineHeight:
          typeof block.lineHeight === "string" ? block.lineHeight : "1",
        indent: Number(block.indent ?? 0),
      };
    },
  });

  const canUndo = snapshot?.canUndo ?? false;
  const canRedo = snapshot?.canRedo ?? false;
  const blockStyle = snapshot?.blockStyle ?? "paragraph";
  const fontFamily = snapshot?.fontFamily ?? "";
  const fontMixed = snapshot?.fontMixed ?? false;
  const knownFont = FONT_FAMILIES.some((font) => font.value === fontFamily);
  const fontSize = snapshot?.fontSize ?? "";
  const rawColor = snapshot?.color ?? "#000000";
  const color = /^#[0-9a-f]{6}$/i.test(rawColor) ? rawColor : "#000000";
  const alignment = snapshot?.alignment ?? "left";
  const lineHeight = snapshot?.lineHeight ?? "1";
  const currentIndent = snapshot?.indent ?? 0;

  useEffect(() => {
    if (!linkOpen && !paragraphOpen) {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setLinkOpen(false);
        setParagraphOpen(false);
      }
    };

    document.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [linkOpen, paragraphOpen]);

  const openLinkDialog = () => {
    captureSelection();
    setLinkValue(
      typeof editor?.getAttributes("link").href === "string"
        ? editor.getAttributes("link").href
        : "",
    );
    setParagraphOpen(false);
    setLinkOpen((open) => !open);
  };

  const applyLink = () => {
    if (!editor) {
      return;
    }

    const href = normalizeLink(linkValue);

    applyToSelection((chain) => {
      chain.extendMarkRange("link");

      if (href) {
        chain.setLink({ href });
      } else {
        chain.unsetLink();
      }
    });

    setLinkOpen(false);
  };

  const openParagraphDialog = () => {
    captureSelection();
    const attributes = editor ? getBlockAttributes(editor) : {};
    setIndent(String(attributes.indent ?? 0));
    setSpaceBefore(String(attributes.spaceBefore ?? 0));
    setSpaceAfter(String(attributes.spaceAfter ?? 0));
    setLinkOpen(false);
    setParagraphOpen((open) => !open);
  };

  const applyParagraphFormat = () => {
    applyToSelection((chain) => {
      chain.setParagraphFormat({
        indent: clamp(Number(indent) || 0, 0, 8),
        spaceBefore: clamp(Number(spaceBefore) || 0, 0, 72),
        spaceAfter: clamp(Number(spaceAfter) || 0, 0, 72),
      });
    });
    setParagraphOpen(false);
  };

  return (
    <>
      <div className="toolbar" role="toolbar" aria-label="Форматирование">
        <div className="toolbar__row">
          <ToolButton
            label={ru.toolbar.undo}
            disabled={!canUndo}
            onClick={() => editor?.chain().focus().undo().run()}
          >
            ↺
          </ToolButton>
          <ToolButton
            label={ru.toolbar.redo}
            disabled={!canRedo}
            onClick={() => editor?.chain().focus().redo().run()}
          >
            ↻
          </ToolButton>

          <span className="tool-separator" aria-hidden="true" />

          <select
            className="tool-select tool-select--style"
            title={ru.toolbar.blockStyle}
            aria-label={ru.toolbar.blockStyle}
            value={blockStyle}
            onPointerDown={captureSelection}
            onChange={(event) => {
              const value = event.target.value;

              if (!editor) {
                return;
              }

              applyToSelection((chain) => {
                if (value === "paragraph") {
                  chain.setParagraph();
                } else {
                  chain.setHeading({ level: Number(value) as 1 | 2 | 3 });
                }
              });
            }}
          >
            {BLOCK_STYLES.map((style) => (
              <option key={style.value} value={style.value}>
                {style.label}
              </option>
            ))}
          </select>

          <select
            className="tool-select tool-select--font"
            title={ru.toolbar.fontFamily}
            aria-label={ru.toolbar.fontFamily}
            value={fontMixed ? "__mixed__" : fontFamily}
            onPointerDown={captureSelection}
            onChange={(event) => {
              const value = event.target.value;

              if (!editor || value === "__mixed__") {
                return;
              }

              applyToSelection((chain) => {
                if (value) {
                  chain.setFontFamily(value);
                } else {
                  chain.unsetFontFamily();
                }
              });
            }}
          >
            {fontMixed ? (
              <option value="__mixed__" disabled>
                {ru.toolbar.mixedFont}
              </option>
            ) : null}
            {!fontMixed && fontFamily && !knownFont ? (
              <option value={fontFamily}>{fontFamily}</option>
            ) : null}
            {FONT_FAMILIES.map((font) => (
              <option key={font.value || "default"} value={font.value}>
                {font.label}
              </option>
            ))}
          </select>

          <select
            className="tool-select tool-select--size"
            title={ru.toolbar.fontSize}
            aria-label={ru.toolbar.fontSize}
            value={fontSize}
            onPointerDown={captureSelection}
            onChange={(event) => {
              const value = event.target.value;

              if (!editor) {
                return;
              }

              applyToSelection((chain) => {
                if (value) {
                  chain.setFontSize(value);
                } else {
                  chain.unsetFontSize();
                }
              });
            }}
          >
            <option value="">{ru.toolbar.defaultSize}</option>
            {fontSize && !FONT_SIZES.includes(fontSize) ? (
              <option value={fontSize}>{fontSize}</option>
            ) : null}
            {FONT_SIZES.map((size) => (
              <option key={size} value={size}>
                {size.replace("pt", "")}
              </option>
            ))}
          </select>

          <span className="tool-separator" aria-hidden="true" />

          <ToolButton
            label={ru.toolbar.bold}
            active={snapshot?.bold ?? false}
            onClick={() => editor?.chain().focus().toggleBold().run()}
          >
            <strong>Ж</strong>
          </ToolButton>
          <ToolButton
            label={ru.toolbar.italic}
            active={snapshot?.italic ?? false}
            onClick={() => editor?.chain().focus().toggleItalic().run()}
          >
            <em>К</em>
          </ToolButton>
          <ToolButton
            label={ru.toolbar.underline}
            active={snapshot?.underline ?? false}
            onClick={() => editor?.chain().focus().toggleUnderline().run()}
          >
            <u>Ч</u>
          </ToolButton>
          <ToolButton
            label={ru.toolbar.strike}
            active={snapshot?.strike ?? false}
            onClick={() => editor?.chain().focus().toggleStrike().run()}
          >
            <s>З</s>
          </ToolButton>

          <label
            className="tool-color"
            title={ru.toolbar.textColor}
            aria-label={ru.toolbar.textColor}
            onPointerDown={captureSelection}
          >
            <span aria-hidden="true">А</span>
            <input
              type="color"
              value={color}
              onChange={(event) => {
                applyToSelection((chain) => {
                  chain.setColor(event.target.value);
                });
              }}
            />
          </label>
          <ToolButton
            label={ru.toolbar.highlight}
            active={snapshot?.highlight ?? false}
            onClick={() => editor?.chain().focus().toggleHighlight().run()}
          >
            <HighlightIcon />
          </ToolButton>
        </div>

        <div className="toolbar__row">
          {(
            [
              ["left", ru.toolbar.alignLeft, AlignLeftIcon],
              ["center", ru.toolbar.alignCenter, AlignCenterIcon],
              ["right", ru.toolbar.alignRight, AlignRightIcon],
              ["justify", ru.toolbar.alignJustify, AlignJustifyIcon],
            ] as const
          ).map(([value, label, Icon]) => (
            <ToolButton
              key={value}
              label={label}
              active={alignment === value}
              onClick={() =>
                editor?.chain().focus().setTextAlign(value).run()
              }
            >
              <Icon />
            </ToolButton>
          ))}

          <span className="tool-separator" aria-hidden="true" />

          <ToolButton
            label={ru.toolbar.bulletList}
            active={snapshot?.bulletList ?? false}
            onClick={() => editor?.chain().focus().toggleBulletList().run()}
          >
            ••
          </ToolButton>
          <ToolButton
            label={ru.toolbar.orderedList}
            active={snapshot?.orderedList ?? false}
            onClick={() => editor?.chain().focus().toggleOrderedList().run()}
          >
            1.
          </ToolButton>
          <ToolButton
            label={ru.toolbar.taskList}
            active={snapshot?.taskList ?? false}
            onClick={() => editor?.chain().focus().toggleTaskList().run()}
          >
            <TaskListIcon />
          </ToolButton>
          <ToolButton
            label={ru.toolbar.blockquote}
            active={snapshot?.blockquote ?? false}
            onClick={() => editor?.chain().focus().toggleBlockquote().run()}
          >
            ❝
          </ToolButton>
          <ToolButton
            label={ru.toolbar.codeBlock}
            active={snapshot?.codeBlock ?? false}
            onClick={() => editor?.chain().focus().toggleCodeBlock().run()}
          >
            {"</>"}
          </ToolButton>

          <span className="tool-separator" aria-hidden="true" />

          <ToolButton
            label={ru.toolbar.outdent}
            disabled={currentIndent <= 0}
            onClick={() => editor?.chain().focus().outdent().run()}
          >
            <OutdentIcon />
          </ToolButton>
          <ToolButton
            label={ru.toolbar.indent}
            disabled={currentIndent >= 8}
            onClick={() => editor?.chain().focus().indent().run()}
          >
            <IndentIcon />
          </ToolButton>

          <select
            className="tool-select tool-select--spacing"
            title={ru.toolbar.lineSpacing}
            aria-label={ru.toolbar.lineSpacing}
            value={LINE_HEIGHTS.includes(lineHeight) ? lineHeight : "1"}
            onPointerDown={captureSelection}
            onChange={(event) => {
              const value = event.target.value;

              applyToSelection((chain) => {
                chain.setParagraphFormat({
                  lineHeight: value === "1" ? null : value,
                });
              });
            }}
          >
            {LINE_HEIGHTS.map((value) => (
              <option key={value} value={value}>
                {value.replace(".", ",")}
              </option>
            ))}
          </select>

          <span className="tool-separator" aria-hidden="true" />

          <ToolButton
            label={ru.toolbar.link}
            active={snapshot?.link ?? false}
            onClick={openLinkDialog}
          >
            <LinkIcon />
          </ToolButton>
          <ToolButton
            label={ru.toolbar.paragraph}
            onClick={openParagraphDialog}
          >
            ¶
          </ToolButton>
          <ToolButton
            label={ru.toolbar.clearFormatting}
            onClick={() =>
              editor
                ?.chain()
                .focus()
                .unsetAllMarks()
                .unsetTextAlign()
                .setParagraphFormat({
                  indent: 0,
                  lineHeight: null,
                  spaceBefore: 0,
                  spaceAfter: 0,
                })
                .run()
            }
          >
            Очистить
          </ToolButton>
        </div>
      </div>

      {linkOpen ? (
        <div
          className="tool-modal-overlay"
          onPointerDown={(event) => {
            if (event.target === event.currentTarget) {
              setLinkOpen(false);
            }
          }}
        >
          <form
            className="tool-modal"
            role="dialog"
            aria-modal="true"
            aria-label={ru.toolbar.linkDialogTitle}
            onSubmit={(event) => {
              event.preventDefault();
              applyLink();
            }}
          >
            <h2>{ru.toolbar.linkDialogTitle}</h2>
            <label className="tool-field">
              <span>{ru.toolbar.linkLabel}</span>
              <input
                autoFocus
                type="text"
                value={linkValue}
                placeholder="https://example.com"
                onChange={(event) => setLinkValue(event.target.value)}
              />
            </label>
            <div className="tool-modal__actions">
              <button
                type="button"
                className="tool-modal__button"
                onClick={() => {
                  editor?.chain().focus().unsetLink().run();
                  setLinkOpen(false);
                }}
              >
                {ru.toolbar.remove}
              </button>
              <button
                type="button"
                className="tool-modal__button"
                onClick={() => setLinkOpen(false)}
              >
                {ru.toolbar.cancel}
              </button>
              <button
                type="submit"
                className="tool-modal__button tool-modal__button--primary"
              >
                {ru.toolbar.apply}
              </button>
            </div>
          </form>
        </div>
      ) : null}

      {paragraphOpen ? (
        <div
          className="tool-modal-overlay"
          onPointerDown={(event) => {
            if (event.target === event.currentTarget) {
              setParagraphOpen(false);
            }
          }}
        >
          <form
            className="tool-modal"
            role="dialog"
            aria-modal="true"
            aria-label={ru.toolbar.paragraphDialogTitle}
            onSubmit={(event) => {
              event.preventDefault();
              applyParagraphFormat();
            }}
          >
            <h2>{ru.toolbar.paragraphDialogTitle}</h2>
            <label className="tool-field">
              <span>{ru.toolbar.indentLabel}</span>
              <input
                type="number"
                min="0"
                max="8"
                step="1"
                value={indent}
                onChange={(event) => setIndent(event.target.value)}
              />
            </label>
            <label className="tool-field">
              <span>{ru.toolbar.spaceBeforeLabel}</span>
              <input
                type="number"
                min="0"
                max="72"
                step="1"
                value={spaceBefore}
                onChange={(event) => setSpaceBefore(event.target.value)}
              />
            </label>
            <label className="tool-field">
              <span>{ru.toolbar.spaceAfterLabel}</span>
              <input
                type="number"
                min="0"
                max="72"
                step="1"
                value={spaceAfter}
                onChange={(event) => setSpaceAfter(event.target.value)}
              />
            </label>
            <div className="tool-modal__actions">
              <button
                type="button"
                className="tool-modal__button"
                onClick={() => setParagraphOpen(false)}
              >
                {ru.toolbar.cancel}
              </button>
              <button
                type="submit"
                className="tool-modal__button tool-modal__button--primary"
              >
                {ru.toolbar.apply}
              </button>
            </div>
          </form>
        </div>
      ) : null}
    </>
  );
}
