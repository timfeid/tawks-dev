<script lang="ts">
  import { onMount } from "svelte";
  import {
    Bot,
    Check,
    Crown,
    Shield,
    ShoppingBag,
    Skull,
    Swords,
    Upload,
    UserRound,
    X,
    Lightbulb,
    Lock,
  } from "@lucide/svelte";
  import type { CharacterDef, Costume, JerseyDesign } from "../protocol";
  import { PUCKLAB_JERSEY } from "./pucklab-jersey";
  import CharacterPreview from "../character-preview.svelte";
  import { Button } from "@tawks/ui/button";
  import * as Tabs from "@tawks/ui/tabs";
  import { Slider } from "@tawks/ui/slider";
  import { BODY_MIN, BODY_MAX, bodyWidth } from "./body-build";
  import ModelImport from "../model-import.svelte";
  import { wardrobeClient, wardrobeSaved, type Wardrobe } from "./client";
  import { ROSTER_COLORS } from "../roster";
  import { MODEL_CATALOG } from "../models/catalog";
  import {
    characterSelection,
    CLOTHING_SLOTS,
    SINGLE_SLOTS,
    headChoices,
    seasonNote,
    hasUnownedSelection,
    toggleExtra,
  } from "./selection";
  import catalog from "./catalog.json";

  const isDark = (hex: string) => {
    const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
    return 0.299 * r + 0.587 * g + 0.114 * b < 96;
  };

  // Lives in the app's settings, not in a game: a saved look reaches every game showing through wardrobeSaved.
  let {
    roster = ROSTER_COLORS,
    guest = false,
    onsaved = () => {},
    onsuggest,
  }: {
    roster?: CharacterDef[];
    guest?: boolean;
    onsaved?: (wardrobe: Wardrobe) => void;
    onsuggest?: () => void;
  } = $props();
  let draft = $state<Costume>({ color: "sunny" });
  let baseLook = $state<Costume>({ color: "sunny" });
  let owned = $state<string[]>([]);
  /** The saved head stays on offer after its season, so the saved look can still be applied. */
  let savedHead = $state<string | null>(null);
  const heads = $derived(headChoices(savedHead));
  let clubJersey = $state<JerseyDesign | null>(null);
  let editorTab = $state("appearance");
  let slot = $state<"model" | (typeof CLOTHING_SLOTS)[number]>("model");
  let busy = $state(false),
    loading = $state(true),
    error = $state(""),
    saved = $state(false),
    imports = $state(false);
  const api = wardrobeClient();
  const slots = ["model", ...CLOTHING_SLOTS] as const;
  const selectedModel = $derived(
    MODEL_CATALOG.find((character) => character.id === draft.model_id),
  );
  const selected = $derived.by(() => {
    const color = roster.find((color) => color.id === draft.color) ?? roster[0];
    return color
      ? {
          ...color,
          name:
            selectedModel?.name ??
            (draft.model_id ? "Custom character" : color.name),
          costume:
            draft.shirt === PUCKLAB_JERSEY
              ? { ...draft, jersey: clubJersey }
              : draft,
        }
      : null;
  });
  const unowned = $derived(
    hasUnownedSelection(
      draft,
      owned,
      MODEL_CATALOG.map((character) => character.id),
    ),
  );
  // Included items show once owned; they are never sold here.
  const available = $derived(
    catalog.filter(
      (item) =>
        item.slot === slot &&
        (!("included_with" in item) || owned.includes(item.id)),
    ),
  );
  const build = $derived(bodyWidth(draft.body_width));
  function changeBuild(value: number) {
    draft = { ...draft, body_width: value };
    baseLook = { ...baseLook, body_width: value };
    saved = false;
    error = "";
  }
  const modelIcon = (id: string) =>
    (
      ({
        "model-robot": Bot,
        "model-ninja": Swords,
        "model-cowboy": UserRound,
        "model-goblin": Skull,
        "model-knight": Shield,
        "model-pirate": Crown,
      }) as Record<string, typeof UserRound>
    )[id] ?? UserRound;
  function accept(wardrobe: Wardrobe) {
    owned = wardrobe.owned;
    savedHead = wardrobe.set_up ? (wardrobe.costume.head ?? null) : null;
    clubJersey = wardrobe.jersey ?? null;
  }
  const wearing = (id: string | null) =>
    slot === "extra"
      ? id === null
        ? !(draft.extras ?? []).length
        : (draft.extras ?? []).includes(id)
      : (draft[slot as (typeof SINGLE_SLOTS)[number]] ?? null) === id;
  onMount(() => {
    if (!api || guest) {
      loading = false;
      return;
    }
    let stopped = false;
    void api
      .load()
      .then((wardrobe) => {
        if (!stopped) {
          accept(wardrobe);
          draft = { ...wardrobe.costume };
          if (!draft.model_id) baseLook = { ...draft };
        }
      })
      .catch((e) => {
        if (!stopped) error = e.message;
      })
      .finally(() => {
        if (!stopped) loading = false;
      });
    return () => {
      stopped = true;
    };
  });
  function choose(id: string | null) {
    if (slot === "model") {
      if (id) {
        if (!draft.model_id) baseLook = { ...draft };
        draft = characterSelection(draft, id);
      } else
        draft = {
          ...baseLook,
          color: draft.color,
          head: draft.head,
          bag: draft.bag,
          extras: draft.extras,
          body_width: draft.body_width,
          model_id: null,
        };
    } else if (slot === "bag") {
      // Carried, not worn, so choosing one keeps a whole character on.
      draft = { ...draft, bag: id };
      baseLook = { ...baseLook, bag: id };
    } else if (slot === "extra") {
      // Extras stack: each one toggles on or off, and None takes them all off.
      const worn = draft.extras ?? [];
      draft = {
        ...draft,
        extras:
          id === null
            ? []
            : toggleExtra(worn, id),
      };
      if (!draft.model_id) baseLook = { ...draft };
    } else {
      draft = {
        ...(draft.model_id ? baseLook : draft),
        color: draft.color,
        [slot]: id,
        model_id: null,
      };
      baseLook = { ...draft };
    }
    saved = false;
    error = "";
  }
  async function apply(costume = draft) {
    if (
      !api ||
      guest ||
      busy ||
      hasUnownedSelection(
        costume,
        owned,
        MODEL_CATALOG.map((character) => character.id),
      )
    )
      return;
    busy = true;
    error = "";
    try {
      const wardrobe = await api.apply(costume);
      accept(wardrobe);
      draft = wardrobe.costume;
      if (!draft.model_id) baseLook = { ...draft };
      wardrobeSaved(wardrobe);
      onsaved(wardrobe);
      saved = true;
    } catch (e) {
      error = (e as Error).message;
    } finally {
      busy = false;
    }
  }
  async function buy(id: string) {
    if (!api || guest || busy) return;
    busy = true;
    error = "";
    try {
      accept(await api.buy(id));
    } catch (e) {
      error = (e as Error).message;
    } finally {
      busy = false;
    }
  }
