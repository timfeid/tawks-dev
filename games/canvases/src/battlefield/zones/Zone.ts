import * as THREE from 'three';
import type { CardActor } from '../entities/CardActor';

export interface Zone {
  id: string;
  update(cards: any[]): void; // Sync with game state
  layout(actors: Map<string, CardActor>): void; // Calculate positions
  getActor(cardId: string): CardActor | undefined;
  getDropIndex(point: THREE.Vector3, viewerId: string | null): number;
  getCardIds(viewerId: string | null): string[];
  getSceneObjects(): THREE.Object3D[];
  setPreviewOrder?(order: string[] | null): void;
  tick?(delta: number, time?: number): void;
}

export abstract class BaseZone implements Zone {
  protected actors = new Map<string, CardActor>();
  
  constructor(public id: string) {}

  abstract update(cards: any[]): void;
  abstract layout(allActors: Map<string, CardActor>): void;
  abstract getDropIndex(point: THREE.Vector3, viewerId: string | null): number;
  abstract getCardIds(viewerId: string | null): string[];

  public getSceneObjects(): THREE.Object3D[] {
      return [];
  }

  public getActor(cardId: string): CardActor | undefined {
    return this.actors.get(cardId);
  }
}
