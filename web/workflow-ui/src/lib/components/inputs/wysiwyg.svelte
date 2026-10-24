<script lang="ts">
  import {
    BubbleMenu,
    createEditor,
    Editor,
    EditorContent,
    getHTMLContentBetween,
  } from '@tawks/editor';
  import type { BubbleMenuPluginProps } from '@tiptap/extension-bubble-menu';
  import { getNestedValue, interpolate, resolveInputValue, evaluateCondition, formatDate, cn, debounce, deepEqual, objectInterpolate } from '../../utils.js';
  import type { Props } from '../workflow-input.svelte';
  import { onMount, getContext } from 'svelte';
  import { writable } from 'svelte/store';
  import { onChange } from '../workflow.js';
  import { Button } from '@tawks/ui/button/index.js';
  import { Link, Trash } from '@lucide/svelte';
  import { Input } from '@tawks/ui/input/index.js';
  import type { WorkflowStateProvider } from '../workflow';

  let { workflow, input, inputs }: Props = $props();
  const provider = getContext<WorkflowStateProvider>('workflow-provider');
  let editor = writable<Editor | null>(null);

  function updated(payload: Record<string, unknown>) {
    onChange(provider, workflow.instance_id, inputs, { [input.id]: payload }, input.input.on_change_action as any);
  }

  const delayedUpdate = debounce(updated, 250);

  onMount(() => {
    let value = getNestedValue(inputs, input.id) || input.input.default_value;
    const e = createEditor({
      content: value || { type: 'doc', content: [] },
      editable: true,
      editorProps: {
        attributes: {
          class: 'prose prose-invert outline-none',
        },
      },
    });

    editor.set(e);

    e.on('update', (v) => {
      delayedUpdate(e.getJSON());
    });

    return () => e && e.destroy();
  });

  const shouldShowHeader: BubbleMenuPluginProps['shouldShow'] = ({ editor }) => {
    return (
      !editor.view.state.selection.empty &&
      getHTMLContentBetween(
        editor,
        editor.view.state.selection.from,
        editor.view.state.selection.to,
      ).includes('<h3>')
    );
  };

  const shouldShow: BubbleMenuPluginProps['shouldShow'] = ({ editor }) => {
    return (
      !editor.view.state.selection.empty &&
      !getHTMLContentBetween(
        editor,
        editor.view.state.selection.from,
        editor.view.state.selection.to,
      ).includes('<h3>')
    );
  };

  const shouldShowLink: BubbleMenuPluginProps['shouldShow'] = ({ editor }) => {
    return showLink;
  };

  function createLinkForSelection() {
    const e = getEditor();
    if (e) e.commands.setLink({ href: currentLink });
  }

  function removeLinkForSelection() {
    const e = getEditor();
    if (e) e.commands.unsetLink();
  }

  function getEditor() {
      let e: Editor | null = null;
      editor.subscribe(v => e = v)();
      return e;
  }

  let showLink = $state(false);
  let currentLink = $state('');
  $effect(() => {
    const e = getEditor();
    if (e) {
      currentLink = e.getAttributes('link').href;
    }
  });

  let showHeading = $state(false);
  let headingLevel = $state(2);

  function setHeading(level: number) {
    const e = getEditor();
    if (e) {
        e.chain().focus().toggleHeading({ level }).run();
        showHeading = false;
        headingLevel = level;
    }
  }

  $effect(() => {
    const e = getEditor();
    if (e) {
      // get current selection heading level
      const attrs = e.getAttributes('heading');
      if (attrs.level) {
        headingLevel = attrs.level;
      } else {
        headingLevel = 0;
      }
    }
  });
  let element: HTMLElement;
</script>

