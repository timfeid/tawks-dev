import DOMPurify from 'dompurify';

const BLOCKED_TAGS = ['style', 'form', 'input', 'button', 'textarea', 'select', 'option', 'iframe', 'frame', 'object', 'embed', 'base', 'meta', 'link'];

const KEPT_CLASSES = new Set(['community', 'mention', 'aspect-video']);

function keepSafeAttributes(node: Element) {
  const classes = node.getAttribute('class');
  if (classes !== null) {
    const kept = classes.split(/\s+/).filter((c) => KEPT_CLASSES.has(c) || /^language-[a-z0-9-]+$/.test(c));
    if (kept.length) node.setAttribute('class', kept.join(' '));
    else node.removeAttribute('class');
  }
  if (node.tagName === 'A') {
    const href = node.getAttribute('href') ?? '';
    if (/^https?:/i.test(href)) {
      node.setAttribute('target', '_blank');
      node.setAttribute('rel', 'noopener noreferrer nofollow');
    } else {
      node.removeAttribute('target');
    }
  }
}

let hooked = false;

/** DOMPurify needs a DOM, so during server rendering there is nothing to sanitize with. */
function purifier(): typeof DOMPurify | null {
  if (typeof DOMPurify.addHook !== 'function' || !DOMPurify.isSupported) return null;
  if (!hooked) {
    DOMPurify.addHook('afterSanitizeAttributes', keepSafeAttributes);
    hooked = true;
  }
  return DOMPurify;
}

/**
 * Makes HTML from bots and page editors safe to render: no scripts, event
 * handlers, script URLs, forms, frames or styling that could cover the app.
 */
export function sanitizeHtml(html: string): string {
  const purify = purifier();
  if (!purify) return '';
  return purify.sanitize(html, {
    USE_PROFILES: { html: true },
    FORBID_TAGS: BLOCKED_TAGS,
    FORBID_ATTR: ['style', 'id'],
    ALLOWED_URI_REGEXP: /^(?:https?:|mailto:|\/(?![/\\])|#)/i,
  });
}

/** Inline formatting only, for short labels such as tooltips. */
export function sanitizeInlineHtml(html: string): string {
  const purify = purifier();
  if (!purify) return '';
  return purify.sanitize(html, {
    ALLOWED_TAGS: ['b', 'strong', 'i', 'em', 'u', 's', 'code', 'br', 'span', 'small', 'p', 'ul', 'ol', 'li'],
    ALLOWED_ATTR: [],
  });
}
