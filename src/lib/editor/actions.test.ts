// @vitest-environment happy-dom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { dispatchEditorAction, onEditorAction, type EditorAction } from './actions';

describe('EditorAction bus', () => {
  let handler: (a: EditorAction) => void;
  let removeListener: () => void;

  beforeEach(() => {
    handler = vi.fn();
    removeListener = onEditorAction(handler);
  });

  afterEach(() => {
    removeListener();
  });

  it('dispatches and receives a simple action', () => {
    dispatchEditorAction({ action: 'undo' });
    expect(handler).toHaveBeenCalledWith({ action: 'undo' });
  });

  it('dispatches and receives a payload action', () => {
    dispatchEditorAction({ action: 'go-to-line', line: 42 });
    expect(handler).toHaveBeenCalledWith({ action: 'go-to-line', line: 42 });
  });

  it('stops receiving after unsubscribe', () => {
    removeListener();
    dispatchEditorAction({ action: 'redo' });
    expect(handler).not.toHaveBeenCalled();
  });

  it('multiple listeners can subscribe independently', () => {
    const h2 = vi.fn();
    const off2 = onEditorAction(h2);
    dispatchEditorAction({ action: 'select-all' });
    expect(handler).toHaveBeenCalledOnce();
    expect(h2).toHaveBeenCalledOnce();
    off2();
  });

  it('ignores events without a detail', () => {
    window.dispatchEvent(new CustomEvent('editor-action'));
    expect(handler).not.toHaveBeenCalled();
  });
});
