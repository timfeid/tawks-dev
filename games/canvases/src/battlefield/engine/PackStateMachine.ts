import * as THREE from 'three';

export type PackPhase =
  | 'inventory'
  | 'grab'
  | 'tear'
  | 'opening'
  | 'burst'
  | 'reveal'
  | 'summary';

export interface PackStateUpdate {
  packPhase?: PackPhase;
  packOpeningOpeningId?: string | null;
  packOpeningTypeId?: string | null;
  packTearTypeId?: string | null;
  packTearProgress?: number;
  packOpeningShaking?: boolean;
  openedPackResult?: any;
  revealedPackIndices?: Set<number>;
  packRevealZoomed?: boolean;
  packBurstStartedAt?: number;
  packFlyInCount?: number;
  packFocusIndex?: number;
  flashRarity?: string | null;
}

export interface PackDeps {
  onSetState: (updates: PackStateUpdate) => void;
  submitAction: (action: string, payload: any) => void;
  addPackBurst: (x: number, y: number, z: number) => void;
}

const PACK_TEAR_AUTOFINISH_MS = 280;
const PACK_BURST_MIN_MS = 1400;
const PACK_RISE_MS = 1200;

export class PackStateMachine {
  private flyInTimers: number[] = [];
  private autoFinishRaf: number | null = null;
  private revealTimeout: number | null = null;

  constructor(private deps: PackDeps) {}

  public clearTimers(): void {
    if (this.autoFinishRaf != null) {
      cancelAnimationFrame(this.autoFinishRaf);
      this.autoFinishRaf = null;
    }
    if (this.revealTimeout != null) {
      clearTimeout(this.revealTimeout);
      this.revealTimeout = null;
    }
    this.flyInTimers.forEach((t) => clearTimeout(t));
    this.flyInTimers = [];
  }

  public tearStart(typeId: string): void {
    this.deps.onSetState({
      packTearTypeId: typeId,
      packTearProgress: 0,
      packPhase: 'grab',
    });
  }

  public tearProgress(progress: number, currentPhase: PackPhase): void {
    const updates: PackStateUpdate = { packTearProgress: progress };
    if (currentPhase === 'grab' || currentPhase === 'inventory') {
      updates.packPhase = 'tear';
    }
    this.deps.onSetState(updates);
  }

  public tearCancel(): void {
    this.deps.onSetState({
      packTearProgress: 0,
      packTearTypeId: null,
      packPhase: 'inventory',
    });
  }

  public commitOpen(
    typeId: string,
    packsByType: Map<string, { packType: any; packs: any[] }>,
    currentTearProgress: number,
  ): void {
    const entry = packsByType.get(typeId);
    const nextPack = entry?.packs?.[0];
    if (!nextPack) {
      this.deps.onSetState({ packPhase: 'inventory', packTearTypeId: null, packTearProgress: 0 });
      return;
    }

    this.clearTimers();
    const from = Math.max(0, Math.min(1, currentTearProgress));
    this.deps.onSetState({ packTearTypeId: typeId });

    this.deps.submitAction('games.cardgame.open_pack', {
      pack_instance_id: nextPack.pack_instance_id,
    });

    if (from >= 0.95) {
      this.deps.onSetState({
        packTearProgress: 1,
        packPhase: 'opening',
        packOpeningOpeningId: nextPack.pack_instance_id,
        packOpeningTypeId: typeId,
      });
      setTimeout(() => {
        this.enterBurst(typeId, nextPack.pack_instance_id);
      }, PACK_RISE_MS);
      return;
    }

    this.deps.onSetState({ packPhase: 'tear' });
    const start = performance.now();
    const tick = (now: number) => {
      const t = Math.max(0, Math.min(1, (now - start) / PACK_TEAR_AUTOFINISH_MS));
      const eased = 1 - Math.pow(1 - t, 3);
      this.deps.onSetState({
        packTearProgress: THREE.MathUtils.lerp(from, 1, eased),
      });
      if (t < 1) {
        this.autoFinishRaf = requestAnimationFrame(tick);
        return;
      }
      this.autoFinishRaf = null;
      this.deps.onSetState({
        packTearProgress: 1,
        packPhase: 'opening',
        packOpeningOpeningId: nextPack.pack_instance_id,
        packOpeningTypeId: typeId,
      });
      setTimeout(() => {
        this.enterBurst(typeId, nextPack.pack_instance_id);
      }, PACK_RISE_MS);
    };
    this.autoFinishRaf = requestAnimationFrame(tick);
  }

