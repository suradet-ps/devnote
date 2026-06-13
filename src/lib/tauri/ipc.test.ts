import { describe, it, expect, vi } from 'vitest';

describe('ipc wrapper', () => {
  it('exposes a frontendReady call', async () => {
    const { ipc } = await import('./ipc');
    // The wrapper must exist; we don't need to invoke it (Tauri runtime
    // is not available in unit tests) — we just check the surface.
    expect(typeof ipc.frontendReady).toBe('function');
    expect(typeof ipc.getPending).toBe('function');
    expect(typeof ipc.openFile).toBe('function');
    expect(typeof ipc.readFile).toBe('function');
    expect(typeof ipc.saveFile).toBe('function');
    expect(typeof ipc.setWindowTitle).toBe('function');
  });
});
