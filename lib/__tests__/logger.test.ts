import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { logger } from '../logger'

describe('Logger', () => {
  let consoleLogSpy: ReturnType<typeof vi.spyOn>
  let consoleWarnSpy: ReturnType<typeof vi.spyOn>
  let consoleErrorSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {})
    consoleWarnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
  })

  afterEach(() => {
    consoleLogSpy.mockRestore()
    consoleWarnSpy.mockRestore()
    consoleErrorSpy.mockRestore()
  })

  describe('debug', () => {
    it('should log debug messages in development', () => {
      logger.debug('Debug message')
      expect(consoleLogSpy).toHaveBeenCalledWith('[DEBUG]', 'Debug message', '')
    })

    it('should include category when provided', () => {
      logger.debug('Debug with category', { category: 'TestCategory' })
      expect(consoleLogSpy).toHaveBeenCalledWith('[TestCategory]', 'Debug with category', '')
    })

    it('should include data when provided', () => {
      const testData = { key: 'value' }
      logger.debug('Debug with data', { data: testData })
      expect(consoleLogSpy).toHaveBeenCalledWith('[DEBUG]', 'Debug with data', testData)
    })
  })

  describe('info', () => {
    it('should log info messages', () => {
      logger.info('Info message')
      expect(consoleLogSpy).toHaveBeenCalledWith('[INFO]', 'Info message', '')
    })

    it('should include category and data', () => {
      logger.info('Info', { category: 'Test', data: { id: 1 } })
      expect(consoleLogSpy).toHaveBeenCalledWith('[Test]', 'Info', { id: 1 })
    })
  })

  describe('warn', () => {
    it('should log warning messages', () => {
      logger.warn('Warning message')
      expect(consoleWarnSpy).toHaveBeenCalledWith('[WARN]', 'Warning message', '')
    })
  })

  describe('error', () => {
    it('should always log error messages', () => {
      logger.error('Error message')
      expect(consoleErrorSpy).toHaveBeenCalledWith('[ERROR]', 'Error message', '')
    })

    it('should include error details', () => {
      const error = new Error('Test error')
      logger.error('Something failed', { category: 'Database', data: error })
      expect(consoleErrorSpy).toHaveBeenCalledWith('[Database]', 'Something failed', error)
    })
  })

  describe('isDebug', () => {
    it('should return true in development mode', () => {
      expect(logger.isDebug).toBe(true)
    })
  })
})
