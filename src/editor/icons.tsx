import type { ReactNode } from "react";

function Icon({ children }: { children: ReactNode }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {children}
    </svg>
  );
}

export function ImageIcon() {
  return (
    <Icon>
      <rect x="3" y="4" width="18" height="16" rx="2" />
      <circle cx="8.5" cy="9.5" r="1.5" />
      <path d="m21 16-5-5L5 20" />
    </Icon>
  );
}

export function TableIcon() {
  return (
    <Icon>
      <rect x="3" y="4" width="18" height="16" rx="2" />
      <path d="M3 10h18M9 4v16M15 4v16" />
    </Icon>
  );
}

export function PageBreakIcon() {
  return (
    <Icon>
      <rect x="5" y="3" width="14" height="18" rx="2" />
      <path d="M8 12h8" strokeDasharray="2 2" />
    </Icon>
  );
}

export function AddColumnIcon() {
  return (
    <Icon>
      <rect x="3" y="4" width="10" height="16" rx="2" />
      <path d="M3 10h10M8 4v16" />
      <path d="M18 9v6M15 12h6" />
    </Icon>
  );
}

export function RemoveColumnIcon() {
  return (
    <Icon>
      <rect x="3" y="4" width="10" height="16" rx="2" />
      <path d="M3 10h10M8 4v16" />
      <path d="M15 12h6" />
    </Icon>
  );
}

export function AddRowIcon() {
  return (
    <Icon>
      <rect x="4" y="3" width="16" height="10" rx="2" />
      <path d="M4 8h16M10 3v10" />
      <path d="M12 17v4M10 19h4" />
    </Icon>
  );
}

export function RemoveRowIcon() {
  return (
    <Icon>
      <rect x="4" y="3" width="16" height="10" rx="2" />
      <path d="M4 8h16M10 3v10" />
      <path d="M10 19h4" />
    </Icon>
  );
}

export function DeleteTableIcon() {
  return (
    <Icon>
      <path d="M4 7h16M10 4h4M6 7l1 13h10l1-13" />
      <path d="M10 11v6M14 11v6" />
    </Icon>
  );
}
