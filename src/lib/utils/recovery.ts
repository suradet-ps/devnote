import type { RecoveryEntry } from '$lib/types/ipc';

/**
 * Hash used to skip recovery writes when nothing changed.
 *
 * Must exclude `saved_at` — a fresh timestamp on every autosave tick would
 * make the hash differ even when the content is identical, defeating the
 * coalescing entirely.
 */
export function recoveryHash(entries: RecoveryEntry[]): string {
  return entries
    .map((e) => `${e.path ?? ''}\u0000${e.content}`)
    .join('\u0001');
}
