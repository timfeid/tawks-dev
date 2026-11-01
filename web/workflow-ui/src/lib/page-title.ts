import type { WorkflowDisplay } from '@tawks/workflow-types';
import { evaluateCondition } from './utils';

type Inputs = Record<string, any>;

const PLACEHOLDERS = new Set(['Loading', 'ServerActionLoader']);
const MAX_DEPTH = 6;

function kindOf(display: WorkflowDisplay): string {
  return typeof display.type === 'string' ? display.type : (Object.keys(display.type)[0] ?? '');
}

function bodyOf(display: WorkflowDisplay): Record<string, any> | null {
  if (typeof display.type === 'string') return null;
  return (display.type as Record<string, Record<string, any>>)[kindOf(display)] ?? null;
}

function isDisplay(value: unknown): value is WorkflowDisplay {
  return typeof value === 'object' && value !== null && 'type' in value;
}

function childrenOf(display: WorkflowDisplay): WorkflowDisplay[] {
  const body = bodyOf(display);
  if (!body) return [];
  return Object.values(body).flatMap((value) =>
    Array.isArray(value) ? value.filter(isDisplay) : isDisplay(value) ? [value] : [],
  );
}

/** What a page shows at its top level, with its true conditions opened up. */
function shown(content: WorkflowDisplay[], inputs: Inputs): WorkflowDisplay[] {
  return content.flatMap((display) => {
    const body = bodyOf(display);
    if (kindOf(display) !== 'Condition' || !body) return [display];
    return evaluateCondition(body.condition, inputs) ? shown(body.content, inputs) : [];
  });
}

function hasTitle(display: WorkflowDisplay, inputs: Inputs, depth: number): boolean {
  const kind = kindOf(display);
  if (kind === 'Hero') return true;
  if (kind === 'Typography') {
    const role = bodyOf(display)?.role;
    return role === 'Title' || role === 'Display';
  }
  if (depth >= MAX_DEPTH) return false;
  if (kind === 'Condition') {
    const body = bodyOf(display);
    if (!body || !evaluateCondition(body.condition, inputs)) return false;
  }
  return childrenOf(display).some((child) => hasTitle(child, inputs, depth + 1));
}

/**
 * Whether a page needs the host's title above it: not when its first block
 * already names it with a hero or a title, and not while it is still loading.
 */
export function needsPageTitle(content: WorkflowDisplay[], inputs: Inputs): boolean {
  const blocks = shown(content, inputs).filter((display) => kindOf(display) !== 'Notice');
  if (blocks.every((display) => PLACEHOLDERS.has(kindOf(display)))) return false;
  return !hasTitle(blocks[0], inputs, 0);
}
