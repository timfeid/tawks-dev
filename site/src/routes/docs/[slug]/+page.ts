import { error } from '@sveltejs/kit';
import { docs, findDoc, headings, render } from '$lib/docs';

export function entries() {
  return docs.map((d) => ({ slug: d.slug }));
}

export function load({ params }) {
  const doc = findDoc(params.slug);
  if (!doc) error(404, 'No such page');
  const index = docs.indexOf(doc);
  return {
    slug: doc.slug,
    title: doc.title,
    html: render(doc),
    toc: headings(doc),
    nav: docs.map((d) => ({ slug: d.slug, title: d.title })),
    prev: docs[index - 1] ? { slug: docs[index - 1].slug, title: docs[index - 1].title } : null,
    next: docs[index + 1] ? { slug: docs[index + 1].slug, title: docs[index + 1].title } : null,
  };
}
