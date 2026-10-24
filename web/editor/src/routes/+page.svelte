<script lang="ts">
  import { Editor, EditorContent, FloatingMenu, createEditor } from '$lib';
  import BubbleMenu from '$lib/BubbleMenu.svelte';
  import { onMount } from 'svelte';
  import Button from '../../../../apps/tawks/src/lib/components/ui/button/button.svelte';
  // import { SvelteEditableExtension } from './_components/SvelteExtension';

  function listener() {
    document.removeEventListener('click', listener);
    open = false;
  }

  let editor = $state<Editor>();

  onMount(() => {
    editor = createEditor({
      editable: true,
      content: `
        <p>suh duu</p>
      `,
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

  $effect(() => {
    if (editor) {
      console.log(JSON.stringify(editor.getJSON()));
    }
  });
</script>

<svelte:head>
  <title>Floating Menu | Tiptap Svelte</title>
</svelte:head>

<div class="mt-12"></div>

{#if editor}
  <BubbleMenu {editor}>
    <div class="bg-gray-950">
      <Button class="h-5 w-10">B</Button>
      <Button class="h-5 w-10">I</Button>
    </div>
  </BubbleMenu>
  <FloatingMenu {editor}>
    <div data-test-id="floating-menu" class="absolute inline-block text-left">
      <button
        onclick={openMenu}
        class="transform -translate-x-full -translate-y-2/4 absolute -left-16 -top-5 w-9.5 h-9.5"
      >
        <div class="absolute bg-primary text-white rounded-full w-9.5 h-9.5 p-2 px-4 top-0 lg:hover:bg-primary-300">
          +
        </div>
      </button>
      {#if open}
        <div
          class="origin-top-right absolute -left-3 -top-6 mt-2 w-80 rounded-md shadow-lg bg-gray-900 ring-1 ring-black ring-opacity-5 focus:outline-none"
        >
          <div class="bg-gray-900 rounded-md">
            <div
              class="origin-top-right absolute -left-3 -top-6 mt-2 w-80 rounded-md shadow-lg bg-gray-900 ring-1 ring-black ring-opacity-5 focus:outline-none"
            >
              <div class="py-1 bg-gray-900 rounded-md">
                <button
                  type="button"
                  class="w-full flex items-center p-4 text-sm relative border-b border-gray-800 lg:hover:bg-gray-800"
                  onclick={(event) => console.log(event)}
                >
                  <span class="h-10 w-10 rounded-lg p-2 overflow-hidden"> i </span>
                  <div class="ml-3 flex flex-col justify-between text-left">
                    <p class="leading-5 font-medium text-gray-50">Image</p>
                    <p class="leading-5 font-normal text-gray-50">Add an image</p>
                  </div>
                </button>
                <button
                  type="button"
                  class="w-full flex items-center p-4 text-sm relative border-b border-gray-800 lg:hover:bg-gray-800"
                  onclick={(event) => insertContent({ type: 'blockquote', content: [{ type: 'paragraph' }] })}
                >
                  <span class="h-10 w-10 rounded-lg p-2 overflow-hidden"> i </span>
                  <div class="ml-3 flex flex-col justify-between text-left">
                    <p class="leading-5 font-medium text-gray-50">Quote</p>
                    <p class="leading-5 font-normal text-gray-50">Create a quotation</p>
                  </div>
                </button>
                <button
                  type="button"
                  class="w-full flex items-center p-4 text-sm relative border-b border-gray-800 lg:hover:bg-gray-800"
                  onclick={(event) =>
                    insertContent({
                      type: 'bulletList',
                      content: [{ type: 'listItem', content: [{ type: 'paragraph' }] }],
                    })}
                >
                  <span class="h-10 w-10 rounded-lg p-2 overflow-hidden"> i </span>
                  <div class="ml-3 flex flex-col justify-between text-left">
                    <p class="leading-5 font-medium text-gray-50">Generic List</p>
                    <p class="leading-5 font-normal text-gray-50">A simple list with bullets</p>
                  </div>
                </button>
                <button
                  type="button"
                  class="w-full flex items-center p-4 text-sm relative border-b border-gray-800 lg:hover:bg-gray-800"
                  onclick={(event) =>
                    insertContent({
                      type: 'orderedList',
                      content: [{ type: 'listItem', content: [{ type: 'paragraph' }] }],
                    })}
                >
                  <span class="h-10 w-10 rounded-lg p-2 overflow-hidden"> i </span>
                  <div class="ml-3 flex flex-col justify-between text-left">
                    <p class="leading-5 font-medium text-gray-50">Numbered List</p>
                    <p class="leading-5 font-normal text-gray-50">An ordered list with numbering</p>
                  </div>
                </button>
              </div>
            </div>
          </div>
        </div>
      {/if}
    </div>
  </FloatingMenu>
{/if}

{#if editor}
  <EditorContent {editor} />
{/if}
