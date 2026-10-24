<script lang="ts">
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { workflow, display, inputs, ...rest }: Props = $props();
  let divDisplay = $derived(
    typeof display.type === 'string' || !('Div' in display.type)
      ? null
      : display.type.Div,
  );
  let stuckAware = $derived(/(^|\s)stuck-aware(\s|$)/.test(display.class ?? ''));

  // Toggles a `stuck` class on a sticky element while it is pinned to the top
  // of its scroll container, so bot styles can react via `[&.stuck]:...`.
  function trackStuck(el: HTMLElement) {
    let container: HTMLElement | null = el.parentElement;
    while (container) {
      const overflowY = getComputedStyle(container).overflowY;
      if (overflowY === 'auto' || overflowY === 'scroll') break;
      container = container.parentElement;
    }
    const target: HTMLElement | Window = container ?? window;
    const update = () => {
      const top = parseFloat(getComputedStyle(el).top) || 0;
      const containerTop = container ? container.getBoundingClientRect().top + (parseFloat(getComputedStyle(container).paddingTop) || 0) : 0;
      const scrolled = container ? container.scrollTop : window.scrollY;
      const stuck = scrolled > 0 && el.getBoundingClientRect().top <= containerTop + top + 1;
      el.classList.toggle('stuck', stuck);
    };
    target.addEventListener('scroll', update, { passive: true });
    update();
    return {
      destroy() {
        target.removeEventListener('scroll', update);
      },
    };
  }
</script>

{#if divDisplay}
  {#if stuckAware}
    <div class={display.class} use:trackStuck>
      {#each divDisplay.content as displayItem}
        <WorkflowDisplay {...rest} {inputs} {workflow} display={displayItem} />
      {/each}
    </div>
  {:else}
    <div class={display.class}>
      {#each divDisplay.content as displayItem}
        <WorkflowDisplay {...rest} {inputs} {workflow} display={displayItem} />
      {/each}
    </div>
  {/if}
{/if}
