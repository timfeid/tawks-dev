import { Node, mergeAttributes } from '@tiptap/core';
import { SvelteNodeViewRenderer } from '$lib';

import CounterComponent from './Counter.svelte';
import EditableComponent from './Editable.svelte';
import { HtmlTag } from 'svelte/internal';

export const SvelteCounterExtension = Node.create({
  name: 'SvelteCounterComponent',
  group: 'block',
  atom: true,
  draggable: true,
  inline: false,

  addAttributes() {
    return {
      count: {
        default: 0,
      },
    };
  },

  parseHTML() {
    return [{ tag: 'svelte-counter-component' }];
  },

  renderHTML({ HTMLAttributes }) {
    return ['svelte-counter-component', mergeAttributes(HTMLAttributes)];
  },

  addNodeView() {
    return SvelteNodeViewRenderer(CounterComponent);
  },
});

export const SvelteEditableExtension = Node.create({
  name: 'SvelteEditableComponent',
  group: 'block',
  content: 'inline*',

  addAttributes() {
    return {
      analysts: {
        default: 'W10=',
      },
    };
  },

  parseHTML() {
    return [{ tag: 'svelte-editable-component' }];
  },

  renderHTML({ HTMLAttributes }) {
    return ['svelte-editable-component', mergeAttributes(HTMLAttributes)];
  },

  addNodeView() {
    return SvelteNodeViewRenderer(EditableComponent);
  },
});
