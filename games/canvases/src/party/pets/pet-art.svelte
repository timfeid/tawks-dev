<script lang="ts">
  import type { Pet } from './catalog';

  let { pet, class: className = 'size-24' }: { pet: Pet; class?: string } = $props();

  const CRESTS: Record<string, string> = {
    'pet-ember': 'M50 12 C58 24 64 30 58 40 C56 34 52 34 50 38 C48 32 42 32 42 40 C36 30 44 22 50 12Z',
    'pet-puddle': 'M50 14 C56 24 60 30 60 35 A10 10 0 0 1 40 35 C40 30 44 24 50 14Z',
    'pet-sprout': 'M50 38 C40 30 40 18 52 12 C60 20 58 32 50 38Z',
    'pet-fizz': 'M54 10 L42 28 L50 28 L45 40 L60 22 L52 22Z',
    'pet-mochi': 'M56 12 A12 12 0 1 0 64 34 A10 10 0 1 1 56 12Z',
  };
</script>

<svg class="shrink-0 {className}" viewBox="0 0 100 100" role="img" aria-label={pet.name}>
  <path d={CRESTS[pet.id]} fill={pet.accent} stroke="rgba(0,0,0,0.25)" stroke-width="2" stroke-linejoin="round" />
  <ellipse cx="38" cy="88" rx="8" ry="4" fill={pet.body} stroke="rgba(0,0,0,0.25)" stroke-width="2" />
  <ellipse cx="62" cy="88" rx="8" ry="4" fill={pet.body} stroke="rgba(0,0,0,0.25)" stroke-width="2" />
  <ellipse cx="50" cy="62" rx="31" ry="27" fill={pet.body} stroke="rgba(0,0,0,0.25)" stroke-width="2" />
  <ellipse cx="50" cy="71" rx="17" ry="13" fill={pet.accent} opacity="0.85" />
  <circle cx="40" cy="56" r="4.5" fill="#1b1b2b" />
  <circle cx="60" cy="56" r="4.5" fill="#1b1b2b" />
  <circle cx="41.5" cy="54.5" r="1.5" fill="#fff" />
  <circle cx="61.5" cy="54.5" r="1.5" fill="#fff" />
  <path d="M45 65 Q50 69 55 65" fill="none" stroke="#1b1b2b" stroke-width="2.2" stroke-linecap="round" />
</svg>