</script>

<div class="@container/avatar space-y-5">
  <div
    class="grid items-start gap-6 @min-[640px]/avatar:grid-cols-[14rem_minmax(0,1fr)]"
  >
    <div
      class="bg-card relative mx-auto aspect-[4/5] w-full max-w-60 overflow-hidden rounded-xl border @min-[640px]/avatar:sticky @min-[640px]/avatar:top-0"
    >
      <div class="absolute inset-x-0 top-0 bottom-16">
        <CharacterPreview character={selected} compact stage hoverReaction />
      </div>
      <div class="absolute inset-x-0 bottom-0 h-16 border-t px-4 py-3">
        <p class="text-sm font-medium">{selected?.name ?? "Your character"}</p>
        <p class="mt-1 text-xs text-neutral-400">
          Your look, everywhere you play.
        </p>
      </div>
    </div>
    <Tabs.Root bind:value={editorTab} class="min-w-0">
      <Tabs.List class="mb-5 w-full" aria-label="Avatar editor"
        ><Tabs.Trigger value="appearance" class="flex-1"
          >Appearance</Tabs.Trigger
        ><Tabs.Trigger value="wardrobe" class="flex-1">Wardrobe</Tabs.Trigger
        ></Tabs.List
      >
      <Tabs.Content value="appearance" class="space-y-6">
        <div class="flex flex-col gap-2">
          <span class="text-sm font-medium text-neutral-300"
            >{draft.model_id ? "Player colour" : "Skin colour"}</span
          >
          <div class="flex flex-wrap gap-2" aria-label="Skin colors">
            {#each roster as color (color.id)}
              <button
                class="grid size-7 place-items-center rounded-full transition {isDark(color.body)
                  ? 'text-white'
                  : 'text-black'} {draft.color === color.id
                  ? 'ring-2 ring-white ring-offset-2 ring-offset-zinc-900'
                  : 'hover:scale-110'}"
                style:background={color.body}
                title={color.name}
                aria-label={color.name}
                aria-pressed={draft.color === color.id}
                disabled={busy}
                onclick={() => {
                  draft = { ...draft, color: color.id };
                  saved = false;
                }}
              >
                {#if draft.color === color.id}<Check class="size-3.5" />{/if}
              </button>
            {/each}
          </div>
        </div>

        <div class="flex flex-col gap-2">
          <span class="text-sm font-medium text-neutral-300">Head</span>
          <div class="flex flex-wrap gap-2" aria-label="Heads">
            {#each heads as option (option.name)}
              {@const on = (draft.head ?? null) === option.id}
              <button
                class="rounded-full border px-3 py-1 text-sm transition-colors {on
                  ? 'border-primary bg-primary/10 text-white'
                  : 'border-white/10 text-neutral-400 hover:text-white'}"
                aria-pressed={on}
                title={seasonNote(option) || undefined}
                disabled={busy}
                onclick={() => {
                  draft = { ...draft, head: option.id };
                  baseLook = { ...baseLook, head: option.id };
                  saved = false;
                }}>{option.name}</button
              >
            {/each}
          </div>
          {#if heads.some((option) => option.until)}<p
              class="text-xs text-neutral-500"
            >
              Seasonal heads come and go. One you have on stays until you take it off.
            </p>{/if}
        </div>

        <div class="flex flex-col gap-3">
          <div class="flex items-center justify-between text-sm">
            <span class="font-medium text-neutral-300">Body build</span><span
              class="text-xs text-neutral-500"
              >{build < 0.9
                ? "Slim"
                : build > 1.15
                  ? "Round"
                  : "Balanced"}</span
            >
          </div>
          <Slider
            type="single"
            value={build}
            min={BODY_MIN}
            max={BODY_MAX}
            step={0.01}
            onValueChange={changeBuild}
            disabled={busy || loading || !!draft.model_id}
            thumbLabel="Body build"
          />
          <div class="flex justify-between text-xs text-neutral-500">
            <span>Slim</span><span>Round</span>
          </div>
          {#if draft.model_id}<p class="text-xs text-neutral-500">
              Choose Original to change body build.
            </p>{/if}
        </div>
      </Tabs.Content>
      <Tabs.Content value="wardrobe" class="@container/outfits space-y-4">
        <div class="flex items-center gap-3">
          <select
            class="bg-background border-input h-9 min-w-0 flex-1 rounded-md border px-3 text-sm"
            aria-label="Outfit category"
            bind:value={slot}
          >
            {#each slots as name}<option value={name}
                >{name === "model"
                  ? "Characters"
                  : name === "extra"
                    ? "Extras"
                    : name === "bag"
                      ? "Bags"
                      : name.charAt(0).toUpperCase() + name.slice(1)}</option
              >{/each}
          </select>
          <Button
            variant="ghost"
            size="sm"
            disabled={guest || !api || busy}
            onclick={() => (imports = true)}
            ><Upload class="size-4" />Your models</Button
          >
        </div>
        <div class="grid grid-cols-2 gap-2 @min-[460px]/outfits:grid-cols-3">
          {#if slot === "model"}
            {@render tile(
              !draft.model_id,
              "Original",
              "Free · fitted outfits",
              () => choose(null),
              UserRound,
            )}
            {#each MODEL_CATALOG as character (character.id)}{@render tile(
                draft.model_id === character.id,
                character.name,
                owned.includes(character.id)
                  ? "Owned"
                  : `${character.price} tickets`,
                () => choose(character.id),
                modelIcon(character.id),
                !owned.includes(character.id) && draft.model_id === character.id
                  ? { id: character.id, price: character.price }
                  : null,
              )}{/each}
            {#if draft.model_id && !selectedModel}{@render tile(
                true,
                "Your uploaded character",
                "From your model library",
                () => {},
                Upload,
              )}{/if}
          {:else}
            {@render tile(
              wearing(null) &&
                (slot === "bag" || slot === "extra" || !draft.model_id),
              "None",
              "",
              () => choose(null),
              X,
            )}
            {#each available as item (item.id)}{@render tile(
                wearing(item.id),
                item.name,
                "included_with" in item
                  ? "With PuckLab"
                  : owned.includes(item.id)
                    ? "Owned"
                    : "unlock" in item
                      ? `Achievement: ${item.unlock}`
                      : "10 tickets",
                () => choose(item.id),
                "unlock" in item && !owned.includes(item.id) ? Lock : null,
                owned.includes(item.id) || "unlocked_by" in item
                  ? null
                  : { id: item.id, price: 10 },
              )}{/each}
          {/if}
        </div>
        {#if draft.model_id}<p class="text-xs text-neutral-500">
            Extras and bags go with your character. Fitted clothing uses
            Original.
          </p>{/if}
      </Tabs.Content>
    </Tabs.Root>
  </div>
  <div
    class="bg-background sticky bottom-0 flex flex-wrap items-center justify-between gap-3 border-t py-4"
  >
    {#if onsuggest}<Button variant="ghost" size="sm" onclick={onsuggest}
        ><Lightbulb class="size-4" />Suggest items</Button
      >{:else}<span></span>{/if}
    <Button
      disabled={guest || !api || busy || loading || unowned}
      onclick={() => apply()}
      >{busy ? "Working…" : saved ? "Applied" : "Apply look"}</Button
    >
    {#if error}<p class="w-full text-xs text-red-400" role="alert">
        {error}
      </p>{/if}
    {#if unowned}<p class="w-full text-xs text-neutral-500">
        Preview only. Buy or earn this selection before applying it.
      </p>{/if}
  </div>
</div>

{#snippet tile(
  on: boolean,
  name: string,
  note: string,
  pick: () => void,
  Icon: typeof UserRound | null,
  sale: { id: string; price: number } | null = null,
)}
  <div
    class="flex flex-col overflow-hidden rounded-lg border transition-colors {on
      ? 'border-primary bg-primary/10'
      : 'border-white/10 bg-white/[0.03] hover:border-white/25'}"
  >
    <button
      class="flex min-h-16 flex-1 flex-col items-center justify-center gap-1 px-2 py-2.5 text-center"
      disabled={busy}
      aria-pressed={on}
      onclick={pick}
    >
      {#if Icon}<Icon
          class="size-5 {on ? 'text-primary' : 'text-neutral-300'}"
        />{/if}
      <span class="text-sm font-medium">{name}</span>
      {#if note}<span class="text-xs text-neutral-400">{note}</span>{/if}
    </button>
    {#if sale}
      <button
        class="flex items-center justify-center gap-1.5 border-t border-white/10 py-1.5 text-xs font-medium text-neutral-200 hover:bg-white/5 disabled:opacity-50"
        disabled={guest || !api || busy || loading}
        onclick={() => buy(sale.id)}
        aria-label="Buy {name} for {sale.price} tickets"
        ><ShoppingBag class="size-3" /> Buy · {sale.price}</button
      >
    {/if}
  </div>
{/snippet}
{#if imports && api}
  <div class="model-overlay">
    <ModelImport
      api={api.models}
      onclose={() => (imports = false)}
      onequip={async (model) => {
        imports = false;
        await apply(characterSelection(draft, model.id));
      }}
    />
  </div>
{/if}

<style>
  .model-overlay {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: grid;
    place-items: center;
    background: #020617dd;
    padding: 1rem;
    overflow: auto;
  }
  .model-overlay > :global(*) {
    width: min(54rem, 100%);
    max-height: 90dvh;
    overflow: auto;
  }
</style>
