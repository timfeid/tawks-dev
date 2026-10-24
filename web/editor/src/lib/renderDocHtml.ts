import { generateHTML } from '@tiptap/core';

import { extensions } from './extensions';
import { sanitizeHtml } from './sanitize';

/// Static HTML for read-only content: same schema as the editor, no editor
/// instance, no ProseMirror view.
export function renderDocHtml(content: unknown): string {
  if (content == null) return '';
  if (typeof content === 'string') return sanitizeHtml(content);
  return sanitizeHtml(generateHTML(content as Parameters<typeof generateHTML>[0], extensions));
}
