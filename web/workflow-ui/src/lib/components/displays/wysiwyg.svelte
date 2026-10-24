<script lang="ts">
  import { createEditor, Editor, EditorContent, renderDocHtml } from '@tawks/editor';
  import { getNestedValue, cn } from '../../utils.js';
  import type { Props } from '../workflow-display.svelte';
  import { writable } from 'svelte/store';
  import { onMount } from 'svelte';

  let { display, inputs }: Props = $props();

  let wysiwyg = $derived('Wysiwyg' in display.type ? display.type.Wysiwyg : undefined);
  let value = $derived(wysiwyg ? getNestedValue(inputs, wysiwyg.content_key) : null);

  // Read-only documents render as static HTML. A full editor per node is far
  // too expensive when a feed shows dozens of them, and tearing those editors
  // down on every workflow update stalled the page for seconds. The editor is
  // kept only as a fallback for content the static renderer cannot handle.
  let html = $derived.by(() => {
    if (value === undefined || value === null) return '';
    try {
      return renderDocHtml(value);
    } catch {
      return null;
    }
  });

  const editorStore = writable<Editor | null>(null);
  let editor: Editor | null = null;

  onMount(() => {
    if (!wysiwyg || html !== null) return;
    editor = createEditor({
      content: value,
      editable: false,
      editorProps: {
        attributes: {
          class: `prose prose-invert outline-none ${display.class}`,
        },
      },
    });
    editorStore.set(editor);

    return () => {
      if (editor) {
        editor.destroy();
        editor = null;
      }
    };
  });

  $effect(() => {
    if (editor && value !== undefined && value !== null) {
      editor.commands.setContent(value);
    }
  });
</script>

{#if wysiwyg}
  {#if html !== null}
    <div
      class={cn(
        'prose prose-invert prose-code:before:content-none prose-code:after:content-none [&_blockquote_p]:before:content-none [&_blockquote_p]:after:content-none',
        display.class,
      )}
    >
      {@html html}
    </div>
  {:else if $editorStore}
    <EditorContent editor={$editorStore}></EditorContent>
  {/if}
{/if}
