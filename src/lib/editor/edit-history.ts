/**
 * Edit-site history (Roadmap Phase 4): remembers cursor positions after edits
 * so the user can jump back/forward between edit locations (Ctrl+Alt+-/=).
 * Pure and testable — the editor wires it into its update listener.
 */
export class EditHistory {
  private positions: number[] = [];
  private index = -1;

  constructor(private max: number = 50) {}

  /** Record a cursor position after an edit. Clears the forward trail. */
  push(pos: number): void {
    // Coalesce consecutive edits at the same position
    if (this.index >= 0 && this.positions[this.index] === pos) return;
    // Drop stale forward history
    this.positions = this.positions.slice(0, this.index + 1);
    this.positions.push(pos);
    if (this.positions.length > this.max) {
      this.positions.shift();
    }
    this.index = this.positions.length - 1;
  }

  /** Move to the previous edit site; null when there is none. */
  back(): number | null {
    if (this.index <= 0) return null;
    this.index--;
    return this.positions[this.index];
  }

  /** Move to the next edit site; null when there is none. */
  forward(): number | null {
    if (this.index < 0 || this.index >= this.positions.length - 1) return null;
    this.index++;
    return this.positions[this.index];
  }

  get current(): number | null {
    return this.index >= 0 ? this.positions[this.index] : null;
  }

  reset(): void {
    this.positions = [];
    this.index = -1;
  }
}
