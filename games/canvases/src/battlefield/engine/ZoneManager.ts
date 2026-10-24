import * as THREE from 'three';
import type { Zone } from '../zones/Zone';
import { CardActor } from '../entities/CardActor';
import type { CardView } from '../types';
import { CardFactory } from '../entities/CardFactory';

export class ZoneManager {
  private zones = new Map<string, Zone>();
  private actors = new Map<string, CardActor>();
  private factory = new CardFactory();
  // Actors that are being animated out before removal.
  private dyingActors: { actor: CardActor; removeAt: number }[] = [];

  constructor(private scene: THREE.Scene) {}

  public register(zone: Zone) {
    this.zones.set(zone.id, zone);
    const objs = zone.getSceneObjects?.();
    if (objs) {
        objs.forEach(o => this.scene.add(o));
    }
  }

  public get(id: string) {
    return this.zones.get(id);
  }

  public syncActors(
    allCards: CardView[],
    getSpawnPosition?: (card: CardView) => THREE.Vector3 | null,
  ) {
    const validIds = new Set(allCards.map(c => c.id));

    // Remove stale actors — animate them out instead of popping.
    for (const [id, actor] of this.actors) {
      if (!validIds.has(id)) {
        // Send the card downward so it visually "falls" off the table.
        const exitPos = actor.group.position.clone();
        exitPos.y -= 3.0;
        actor.setTarget(
          exitPos,
          actor.group.rotation.clone(),
          actor.group.scale.x * 0.3,
          0.08,
        );
        this.dyingActors.push({
          actor,
          removeAt: performance.now() + 500,
        });
        this.actors.delete(id);
      }
    }

    // Create/Update actors
    allCards.forEach(card => {
      let actor = this.actors.get(card.id);
      if (!actor) {
        actor = new CardActor(card, this.factory);
        // Optionally seed the new actor's position so it flies in from
        // a meaningful place (e.g. the caster's seat) instead of (0,0,0).
        // Must also disable the default snap-to-target so it actually
        // lerps from the seeded position to its layout spot.
        if (getSpawnPosition) {
          const spawn = getSpawnPosition(card);
          if (spawn) {
            actor.group.position.copy(spawn);
            actor.disableSnap();
          }
        }
        this.scene.add(actor.group);
        this.actors.set(card.id, actor);
      }
      actor.setDragging(false); // Safety: clear dragging on every sync
      actor.update(card);
    });
  }

  public updateLayouts() {
    this.zones.forEach(zone => {
        zone.layout(this.actors);
    });
  }

  public tick(delta: number, time: number) {
    this.zones.forEach(zone => {
        zone.tick?.(delta, time);
    });
    this.actors.forEach(actor => actor.tick(delta, time));

    // Animate and clean up dying actors.
    const now = performance.now();
    for (let i = this.dyingActors.length - 1; i >= 0; i--) {
      const entry = this.dyingActors[i];
      entry.actor.tick(delta, time);
      if (now >= entry.removeAt) {
        this.scene.remove(entry.actor.group);
        this.dyingActors.splice(i, 1);
      }
    }
  }
  
  public getAllActors() {
      return Array.from(this.actors.values());
  }

  public getActor(id: string) {
      return this.actors.get(id) ?? null;
  }

  public getInteractables() {
      const cardHits = Array.from(this.actors.values()).flatMap(actor => actor.getInteractables());
      const bfZone = this.zones.get('battlefield') as any;
      const playerHits =
        bfZone && typeof bfZone.getPlayerInteractables === 'function'
          ? bfZone.getPlayerInteractables()
          : [];
      const extraHits = Array.from(this.zones.values()).flatMap((zone: any) =>
        typeof zone.getExtraInteractables === 'function' ? zone.getExtraInteractables() : [],
      );
      return [...cardHits, ...playerHits, ...extraHits];
  }

  public getFactory() {
      return this.factory;
  }
}