  private enterBurst(typeId: string, packInstanceId: string): void {
    this.deps.onSetState({
      packPhase: 'burst',
      packOpeningShaking: false,
      packOpeningOpeningId: packInstanceId,
      packOpeningTypeId: typeId,
      packTearTypeId: null,
      packTearProgress: 1,
      openedPackResult: null,
      revealedPackIndices: new Set(),
      packRevealZoomed: false,
      packBurstStartedAt: performance.now(),
    });
    this.deps.addPackBurst(0, 10.0, 0);
  }

  public onPackResultReceived(
    packOpenResult: any,
    currentPhase: PackPhase,
    burstStartedAt: number,
    currentOpenedResult: any,
  ): void {
    if (!packOpenResult || currentPhase !== 'burst' || currentOpenedResult) return;

    this.clearTimers();
    const elapsed = performance.now() - burstStartedAt;
    const wait = Math.max(0, PACK_BURST_MIN_MS - elapsed);

    this.revealTimeout = window.setTimeout(() => {
      this.revealTimeout = null;
      const cardCount = packOpenResult?.cards?.length || 0;
      this.deps.onSetState({
        packPhase: 'reveal',
        packFlyInCount: 0,
        packFocusIndex: 0,
        openedPackResult: packOpenResult,
        revealedPackIndices: new Set(),
      });
      for (let i = 0; i < cardCount; i++) {
        const t = window.setTimeout(() => {
          this.deps.onSetState({ packFlyInCount: i + 1 });
        }, i * 250);
        this.flyInTimers.push(t);
      }
    }, wait);
  }

  public revealCard(
    index: number,
    currentPhase: PackPhase,
    revealedIndices: Set<number>,
    packResult: any,
  ): void {
    if (currentPhase !== 'reveal') return;

    if (revealedIndices.has(index)) {
      this.deps.onSetState({ packRevealZoomed: false });
      return;
    }

    const next = new Set(revealedIndices);
    next.add(index);
    this.deps.onSetState({ packRevealZoomed: true, revealedPackIndices: next });

    const card = packResult?.cards[index];
    if (card) {
      const flashRarities = ['classified', 'singularity'];
      if (flashRarities.includes(card.rarity)) {
        this.deps.onSetState({ flashRarity: card.rarity });
        setTimeout(() => {
          this.deps.onSetState({ flashRarity: null });
        }, 400);
      }
    }

    if (next.size === (packResult?.cards?.length || 0)) {
      setTimeout(() => {
        this.deps.onSetState({ packPhase: 'summary' });
      }, 1000);
    }
  }

  public revealAll(packResult: any): void {
    if (!packResult) return;
    const next = new Set<number>();
    for (let i = 0; i < packResult.cards.length; i++) next.add(i);
    this.deps.onSetState({ revealedPackIndices: next, packPhase: 'summary' });
  }

  public openAnother(): void {
    this.clearTimers();
    this.deps.onSetState({
      openedPackResult: null,
      revealedPackIndices: new Set(),
      packRevealZoomed: false,
      packPhase: 'inventory',
      packFlyInCount: 0,
      packFocusIndex: 0,
      packOpeningOpeningId: null,
      packOpeningTypeId: null,
      packTearTypeId: null,
      packTearProgress: 0,
    });
    this.deps.submitAction('games.cardgame.get_user_packs', {});
  }

  public close(): void {
    this.clearTimers();
    this.deps.onSetState({
      packPhase: 'inventory',
      packFlyInCount: 0,
      packFocusIndex: 0,
      packTearTypeId: null,
      packTearProgress: 0,
      packOpeningOpeningId: null,
      packOpeningTypeId: null,
    });
  }

  public toggle(
    currentShowPacks: boolean,
    submitAction: (action: string, payload: any) => void,
  ): boolean {
    if (!currentShowPacks) {
      submitAction('games.cardgame.list_pack_types', {});
      submitAction('games.cardgame.get_user_packs', {});
      return true;
    }
    this.close();
    return false;
  }

  public dispose(): void {
    this.clearTimers();
  }
}
