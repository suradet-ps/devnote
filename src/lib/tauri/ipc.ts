import { invoke } from '@tauri-apps/api/core';
import type { FilePayload, RecoveryEntry } from '$lib/types/ipc';

export const ipc = {
  openFile: () => invoke<FilePayload | null>('open_file'),
  readFile: (path: string) => invoke<FilePayload>('read_file', { path }),

  /**
   * Returns the file size in bytes as a decimal string.
   * String avoids JS Number precision loss on files > 2^53.
   */
  checkFileSize: (path: string) => invoke<string>('check_file_size', { path }),

  saveFile: (args: {
    path: string;
    content: string;
    lineEnding: string;
    encoding: string;
  }) => invoke<void>('save_file', args),

  saveFileAs: (args: {
    content: string;
    suggestedName: string;
    lineEnding: string;
    encoding: string;
  }) => invoke<string | null>('save_file_as', args),

  addRecent: (path: string) => invoke<void>('add_recent_file', { path }),
  getRecent: () => invoke<string[]>('get_recent_files'),
  removeRecent: (path: string) => invoke<void>('remove_recent_file', { path }),

  getPending: () => invoke<string[]>('get_pending_files'),

  /**
   * Signal to Rust that the frontend is ready. Rust will then
   * re-emit any pending files via the `file-opened` Tauri event
   * so this frontend can pick them up. This closes the race
   * where the frontend's `listen('file-opened')` is not yet
   * registered when the OS hands the file to the app.
   */
  frontendReady: () => invoke<string[]>('frontend_ready'),

  saveRecovery: (tabs: RecoveryEntry[]) =>
    invoke<void>('save_recovery_data', { tabs }),
  checkRecovery: () => invoke<RecoveryEntry[] | null>('check_recovery_data'),
  clearRecovery: () => invoke<void>('clear_recovery_data'),

  setWindowTitle: (title: string) =>
    invoke<void>('set_window_title', { title }),
} as const;
