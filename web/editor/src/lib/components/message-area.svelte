<script lang="ts">
  import { configure } from '@testing-library/svelte';
  import PlaceholderExtension from '@tiptap/extension-placeholder';
  import { Editor, EditorContent, FloatingMenu, createEditor } from '$lib';
  import BubbleMenu from '$lib/BubbleMenu.svelte';
  import { onMount } from 'svelte';
  import Button from '../../../../../apps/tawks/src/lib/components/ui/button/button.svelte';
  // import { SvelteEditableExtension } from './_components/SvelteExtension';

  function listener() {
    document.removeEventListener('click', listener);
    open = false;
  }

  let editor = $state<Editor>();
  let { channel }: { channel: string } = $props();

  onMount(() => {
    editor = createEditor({
      extensions: [PlaceholderExtension.configure({ placeholder: `Message ${channel}` })],
      editable: true,
      content: '',
      editorProps: {
        attributes: {
          class: 'outline-none ',
        },
      },
    });
  });

  const toggleHeading = (level: any) => {
    return () => {
      editor && editor.chain().focus().toggleHeading({ level }).run();
    };
  };

  const toggleBold = () => {
    editor && editor.chain().focus().toggleBold().run();
  };

  const toggleItalic = () => {
    editor && editor.chain().focus().toggleItalic().run();
  };

  let open = $state(false);

  type ItemActionConfig = {
    content?: any;
    attrs?: any;
    type: string;
  };

  function insertContent(payload: ItemActionConfig) {
    const { content, attrs, type } = payload;

    editor &&
      editor
        .chain()
        .insertContentAt(editor.state.selection.$anchor.pos, {
          type,
          attrs: attrs || {},
          content,
        })
        .focus()
        .run();

    open = false;
  }

  const article = {
    id: 0,
    slug: '',
    image: '',
    // responsiveImage: new ResponsiveImage(),
    summary: '',
    publishedAt: null,
    // contentType: new ArticleContentType(),
    // place: new ArticlePlace(),
    analysts: [],
    // channel: new Channel(),
    uuid: '',
    title: '',
    regionIds: [],
    bullets: [],
    companies: [],
    showVisitButton: false,
    externalUrl: '',
    tags: [],
    entities: [],
    isProExclusive: false,
  };

  // let isActive = editor.isActive(name, attrs);

  async function openMenu() {
    open = true;
    setTimeout(() => {
      document.addEventListener('click', listener);
    }, 100);
  }
</script>

{#if editor}
  <BubbleMenu {editor}>
    <div class="bg-gray-950">
      <Button onclick={toggleBold} class="h-5 w-10">B</Button>
      <Button onclick={toggleItalic} class="h-5 w-10">I</Button>
    </div>
  </BubbleMenu>
  <EditorContent {editor} />
{/if}

<style>
  :global(.ProseMirror p.is-editor-empty:first-child::before) {
    content: attr(data-placeholder);
    float: left;
    color: #adb5bd;
    pointer-events: none;
    height: 0;
  }
</style>
