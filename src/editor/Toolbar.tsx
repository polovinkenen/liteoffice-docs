import type { ReactNode } from "react";
import type { Editor } from "@tiptap/react";
import { ru } from "../i18n/ru";

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

export default function Toolbar({ editor }: ToolbarProps) {
  const canUndo = editor?.can().chain().focus().undo().run() ?? false;
  const canRedo = editor?.can().chain().focus().redo().run() ?? false;

  return (
    <div className="toolbar" role="toolbar" aria-label="Форматирование">
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

      <ToolButton
        label={ru.toolbar.heading1}
        active={editor?.isActive("heading", { level: 1 }) ?? false}
        onClick={() => editor?.chain().focus().toggleHeading({ level: 1 }).run()}
      >
        H1
      </ToolButton>
      <ToolButton
        label={ru.toolbar.heading2}
        active={editor?.isActive("heading", { level: 2 }) ?? false}
        onClick={() => editor?.chain().focus().toggleHeading({ level: 2 }).run()}
      >
        H2
      </ToolButton>

      <span className="tool-separator" aria-hidden="true" />

      <ToolButton
        label={ru.toolbar.bold}
        active={editor?.isActive("bold") ?? false}
        onClick={() => editor?.chain().focus().toggleBold().run()}
      >
        <strong>Ж</strong>
      </ToolButton>
      <ToolButton
        label={ru.toolbar.italic}
        active={editor?.isActive("italic") ?? false}
        onClick={() => editor?.chain().focus().toggleItalic().run()}
      >
        <em>К</em>
      </ToolButton>

      <span className="tool-separator" aria-hidden="true" />

      <ToolButton
        label={ru.toolbar.bulletList}
        active={editor?.isActive("bulletList") ?? false}
        onClick={() => editor?.chain().focus().toggleBulletList().run()}
      >
        ••
      </ToolButton>
      <ToolButton
        label={ru.toolbar.orderedList}
        active={editor?.isActive("orderedList") ?? false}
        onClick={() => editor?.chain().focus().toggleOrderedList().run()}
      >
        1.
      </ToolButton>
      <ToolButton
        label={ru.toolbar.blockquote}
        active={editor?.isActive("blockquote") ?? false}
        onClick={() => editor?.chain().focus().toggleBlockquote().run()}
      >
        ❝
      </ToolButton>
      <ToolButton
        label={ru.toolbar.codeBlock}
        active={editor?.isActive("codeBlock") ?? false}
        onClick={() => editor?.chain().focus().toggleCodeBlock().run()}
      >
        {"</>"}
      </ToolButton>
    </div>
  );
}
