import { describe, it, expect, beforeEach } from 'vitest';
import goldenJson from '../../tests/golden_cases.json';
import { tabsStore, type FilePayload, type Tab } from './stores/tabs.svelte';

/**
 * Golden-case harness (Roadmap Phase 2): drives the user-visible tab/store
 * invariants from `tests/golden_cases.json`. A regression in any case fails
 * this test — no separate invocation needed, it runs with `bun run test`.
 */

interface GoldenCase {
  id: string;
  description: string;
  payload: FilePayload;
  expect: {
    language?: string;
    encoding?: string;
    lineEnding?: string;
    dirty?: boolean;
    tabCount?: number;
  };
  scenario?: ScenarioStep[];
}

type ScenarioStep =
  | { op: 'edit'; content: string }
  | { op: 'open'; payload: FilePayload }
  | { op: 'saveAs'; path: string }
  | {
      op: 'expect';
      dirty?: boolean;
      path?: string;
      fileName?: string;
      tabCount?: number;
      closeRejected?: boolean;
    };

const golden: { cases: GoldenCase[] } = goldenJson as unknown as { cases: GoldenCase[] };

function expectTabState(tab: Tab | undefined, expected: NonNullable<GoldenCase['expect']>, label: string): void {
  expect(tab, label).toBeDefined();
  if (!tab) return;
  if (expected.language !== undefined) expect(tab.language, `${label}: language`).toBe(expected.language);
  if (expected.encoding !== undefined) expect(tab.encoding, `${label}: encoding`).toBe(expected.encoding);
  if (expected.lineEnding !== undefined) expect(tab.lineEnding, `${label}: lineEnding`).toBe(expected.lineEnding);
  if (expected.dirty !== undefined) {
    expect(tab.content !== tab.savedContent, `${label}: dirty`).toBe(expected.dirty);
  }
  if (expected.tabCount !== undefined) expect(tabsStore.tabs.length, `${label}: tabCount`).toBe(expected.tabCount);
}

describe('golden cases', () => {
  beforeEach(() => {
    tabsStore.__resetForTests();
  });

  for (const c of golden.cases) {
    it(`${c.id}: ${c.description}`, () => {
      const opened = tabsStore.openTab(c.payload);
      expectTabState(opened, c.expect, 'after open');

      for (const step of c.scenario ?? []) {
        switch (step.op) {
          case 'edit': {
            const tab = tabsStore.activeTab;
            expect(tab, 'edit: active tab').toBeDefined();
            if (!tab) break;
            const content = step.content === '@restore' ? c.payload.content : step.content;
            tabsStore.updateContent(tab.id, content);
            break;
          }
          case 'open':
            tabsStore.openTab(step.payload);
            break;
          case 'saveAs': {
            const tab = tabsStore.activeTab;
            expect(tab, 'saveAs: active tab').toBeDefined();
            if (!tab) break;
            tabsStore.markSaved(tab.id, step.path);
            break;
          }
          case 'expect': {
            const tab = tabsStore.activeTab;
            if (step.closeRejected) {
              expect(tab, 'closeRejected: active tab').toBeDefined();
              if (tab) expect(tabsStore.closeTab(tab.id)).toBe(false);
            }
            if (step.dirty !== undefined || step.path !== undefined || step.fileName !== undefined) {
              expectTabState(tab, { dirty: step.dirty }, `scenario ${c.id}`);
              expect(tab?.path, 'scenario path').toBe(step.path ?? c.payload.path);
              expect(tab?.fileName, 'scenario fileName').toBe(step.fileName ?? c.payload.file_name);
            }
            if (step.tabCount !== undefined) {
              expect(tabsStore.tabs.length, 'scenario tabCount').toBe(step.tabCount);
            }
            break;
          }
        }
      }
    });
  }
});
