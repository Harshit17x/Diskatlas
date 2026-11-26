import { useScanStore } from '../stores/scanStore';
import { useHistoryStore } from '../stores/historyStore';

const USABLE_STATUSES = new Set(['completed', 'completedWithErrors']);

/**
 * Resolves the best frontend session ID override if one is already active or loaded.
 * Returns `undefined` when none is known yet so backend commands can resolve the latest
 * completed session directly from SQLite.
 */
export function resolvePreferredSessionId(explicitSessionId?: string | null): string | undefined {
  if (explicitSessionId && explicitSessionId !== 'starting') {
    return explicitSessionId;
  }

  const scanState = useScanStore.getState();
  if (scanState.activeSessionId && scanState.activeSessionId !== 'starting') {
    return scanState.activeSessionId;
  }
  if (scanState.summary?.sessionId) {
    return scanState.summary.sessionId;
  }

  const sessions = useHistoryStore.getState().sessionsPage?.items;
  if (sessions && sessions.length > 0) {
    const usable =
      sessions.find((s) => USABLE_STATUSES.has(s.status) && s.totalFiles > 0) ??
      sessions.find((s) => USABLE_STATUSES.has(s.status)) ??
      sessions.find((s) => s.totalFiles > 0);
    if (usable) {
      return usable.id;
    }
  }

  return undefined;
}
