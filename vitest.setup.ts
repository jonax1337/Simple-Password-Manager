import '@testing-library/jest-dom'
import { vi } from 'vitest'

// Mock Tauri APIs
global.window = global.window || {};

(global.window as any).__TAURI__ = {
  tauri: {
    invoke: vi.fn(),
  },
  event: {
    listen: vi.fn(),
    emit: vi.fn(),
  },
};

// Mock Tauri Internals for Window API
(global.window as any).__TAURI_INTERNALS__ = {
  metadata: {
    currentWindow: {
      label: 'main',
      scaleFactor: 1,
      isClosable: true,
      isMaximized: false,
      isMinimized: false,
      isDecorated: true,
      isResizable: true,
      isVisible: true,
    },
  },
  transformCallback: vi.fn((callback: any, once?: boolean) => {
    return callback;
  }),
  invoke: vi.fn().mockResolvedValue(undefined),
  listeners: new Map(),
  channels: [],
  unregisterListener: vi.fn((id: number) => {
    return Promise.resolve();
  }),
};

// Mock matchMedia
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
})