{#if $editor}
  <EditorContent editor={$editor} />
  <BubbleMenu pluginKey="link-menu" shouldShow={shouldShowLink} editor={$editor}>
    {#if showLink}
      <form
        onsubmit={createLinkForSelection}
        class="flex items-end space-x-1 rounded bg-neutral-950 p-3"
        onmouseleave={() => (showLink = false)}
      >
        <label>
          Link
          <Input
            value={currentLink}
            oninput={(v: any) => (currentLink = v.currentTarget?.value || '')}
            placeholder="https://"
          />
        </label>
        <Button size="icon" type="submit"><Link size={16} /></Button>
        <Button size="icon" onclick={removeLinkForSelection} type="button" variant="ghost"
          ><Trash size={16} /></Button
        >
      </form>
    {/if}
  </BubbleMenu>
  <BubbleMenu pluginKey="header-menu" editor={$editor} shouldShow={shouldShowHeader}>
    <div class="flex items-center gap-1 bg-neutral-950">
      <!-- Heading selection button -->
      <Button
        onclick={() => {
          showHeading = !showHeading;
          if ($editor) $editor.chain().focus();
        }}
        class="h-5 w-10"
        aria-label="Toggle heading menu"
        type="button"
      >
        H
      </Button>
      {#if showHeading}
        <div
          class="absolute mt-2 flex flex-col rounded border border-neutral-800 bg-neutral-900 p-2 shadow"
        >
          <Button
            size="sm"
            class="mb-1 text-lg font-bold"
            onclick={() => setHeading(1)}
            data-active={headingLevel === 1}
            aria-label="Heading 1"
            type="button">H1</Button
          >
          <Button
            size="sm"
            class="mb-1 text-base"
            onclick={() => setHeading(2)}
            data-active={headingLevel === 2}
            aria-label="Heading 2"
            type="button">H2</Button
          >
          <Button
            size="sm"
            class="text-sm"
            onclick={() => setHeading(3)}
            data-active={headingLevel === 3}
            aria-label="Heading 3"
            type="button">H3</Button
          >
          <Button
            size="sm"
            class="mt-1"
            onclick={() => {
              if ($editor) $editor.chain().focus().setParagraph().run();
              showHeading = false;
              headingLevel = 0;
            }}
            data-active={headingLevel === 0}
            aria-label="Paragraph"
            type="button">P</Button
          >
        </div>
      {/if}
      <Button
        onclick={() => {
          showLink = true;
          if ($editor) $editor.chain().focus();
        }}
        class="link h-5 w-10"
        aria-label="Add Link"
        type="button">A</Button
      >
    </div>
  </BubbleMenu>
  <BubbleMenu editor={$editor} {shouldShow}>
    <div class="flex items-center gap-1 bg-neutral-950">
      <!-- Heading selection button (outside h3 selection) -->
      <Button
        onclick={() => {
          showHeading = !showHeading;
          if ($editor) $editor.chain().focus();
        }}
        class="h-5 w-10"
        aria-label="Toggle heading menu"
        type="button"
      >
        H
      </Button>
      {#if showHeading}
        <div
          class="absolute mt-2 flex flex-col rounded border border-neutral-800 bg-neutral-900 p-2 shadow"
        >
          <Button
            size="sm"
            class="mb-1 text-lg font-bold"
            onclick={() => setHeading(1)}
            data-active={headingLevel === 1}
            aria-label="Heading 1"
            type="button">H1</Button
          >
          <Button
            size="sm"
            class="mb-1 text-base"
            onclick={() => setHeading(2)}
            data-active={headingLevel === 2}
            aria-label="Heading 2"
            type="button">H2</Button
          >
          <Button
            size="sm"
            class="text-sm"
            onclick={() => setHeading(3)}
            data-active={headingLevel === 3}
            aria-label="Heading 3"
            type="button">H3</Button
          >
          <Button
            size="sm"
            class="mt-1"
            onclick={() => {
              if ($editor) $editor.chain().focus().setParagraph().run();
              showHeading = false;
              headingLevel = 0;
            }}
            data-active={headingLevel === 0}
            aria-label="Paragraph"
            type="button">P</Button
          >
        </div>
      {/if}
      <Button onclick={() => { if ($editor) $editor.chain().focus().toggleBold().run() }} class="h-5 w-10 font-bold"
        >B</Button
      >
      <Button onclick={() => { if ($editor) $editor.chain().focus().toggleItalic().run() }} class="h-5 w-10 italic"
        >I</Button
      >
      <Button
        onclick={() => {
          showLink = true;
          if ($editor) $editor.chain().focus();
        }}
        class="link h-5 w-10"
        aria-label="Add Link"
        type="button">A</Button
      >
      <Button
        onclick={() => { if ($editor) $editor.chain().focus().toggleStrike().run() }}
        class="h-5 w-10 line-through">S</Button
      >
    </div>
  </BubbleMenu>
{/if}