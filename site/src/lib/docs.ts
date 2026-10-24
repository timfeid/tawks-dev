import MarkdownIt from 'markdown-it';
import { createHighlighter } from 'shiki';
import { exampleDefinition, type DisplayExample } from './examples';
import { encode } from './playground/share';

const THEME = 'github-dark-default';
const highlighter = await createHighlighter({
  themes: [THEME],
  langs: ['rust', 'ts', 'js', 'json', 'sh', 'toml', 'html'],
});

function highlight(code: string, lang: string): string {
  const known = highlighter.getLoadedLanguages().includes(lang);
  return highlighter.codeToHtml(code, { lang: known ? lang : 'text', theme: THEME });
}

const escapeAttr = (value: string) => value.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;');

const files = import.meta.glob('../../../docs/*.md', { query: '?raw', import: 'default', eager: true }) as Record<
  string,
  string
>;

export type Doc = { slug: string; title: string; markdown: string };

function slugOf(file: string): string | null {
  const name = file.split('/').pop() ?? '';
  const match = /^\d+-([a-z0-9-]+)\.md$/.exec(name);
  return match ? match[1] : null;
}

export const docs: Doc[] = Object.entries(files)
  .sort(([a], [b]) => a.localeCompare(b))
  .flatMap(([file, markdown]) => {
    const slug = slugOf(file);
    if (!slug) return [];
    const title = /^#\s+(.+)$/m.exec(markdown)?.[1].trim() ?? slug;
    return [{ slug, title, markdown }];
  });

const bySlug = new Map(docs.map((d) => [d.slug, d]));
const fileToSlug = new Map(
  Object.keys(files).flatMap((file) => {
    const slug = slugOf(file);
    return slug ? [[file.split('/').pop()!, slug] as const] : [];
  }),
);

export function anchor(text: string): string {
  return text
    .toLowerCase()
    .replace(/<[^>]+>/g, '')
    .replace(/[^a-z0-9 -]/g, '')
    .trim()
    .replace(/\s+/g, '-');
}

const md = new MarkdownIt({ html: false, linkify: true, highlight });

// A `display` fence is a live example in the Display reference: the docs
// site draws it with the real renderer, and the JSON follows it.
const fence = md.renderer.rules.fence!;
md.renderer.rules.fence = (tokens, i, options, env, self) => {
  const token = tokens[i];
  if (token.info.trim() !== 'display') return fence(tokens, i, options, env, self);
  const example = JSON.parse(token.content) as DisplayExample;
  const playground = `/playground#p=${encode({ definition: exampleDefinition(example), mocks: {} })}`;
  return (
    `<div class="not-prose" data-display-example="${escapeAttr(JSON.stringify(example))}" data-playground="${escapeAttr(playground)}"></div>` +
    `<details class="display-example-json"><summary>Show the JSON</summary>${highlight(token.content.trim(), 'json')}</details>`
  );
};

const linkOpen = md.renderer.rules.link_open ?? ((tokens, i, options, _env, self) => self.renderToken(tokens, i, options));
md.renderer.rules.link_open = (tokens, i, options, env, self) => {
  const href = tokens[i].attrGet('href') ?? '';
  const [file, hash] = href.split('#');
  const slug = fileToSlug.get(file.replace(/^\.\//, ''));
  if (slug) tokens[i].attrSet('href', `/docs/${slug}${hash ? `#${hash}` : ''}`);
  else if (/^https?:/.test(href)) {
    tokens[i].attrSet('target', '_blank');
    tokens[i].attrSet('rel', 'noopener noreferrer');
  }
  return linkOpen(tokens, i, options, env, self);
};

const headingOpen = md.renderer.rules.heading_open ?? ((tokens, i, options, _env, self) => self.renderToken(tokens, i, options));
md.renderer.rules.heading_open = (tokens, i, options, env, self) => {
  const inline = tokens[i + 1];
  if (inline?.type === 'inline') tokens[i].attrSet('id', uniqueAnchor(inline.content, env));
  return headingOpen(tokens, i, options, env, self);
};

export function findDoc(slug: string): Doc | undefined {
  return bySlug.get(slug);
}

/** Repeated headings get `-2`, `-3` and so on, the way GitHub numbers them. */
function uniqueAnchor(text: string, env: { seen?: Map<string, number> }): string {
  env.seen ??= new Map();
  const id = anchor(text);
  const count = env.seen.get(id) ?? 0;
  env.seen.set(id, count + 1);
  return count ? `${id}-${count + 1}` : id;
}

export function render(doc: Doc): string {
  return md.render(doc.markdown, {});
}

export function headings(doc: Doc): { level: number; text: string; id: string }[] {
  const env = {};
  return md
    .parse(doc.markdown, {})
    .flatMap((t, i, all) => {
      if (t.type !== 'heading_open') return [];
      const id = uniqueAnchor(all[i + 1].content, env);
      return t.tag === 'h2' || t.tag === 'h3' ? [{ level: Number(t.tag[1]), text: all[i + 1].content, id }] : [];
    });
}
