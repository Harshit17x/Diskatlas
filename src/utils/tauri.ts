import { invoke } from '@tauri-apps/api/core';
import type { AppError } from '../types/bindings';

/**
 * Type-safe Tauri IPC wrapper.
 * Catches errors and formats them into predictable AppError objects.
 */
export async function safeInvoke<T>(
  command: string,
  args?: Record<string, unknown>
): Promise<{ ok: true; data: T } | { ok: false; error: AppError }> {
  try {
    const data = await invoke<T>(command, args);
    return { ok: true, data };
  } catch (err: unknown) {
    if (typeof err === 'object' && err !== null && 'code' in err && 'message' in err) {
      return { ok: false, error: err as AppError };
    }
    const message = typeof err === 'string' ? err : JSON.stringify(err);
    return {
      ok: false,
      error: {
        code: 'IPC_ERROR',
        message: message || 'Unknown error occurred while calling desktop backend',
      },
    };
  }
}

