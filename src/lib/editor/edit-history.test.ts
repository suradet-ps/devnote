import { describe, it, expect } from 'vitest';
import { EditHistory } from './edit-history';

describe('EditHistory', () => {
  it('pushes positions and walks back to the first', () => {
    const h = new EditHistory();
    h.push(10);
    h.push(25);
    h.push(40);
    expect(h.back()).toBe(25);
    expect(h.back()).toBe(10);
    expect(h.back()).toBeNull(); // at the start
  });

  it('walks forward after going back', () => {
    const h = new EditHistory();
    h.push(10);
    h.push(25);
    h.push(40);
    h.back();
    h.back();
    expect(h.forward()).toBe(25);
    expect(h.forward()).toBe(40);
    expect(h.forward()).toBeNull(); // at the end
  });

  it('coalesces consecutive edits at the same position', () => {
    const h = new EditHistory();
    h.push(5);
    h.push(5);
    h.push(9);
    expect(h.back()).toBe(5);
    expect(h.back()).toBeNull();
  });

  it('drops the forward trail when a new edit is pushed', () => {
    const h = new EditHistory();
    h.push(10);
    h.push(20);
    h.push(30);
    h.back();
    h.back();
    h.push(99); // new edit from position 10
    expect(h.forward()).toBeNull();
    expect(h.back()).toBe(10);
    expect(h.back()).toBeNull();
  });

  it('caps the history at max entries', () => {
    const h = new EditHistory(3);
    h.push(1);
    h.push(2);
    h.push(3);
    h.push(4);
    expect(h.back()).toBe(3);
    expect(h.back()).toBe(2);
    expect(h.back()).toBeNull(); // 1 was evicted
  });

  it('back/forward on empty history returns null', () => {
    const h = new EditHistory();
    expect(h.back()).toBeNull();
    expect(h.forward()).toBeNull();
  });

  it('reset clears everything', () => {
    const h = new EditHistory();
    h.push(1);
    h.reset();
    expect(h.back()).toBeNull();
    expect(h.current).toBeNull();
  });
});
