<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import type { Editor } from './Editor';

  let element: HTMLElement;
  let { editor }: { editor: Editor } = $props();

  const init = async () => {
    await tick();
    if (!editor || !editor.options.element) {
      return;
    }

    if (editor.contentElement) {
      return;
    }

    element.append(...Array.from(editor.options.element.childNodes));
    (element.firstChild as HTMLDivElement).classList.add('prose');
    (element.firstChild as HTMLDivElement).classList.add('prose-invert');
    editor.setOptions({ element });

    editor.contentElement = element;
  };

  onMount(init);

  onDestroy(() => {
    if (!editor) {
      return;
    }

    editor.contentElement = null;

    if (!editor.options.element.firstChild) {
      return;
    }

    const newElement = document.createElement('div');

    editor.setOptions({
      element: newElement,
    });
  });
</script>

<div class="relative" bind:this={element}></div>
