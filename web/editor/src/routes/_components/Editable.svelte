<script lang="ts">
  import type { NodeViewProps } from '@tiptap/core';
  import cx from 'classnames';
  import { draggable, editable, NodeViewWrapper } from '$lib';

  export let node: NodeViewProps['node'];
  export let updateAttributes: NodeViewProps['updateAttributes'];
  export let selected: NodeViewProps['selected'] = false;

  let prettyAnalysts = [];

  let options = ['test', 'arino'];

  let val = '';
  let searchOpen = false;

  $: {
    prettyAnalysts = JSON.parse(atob(node.attrs.analysts));
    searchOpen = val !== '';
  }
</script>

<NodeViewWrapper
  id="svelte-component"
  class="bg-yellow-200/90 border-black border-2 px-4 pb-4 my-4 rounded-md flex flex-col"
>
  <span class="bg-black py-1 px-3 text-white rounded-b-md uppercase text-[0.6rem] font-bold w-max">Analysts</span>

  {JSON.stringify(prettyAnalysts)}

  <div class="relative">
    <input bind:value={val} class="mt-4 border-dashed border-gray-900/20 border-2 p-2 rounded-md w-full" />
    {#if searchOpen}
      <div class="absolute z-50 top-100 left-0 bg-white border rounded-b-md right-0">
        {#each options as option}
          <div>{option}</div>
        {/each}
      </div>
    {/if}
  </div>
</NodeViewWrapper>
