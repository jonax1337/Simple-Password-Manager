/// <reference types="svelte" />
/// <reference types="vite/client" />

declare global {
  interface Window {
    __pwLastDraggedEntryGroup?: string | null;
  }
}

export {};

