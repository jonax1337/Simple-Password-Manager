/**
 * Structured logging utility for development and production
 * 
 * In production: Only logs errors
 * In development: Logs everything for debugging
 */

type LogLevel = 'debug' | 'info' | 'warn' | 'error';

interface LogOptions {
  category?: string;
  data?: unknown;
}

class Logger {
  private isDevelopment: boolean;

  constructor() {
    // Check if running in development mode
    // In Tauri apps, process.env is available
    this.isDevelopment = process.env.NODE_ENV !== 'production';
  }

  /**
   * Debug logs - only shown in development
   * Use for: Flow tracking, state changes, event handling
   */
  debug(message: string, options?: LogOptions): void {
    if (!this.isDevelopment) return;
    
    const prefix = options?.category ? `[${options.category}]` : '[DEBUG]';
    console.log(prefix, message, options?.data ?? '');
  }

  /**
   * Info logs - only shown in development
   * Use for: Important state transitions, successful operations
   */
  info(message: string, options?: LogOptions): void {
    if (!this.isDevelopment) return;
    
    const prefix = options?.category ? `[${options.category}]` : '[INFO]';
    console.log(prefix, message, options?.data ?? '');
  }

  /**
   * Warning logs - shown in development, silent in production
   * Use for: Recoverable errors, deprecated features, validation warnings
   */
  warn(message: string, options?: LogOptions): void {
    if (!this.isDevelopment) return;
    
    const prefix = options?.category ? `[${options.category}]` : '[WARN]';
    console.warn(prefix, message, options?.data ?? '');
  }

  /**
   * Error logs - always shown
   * Use for: Exceptions, critical failures that need attention
   */
  error(message: string, options?: LogOptions): void {
    const prefix = options?.category ? `[${options.category}]` : '[ERROR]';
    console.error(prefix, message, options?.data ?? '');
  }

  /**
   * Check if running in development mode
   */
  get isDebug(): boolean {
    return this.isDevelopment;
  }
}

// Export singleton instance
export const logger = new Logger();

// Export type for external use
export type { LogLevel, LogOptions };
