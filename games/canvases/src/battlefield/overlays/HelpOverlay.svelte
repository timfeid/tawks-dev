<script lang="ts">
  import { CDN_BASE } from '../cdn';
  let { onClose }: { onClose: () => void } = $props();
  let activeTab = $state<string>('controls');
</script>

<div class="absolute inset-0 z-[60] flex items-center justify-center bg-black/70 backdrop-blur-sm" onclick={onClose}>
  <div
    class="relative h-[70vh] min-w-[700px] max-w-[700px] flex flex-col overflow-hidden rounded-2xl border border-cyan-500/30 bg-slate-950/95 shadow-[0_0_40px_rgba(6,182,212,0.15)]"
    onclick={(e) => e.stopPropagation()}
  >
    <!-- Header -->
    <div class="flex shrink-0 items-center justify-between border-b border-white/10 px-6 py-4">
      <div class="flex items-center gap-3">
        <svg class="h-5 w-5 text-cyan-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <h2 class="font-mono text-lg font-bold tracking-wider text-white">
          HOW TO PLAY
        </h2>
      </div>
      <button
        class="flex h-8 w-8 items-center justify-center rounded-lg border border-white/10 text-slate-400 transition hover:border-slate-300 hover:text-white"
        onclick={onClose}
      >
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    <!-- Tabs -->
    <div class="flex shrink-0 border-b border-white/10 px-4">
        {#each [
          { id: 'controls', label: 'Controls' },
          { id: 'phases', label: 'Phases' },
          { id: 'draft', label: 'Draft' },
          { id: 'cards', label: 'Cards' },
          { id: 'zones', label: 'Zones' },
          { id: 'collection', label: 'Collection' },
          { id: 'deckbuilding', label: 'Decks' },
        ] as tab}
          <button
            class="border-b-2 px-3 py-2.5 font-mono text-xs uppercase tracking-wider transition {activeTab === tab.id ? 'border-cyan-400 text-cyan-300' : 'border-transparent text-slate-500 hover:text-slate-300'}"
            onclick={() => activeTab = tab.id}
          >
            {tab.label}
          </button>
        {/each}
    </div>

    <!-- Content -->
    <div class="min-h-0 flex-1 overflow-y-auto p-6" style="scrollbar-gutter: stable;">

      {#if activeTab === 'controls'}
        <div class="space-y-5">
          <section>
            <h3 class="mb-2 font-mono text-xs uppercase tracking-widest text-cyan-400">Mouse</h3>
            <div class="space-y-1.5">
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">Click Card</span>
                <span class="pt-0.5 text-xs text-slate-400">Play, cast, attack, or activate abilities</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">Right-Click</span>
                <span class="pt-0.5 text-xs text-slate-400">Inspect card (detailed view)</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">Drag Card</span>
                <span class="pt-0.5 text-xs text-slate-400">Reorder battlefield, sacrifice, or discard</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">Hover Bottom</span>
                <span class="pt-0.5 text-xs text-slate-400">Open your hand tray</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">Click Table</span>
                <span class="pt-0.5 text-xs text-slate-400">Ping location (visible to all players)</span>
              </div>
            </div>
          </section>

          <section>
            <h3 class="mb-2 font-mono text-xs uppercase tracking-widest text-cyan-400">Keyboard</h3>
            <div class="space-y-1.5">
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">W A S D</span>
                <span class="pt-0.5 text-xs text-slate-400">Move camera (free camera mode)</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">Space / Shift</span>
                <span class="pt-0.5 text-xs text-slate-400">Camera up / down</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">V</span>
                <span class="pt-0.5 text-xs text-slate-400">Rotate view to next player</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">Escape</span>
                <span class="pt-0.5 text-xs text-slate-400">Close menus / exit fullscreen</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">Alt + Enter</span>
                <span class="pt-0.5 text-xs text-slate-400">Toggle fullscreen</span>
              </div>
            </div>
          </section>

          <section>
            <h3 class="mb-2 font-mono text-xs uppercase tracking-widest text-cyan-400">Interactions</h3>
            <div class="space-y-1.5">
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">Cast Spell</span>
                <span class="pt-0.5 text-xs text-slate-400">Click a playable card in your hand. If it needs a target, click the target next.</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">Attack</span>
                <span class="pt-0.5 text-xs text-slate-400">During combat, click a unit then click the target — a player, a unit, or an opposing Operator (drains its loyalty).</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">Block</span>
                <span class="pt-0.5 text-xs text-slate-400">Click your untapped unit, then click an attacker to block it.</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">Abilities</span>
                <span class="pt-0.5 text-xs text-slate-400">Click a card with abilities to see the action menu. Greyed abilities show why they can't activate.</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">Sacrifice</span>
                <span class="pt-0.5 text-xs text-slate-400">When required, drag a unit to the orange sacrifice zone.</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">Discard</span>
                <span class="pt-0.5 text-xs text-slate-400">When required, drag a card from your hand to the purple discard zone.</span>
              </div>
            </div>
          </section>
        </div>

      {:else if activeTab === 'phases'}
        <div class="space-y-4">
          <p class="text-xs text-slate-400">Each turn follows this sequence. Only the active player can advance phases.</p>
          <div class="space-y-3">
            <div class="flex items-start gap-3 rounded-lg border border-emerald-500/20 bg-emerald-500/5 p-3">
              <span class="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-emerald-500/20 font-mono text-[10px] font-bold text-emerald-300">1</span>
              <div>
                <p class="font-mono text-sm font-bold text-emerald-200">Main Phase</p>
                <p class="mt-0.5 text-xs text-slate-400">Play infrastructure (lands), cast units and spells, activate abilities. You draw a card at the start of your turn.</p>
              </div>
            </div>
            <div class="flex items-start gap-3 rounded-lg border border-red-500/20 bg-red-500/5 p-3">
              <span class="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-red-500/20 font-mono text-[10px] font-bold text-red-300">2</span>
              <div>
                <p class="font-mono text-sm font-bold text-red-200">Combat</p>
                <p class="mt-0.5 text-xs text-slate-400">Declare attackers by clicking units and selecting targets. Defending player can then declare blockers. Damage is dealt and units may die.</p>
              </div>
            </div>
            <div class="flex items-start gap-3 rounded-lg border border-red-500/20 bg-red-500/5 p-3">
              <span class="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-red-500/20 font-mono text-[10px] font-bold text-red-300">3</span>
              <div>
                <p class="font-mono text-sm font-bold text-red-200">Post-Combat Main</p>
                <p class="mt-0.5 text-xs text-slate-400">A second main phase. Play more cards and activate abilities before ending your turn.</p>
              </div>
            </div>
            <div class="flex items-start gap-3 rounded-lg border border-emerald-500/20 bg-emerald-500/5 p-3">
              <span class="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-emerald-500/20 font-mono text-[10px] font-bold text-emerald-300">4</span>
              <div>
                <p class="font-mono text-sm font-bold text-emerald-200">End Step</p>
                <p class="mt-0.5 text-xs text-slate-400">Cleanup phase. Discard down to hand limit if needed, then pass the turn to the next player.</p>
              </div>
            </div>
          </div>
          <div class="rounded-lg border border-amber-500/20 bg-amber-500/5 p-3">
            <p class="font-mono text-xs font-bold text-amber-300">The Stack</p>
            <p class="mt-1 text-xs text-slate-400">When spells are cast or abilities activate, they go on the stack. Both players can respond before it resolves. Click <strong class="text-amber-200">RESOLVE</strong> to resolve the top item.</p>
          </div>
        </div>

      {:else if activeTab === 'draft'}
        <div class="space-y-4">
          <div class="rounded-lg border border-cyan-500/20 bg-cyan-500/5 p-3">
            <p class="font-mono text-xs font-bold text-cyan-300">Hex Draft</p>
            <p class="mt-1 text-xs text-slate-400">Draft uses a 19-card hex pack instead of passing hands. Cards are placed randomly, so strong cards are not forced into the final pick.</p>
          </div>

          <section>
            <h3 class="mb-2 font-mono text-xs uppercase tracking-widest text-cyan-400">Pick Rules</h3>
            <div class="space-y-1.5">
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">Outer Ring</span>
                <span class="pt-0.5 text-xs text-slate-400">The outside cards are available first.</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">Unlock Inward</span>
                <span class="pt-0.5 text-xs text-slate-400">A card becomes pickable once all adjacent cards farther from the center are gone.</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">Grey Cards</span>
                <span class="pt-0.5 text-xs text-slate-400">Unavailable cards are dimmed. If a card is still in full color, it can be picked on your turn.</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">Turns</span>
                <span class="pt-0.5 text-xs text-slate-400">Players alternate picks until the whole hex pack is empty, then the next pack is dealt.</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 rounded bg-white/5 px-2 py-1 text-center font-mono text-[11px] text-slate-300">Last Two</span>
                <span class="pt-0.5 text-xs text-slate-400">If only two cards remain in the pack, both become pickable even if one would normally still be locked.</span>
              </div>
            </div>
          </section>

          <section>
            <h3 class="mb-2 font-mono text-xs uppercase tracking-widest text-cyan-400">Pack Counts</h3>
            <div class="grid grid-cols-2 gap-3">
              <div class="rounded-lg border border-white/10 bg-white/5 p-3">
                <p class="font-mono text-xs font-bold text-cyan-200">Normal Draft</p>
                <p class="mt-1 text-[11px] text-slate-400">5 draft packs, 19 cards each.</p>
              </div>
              <div class="rounded-lg border border-white/10 bg-white/5 p-3">
                <p class="font-mono text-xs font-bold text-cyan-200">Operator Draft</p>
                <p class="mt-1 text-[11px] text-slate-400">10 draft packs, 19 cards each.</p>
              </div>
            </div>
          </section>

          <section>
            <h3 class="mb-2 font-mono text-xs uppercase tracking-widest text-cyan-400">Pack Composition</h3>
            <div class="rounded-lg border border-amber-500/20 bg-amber-500/5 p-3">
              <p class="font-mono text-xs font-bold text-amber-300">Draft Pack</p>
              <p class="mt-1 text-xs text-slate-400">Draft uses a dedicated non-store pack with 19 slots and 2 guaranteed rare+ pulls. Store boosters stay separate.</p>
            </div>
          </section>
        </div>

      {:else if activeTab === 'cards'}
        <div class="space-y-4">
          <div class="grid grid-cols-2 gap-3">
            <div class="rounded-lg border border-white/10 bg-white/5 p-3">
              <p class="font-mono text-xs font-bold text-cyan-200">Unit</p>
              <p class="mt-1 text-[11px] text-slate-400">Has power/toughness. Can attack and block. Gets summoning sickness when first played (can't attack until next turn unless it has Haste).</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-white/5 p-3">
              <p class="font-mono text-xs font-bold text-cyan-200">Infrastructure</p>
              <p class="mt-1 text-[11px] text-slate-400">Produces mana when tapped. Displayed as hexagons on the table. One per turn (unless effects allow more).</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-white/5 p-3">
              <p class="font-mono text-xs font-bold text-cyan-200">Instant</p>
              <p class="mt-1 text-[11px] text-slate-400">Can be cast anytime, even during your opponent's turn. Resolves immediately via the stack.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-white/5 p-3">
              <p class="font-mono text-xs font-bold text-cyan-200">Ritual</p>
              <p class="mt-1 text-[11px] text-slate-400">Powerful one-time spells. Can only be cast during your main phase.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-white/5 p-3">
              <p class="font-mono text-xs font-bold text-cyan-200">Aura</p>
              <p class="mt-1 text-[11px] text-slate-400">Persistent effects that stay on the battlefield. Some attach to units, others affect the whole board.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-white/5 p-3">
              <p class="font-mono text-xs font-bold text-cyan-200">Relic</p>
              <p class="mt-1 text-[11px] text-slate-400">Equipment and utility items. Can be activated for effects or produce mana.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-white/5 p-3">
              <p class="font-mono text-xs font-bold text-cyan-200">Agent</p>
              <p class="mt-1 text-[11px] text-slate-400">Powerful allies with loyalty counters. Activate one ability per turn. Can be attacked directly by units.</p>
            </div>
          </div>

          <section>
            <h3 class="mb-2 font-mono text-xs uppercase tracking-widest text-cyan-400">Combat Keywords</h3>
            <div class="grid grid-cols-2 gap-2">
              <div class="flex gap-2">
                <span class="shrink-0 font-mono text-[11px] font-bold text-white">Haste</span>
                <span class="text-[11px] text-slate-400">Can attack immediately</span>
              </div>
              <div class="flex gap-2">
                <span class="shrink-0 font-mono text-[11px] font-bold text-white">Stealth</span>
                <span class="text-[11px] text-slate-400">Cannot be blocked</span>
              </div>
              <div class="flex gap-2">
                <span class="shrink-0 font-mono text-[11px] font-bold text-white">Aero</span>
                <span class="text-[11px] text-slate-400">Only blocked by Aero/Interceptor</span>
              </div>
              <div class="flex gap-2">
                <span class="shrink-0 font-mono text-[11px] font-bold text-white">Trample</span>
                <span class="text-[11px] text-slate-400">Excess damage hits the player</span>
              </div>
              <div class="flex gap-2">
                <span class="shrink-0 font-mono text-[11px] font-bold text-white">Deathtouch</span>
                <span class="text-[11px] text-slate-400">Any damage kills target</span>
              </div>
              <div class="flex gap-2">
                <span class="shrink-0 font-mono text-[11px] font-bold text-white">Interceptor</span>
                <span class="text-[11px] text-slate-400">Can block Aero units</span>
              </div>
              <div class="flex gap-2">
                <span class="shrink-0 font-mono text-[11px] font-bold text-white">Flash</span>
                <span class="text-[11px] text-slate-400">Can be cast at instant speed</span>
              </div>
              <div class="flex gap-2">
                <span class="shrink-0 font-mono text-[11px] font-bold text-white">Indestructible</span>
                <span class="text-[11px] text-slate-400">Cannot be destroyed</span>
              </div>
              <div class="flex gap-2">
                <span class="shrink-0 font-mono text-[11px] font-bold text-white">Piercing</span>
                <span class="text-[11px] text-slate-400">Ignores Armor and Shield</span>
              </div>
            </div>
          </section>

          <section>
            <h3 class="mb-2 font-mono text-xs uppercase tracking-widest text-cyan-400">Utility Keywords</h3>
            <div class="grid grid-cols-2 gap-2">
              <div class="flex gap-2">
                <span class="shrink-0 font-mono text-[11px] font-bold text-white">Glitch</span>
                <span class="text-[11px] text-slate-400">Abilities disabled</span>
              </div>
              <div class="flex gap-2">
                <span class="shrink-0 font-mono text-[11px] font-bold text-white">Signal Jam</span>
                <span class="text-[11px] text-slate-400">Disables opponent abilities</span>
              </div>
              <div class="flex gap-2">
                <span class="shrink-0 font-mono text-[11px] font-bold text-white">Gravecast</span>
                <span class="text-[11px] text-slate-400">Can be cast from graveyard</span>
              </div>
              <div class="flex gap-2">
                <span class="shrink-0 font-mono text-[11px] font-bold text-white">Firewall(X)</span>
                <span class="text-[11px] text-slate-400">Targeting costs X extra mana</span>
              </div>
              <div class="flex gap-2">
                <span class="shrink-0 font-mono text-[11px] font-bold text-white">Throttle(X)</span>
                <span class="text-[11px] text-slate-400">Opponent spells cost X more</span>
              </div>
              <div class="flex gap-2">
                <span class="shrink-0 font-mono text-[11px] font-bold text-white">Malware</span>
                <span class="text-[11px] text-slate-400">Damaged units lose abilities</span>
              </div>
              <div class="flex gap-2">
                <span class="shrink-0 font-mono text-[11px] font-bold text-white">Camouflage</span>
                <span class="text-[11px] text-slate-400">Cannot be targeted by opponent</span>
              </div>
              <div class="flex gap-2">
                <span class="shrink-0 font-mono text-[11px] font-bold text-white">Ethereal</span>
                <span class="text-[11px] text-slate-400">Cannot be targeted by physical damage</span>
              </div>
            </div>
          </section>
        </div>

      {:else if activeTab === 'zones'}
        <div class="space-y-5">
          <section>
            <h3 class="mb-2 font-mono text-xs uppercase tracking-widest text-cyan-400">Mana Colors</h3>
            <div class="grid grid-cols-3 gap-2">
              <div class="flex items-center gap-2 rounded bg-white/5 p-2">
                <span class="h-3 w-3 rounded-full" style="background: #00e5ff;"></span>
                <span class="font-mono text-xs text-slate-200">Neon</span>
              </div>
              <div class="flex items-center gap-2 rounded bg-white/5 p-2">
                <span class="h-3 w-3 rounded-full" style="background: #a855f7;"></span>
                <span class="font-mono text-xs text-slate-200">Glitch</span>
              </div>
              <div class="flex items-center gap-2 rounded bg-white/5 p-2">
                <span class="h-3 w-3 rounded-full" style="background: #cbd5e1;"></span>
                <span class="font-mono text-xs text-slate-200">Chrome</span>
              </div>
              <div class="flex items-center gap-2 rounded bg-white/5 p-2">
                <span class="h-3 w-3 rounded-full" style="background: #f97316;"></span>
                <span class="font-mono text-xs text-slate-200">Slag</span>
              </div>
              <div class="flex items-center gap-2 rounded bg-white/5 p-2">
                <span class="h-3 w-3 rounded-full" style="background: #22c55e;"></span>
                <span class="font-mono text-xs text-slate-200">Bio</span>
              </div>
              <div class="flex items-center gap-2 rounded bg-white/5 p-2">
                <span class="h-3 w-3 rounded-full" style="background: #64748b;"></span>
                <span class="font-mono text-xs text-slate-200">Colorless</span>
              </div>
            </div>
          </section>

          <section>
            <h3 class="mb-2 font-mono text-xs uppercase tracking-widest text-cyan-400">Drop Zones</h3>
            <div class="space-y-2">
              <div class="flex items-center gap-3 rounded-lg border border-orange-500/20 bg-orange-500/5 p-3">
                <span class="h-4 w-4 rounded-full" style="background: #f97316;"></span>
                <div>
                  <p class="font-mono text-xs font-bold text-orange-200">Sacrifice Zone</p>
                  <p class="mt-0.5 text-[11px] text-slate-400">Drag units here when an ability requires sacrifice. Rising fire particles.</p>
                </div>
              </div>
              <div class="flex items-center gap-3 rounded-lg border border-purple-500/20 bg-purple-500/5 p-3">
                <span class="h-4 w-4 rounded-full" style="background: #9333ea;"></span>
                <div>
                  <p class="font-mono text-xs font-bold text-purple-200">Discard Zone</p>
                  <p class="mt-0.5 text-[11px] text-slate-400">Drag hand cards here when you need to discard. Falling dark particles.</p>
                </div>
              </div>
            </div>
          </section>

          <section>
            <h3 class="mb-2 font-mono text-xs uppercase tracking-widest text-cyan-400">Card States</h3>
            <div class="space-y-1.5">
              <div class="flex items-start gap-3">
                <span class="w-32 shrink-0 font-mono text-[11px] font-bold text-white">Tapped</span>
                <span class="text-[11px] text-slate-400">Card tilted sideways. Cannot be used again until untapped next turn.</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-32 shrink-0 font-mono text-[11px] font-bold text-white">Summoning Sick</span>
                <span class="text-[11px] text-slate-400">White border. Unit just entered, cannot attack this turn.</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-32 shrink-0 font-mono text-[11px] font-bold text-white">Burnout</span>
                <span class="text-[11px] text-slate-400">Card is overloaded and will be destroyed at end of turn.</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-32 shrink-0 font-mono text-[11px] font-bold text-white">Playable</span>
                <span class="text-[11px] text-slate-400">Cards in hand are split: playable on left, non-playable greyed on right.</span>
              </div>
            </div>
          </section>
        </div>

      {:else if activeTab === 'collection'}
        <div class="space-y-5">
          <section>
            <h3 class="mb-2 font-mono text-xs uppercase tracking-widest text-cyan-400">Rarity Tiers</h3>
            <div class="grid grid-cols-5 gap-2">
              {#each [
                { name: 'Public', color: '#94a3b8', img: `${CDN_BASE}/rarities/public.png` },
                { name: 'Private', color: '#38bdf8', img: `${CDN_BASE}/rarities/private.png` },
                { name: 'Restricted', color: '#a855f7', img: `${CDN_BASE}/rarities/restricted.png` },
                { name: 'Classified', color: '#f43f5e', img: `${CDN_BASE}/rarities/classified.png` },
                { name: 'Singularity', color: '#facc15', img: `${CDN_BASE}/rarities/singularity.png` },
              ] as rarity}
                <div class="flex flex-col items-center gap-1.5 rounded-lg border p-2.5" style="border-color: {rarity.color}30; background: {rarity.color}08">
                  <img src={rarity.img} alt={rarity.name} class="h-8 w-8 brightness-0 invert opacity-80" />
                  <span class="font-mono text-[10px] font-bold" style="color: {rarity.color}">{rarity.name}</span>
                </div>
              {/each}
            </div>
            <p class="mt-2 text-[11px] text-slate-500">Higher rarity means more powerful cards. Singularity cards are unique.</p>
          </section>

          <section>
            <h3 class="mb-2 font-mono text-xs uppercase tracking-widest text-cyan-400">Card Properties</h3>
            <div class="space-y-2">
              <div class="rounded-lg border border-white/10 bg-white/5 p-3">
                <p class="font-mono text-xs font-bold text-cyan-200">Holo</p>
                <p class="mt-1 text-[11px] text-slate-400">1% chance on any card. Holographic glitch effect with chromatic split and digital artifacts. Highly collectible.</p>
              </div>
              <div class="rounded-lg border border-white/10 bg-white/5 p-3">
                <p class="font-mono text-xs font-bold text-cyan-200">Wear</p>
                <p class="mt-1 text-[11px] text-slate-400">Digital degradation level. Randomly assigned when a card is opened. Affects rust and stain overlays. Purely cosmetic.</p>
              </div>
              <div class="rounded-lg border border-white/10 bg-white/5 p-3">
                <p class="font-mono text-xs font-bold text-cyan-200">Seed</p>
                <p class="mt-1 text-[11px] text-slate-400">Unique procedural value per card instance. Determines scan line patterns, border light colors, and visual effects. Every copy is different.</p>
              </div>
            </div>
          </section>

          <section>
            <h3 class="mb-2 font-mono text-xs uppercase tracking-widest text-cyan-400">Pack Opening</h3>
            <p class="text-xs text-slate-400">Packs contain multiple card slots, each with weighted rarity chances. Open packs to receive cards with randomized rarity, wear, and a chance at holo. Cards are added to your collection permanently.</p>
          </section>
        </div>

      {:else if activeTab === 'deckbuilding'}
        <div class="space-y-5">
          <section>
            <h3 class="mb-2 font-mono text-xs uppercase tracking-widest text-cyan-400">Deck Formats</h3>
            <div class="space-y-3">
              <div class="rounded-lg border border-white/10 bg-white/5 p-3">
                <p class="font-mono text-xs font-bold text-cyan-200">Classic (34 cards)</p>
                <p class="mt-1 text-[11px] text-slate-400">Standard format. Up to 3 copies of any card. Build around your mana colors and strategy.</p>
              </div>
              <div class="rounded-lg border border-white/10 bg-white/5 p-3">
                <p class="font-mono text-xs font-bold text-cyan-200">Operator (66 cards)</p>
                <p class="mt-1 text-[11px] text-slate-400">Choose an apex unit as your Operator. Singleton format (1 copy per card). Deck colors must match your Operator's color identity.</p>
              </div>
            </div>
          </section>

          <section>
            <h3 class="mb-2 font-mono text-xs uppercase tracking-widest text-cyan-400">Operator Rules</h3>
            <div class="space-y-1.5">
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 font-mono text-[11px] font-bold text-white">Operator Zone</span>
                <span class="text-[11px] text-slate-400">Your Operator starts here. Cast it anytime you could cast a unit.</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 font-mono text-[11px] font-bold text-white">Operator Tax</span>
                <span class="text-[11px] text-slate-400">Each recast costs 2 more colorless mana than the last time.</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 font-mono text-[11px] font-bold text-white">Operator Damage</span>
                <span class="text-[11px] text-slate-400">21 combat damage from a single opposing Operator eliminates you — even at full life. Track it in the opponent panel.</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 font-mono text-[11px] font-bold text-white">Loyalty Drain</span>
                <span class="text-[11px] text-slate-400">Operators (Agent type) can be attacked. Combat damage drains loyalty 1-for-1; at 0 the Operator is destroyed and returns to the Operator Zone.</span>
              </div>
              <div class="flex items-start gap-3">
                <span class="w-36 shrink-0 font-mono text-[11px] font-bold text-white">Death / Exile</span>
                <span class="text-[11px] text-slate-400">When your Operator dies or is exiled, it returns to the Operator Zone instead.</span>
              </div>
            </div>
          </section>

          <section>
            <h3 class="mb-2 font-mono text-xs uppercase tracking-widest text-cyan-400">Card Browser</h3>
            <p class="text-xs text-slate-400">Browse your full collection with sorting and filtering. Sort by name, mana cost, rarity, color, or date obtained. Filter by card type or ownership. Right-click any card to inspect it.</p>
          </section>
        </div>
      {/if}
    </div>
  </div>
</div>
