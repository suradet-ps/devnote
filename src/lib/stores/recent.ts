import { invoke } from '@tauri-apps/api/core';

let _recentFiles = $state<string[]>([]);

export const recentStore = {
  get recentFiles() { return _recentFiles; },

  async refresh() {
    try {
      const files = await invoke<string[]>('get_recent_files');
      _recentFiles = files;
    } catch (e) {
      console.error('Failed to load recent files:', e);
    }
  },

  async add(path: string) {
    try {
      await invoke('add_recent_file', { path });
      await this.refresh();
    } catch (e) {
      console.error('Failed to add recent file:', e);
    }
  },

  async remove(path: string) {
    try {
      await invoke('remove_recent_file', { path });
      await this.refresh();
    } catch (e) {
      console.error('Failed to remove recent file:', e);
    }
  }
};
