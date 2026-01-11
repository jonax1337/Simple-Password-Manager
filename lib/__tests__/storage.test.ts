import { describe, it, expect, beforeEach, vi } from 'vitest'
import { 
  getRecentDatabases, 
  addRecentDatabase,
  getHibpEnabled,
  setHibpEnabled,
} from '../storage'

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {}

  return {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => {
      store[key] = value.toString()
    },
    removeItem: (key: string) => {
      delete store[key]
    },
    clear: () => {
      store = {}
    },
  }
})()

Object.defineProperty(window, 'localStorage', {
  value: localStorageMock,
})

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

describe('Storage', () => {
  beforeEach(() => {
    localStorageMock.clear()
  })

  describe('Recent Databases', () => {
    it('should return empty array when no recent databases', () => {
      const recent = getRecentDatabases()
      expect(recent).toEqual([])
    })

    it('should add database to recent list', () => {
      addRecentDatabase('/path/to/database.kdbx')
      const recent = getRecentDatabases()
      expect(recent).toContain('/path/to/database.kdbx')
    })

    it('should not add duplicates', () => {
      addRecentDatabase('/path/to/database.kdbx')
      addRecentDatabase('/path/to/database.kdbx')
      const recent = getRecentDatabases()
      expect(recent.filter(p => p === '/path/to/database.kdbx')).toHaveLength(1)
    })

    it('should limit recent databases to 10', () => {
      for (let i = 0; i < 15; i++) {
        addRecentDatabase(`/path/to/database${i}.kdbx`)
      }
      const recent = getRecentDatabases()
      expect(recent).toHaveLength(10)
    })

    it('should move existing database to top when re-added', () => {
      addRecentDatabase('/path/to/first.kdbx')
      addRecentDatabase('/path/to/second.kdbx')
      addRecentDatabase('/path/to/first.kdbx') // Re-add first
      const recent = getRecentDatabases()
      expect(recent[0]).toBe('/path/to/first.kdbx')
    })

  })

  describe('HIBP Settings', () => {
    it('should default to false', () => {
      const enabled = getHibpEnabled()
      expect(enabled).toBe(false)
    })

    it('should save and retrieve HIBP setting', () => {
      setHibpEnabled(true)
      expect(getHibpEnabled()).toBe(true)

      setHibpEnabled(false)
      expect(getHibpEnabled()).toBe(false)
    })
  })
})
