export const MENU_EVENT = {
  new: "menu:file:new",
  open: "menu:file:open",
  save: "menu:file:save",
  saveAs: "menu:file:save-as",
  closeDocument: "menu:file:close-document",
  closeWindow: "menu:file:close-window",
  printFallback: "menu:file:print-fallback",
  recent: "menu:file:recent",
  recentChanged: "menu:recent:changed",
  quit: "menu:app:quit",
  quitRequested: "menu:app:quit-requested",
} as const;
