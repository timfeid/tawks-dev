<script lang="ts">
  import { THEMES, type ChartConfig } from './chart-utils.js';

  let { id, config }: { id: string; config: ChartConfig } = $props();

  const colorConfig = $derived(
    config ? Object.entries(config).filter(([, config]) => config.theme || config.color) : null,
  );

  const themeContents = $derived.by(() => {
    if (!colorConfig || !colorConfig.length) return;

    const themeContents = [];
    for (let [_theme, prefix] of Object.entries(THEMES)) {
      let content = `${prefix} [data-chart=${id}] {\n`;
      let finalColor = '';
      const color = colorConfig.map(([key, itemConfig]) => {
        const theme = _theme as keyof typeof itemConfig.theme;
        const color = itemConfig.theme?.[theme] || itemConfig.color;
        if (color) {
          finalColor = color;
        }
        return color ? `\t--color-${key}: ${color};` : null;
      });

      content += color.join('\n') + '\n}';
      content += `[data-chart=${id}] .lc-line-chart-line { stroke: ${finalColor}; }`;

      themeContents.push(content);
    }

    return themeContents.join('\n');
  });
</script>

{#if themeContents}
  {#key id}
    <svelte:element this={'style'}>
      {themeContents}
    </svelte:element>
  {/key}
{/if}
