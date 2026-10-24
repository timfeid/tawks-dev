import * as THREE from 'three';
import type { ZoneManager } from './ZoneManager';
import type { TableManager } from './TableManager';
import type { CardView } from '../types';

export class CombatAnimator {
  private previousCombatPairs: any[] = [];
  private previousCardIds = new Set<string>();
  private previousPlayerLife = new Map<string, number>();
  private previousUnitStats = new Map<string, { toughness: number; shield: number; armor: number; damage: number }>();
  private lastActorPositions = new Map<
    string,
    { pos: THREE.Vector3; rot: THREE.Euler; scale: THREE.Vector3; tex: any }
  >();

  constructor(
    private zoneManager: ZoneManager,
    private tableManager: TableManager,
    private scene: THREE.Scene,
  ) {}

  public onCombatUpdate(currentPairs: any[], players: any[]): void {
    // Player damage / life gain tracking
    for (const player of players) {
      const prevLife = this.previousPlayerLife.get(player.id);
      if (prevLife !== undefined && player.life !== prevLife) {
        const playerIndex = players.indexOf(player);
        const targetSeatAngle = (playerIndex / players.length) * Math.PI * 2;
        const edgeRadius = 6.5;
        const x = Math.cos(targetSeatAngle) * edgeRadius;
        const z = Math.sin(targetSeatAngle) * edgeRadius;
        const delta = player.life - prevLife;
        if (delta < 0) {
          this.tableManager.addBurst(x, z, 0xff0000);
          this.tableManager.addFloatingText(x, 1.2, z, `${delta}`, 0xff3333, 1.4);
        } else if (delta > 0) {
          this.tableManager.addFloatingText(x, 1.2, z, `+${delta}`, 0x22c55e, 1.2);
        }
      }
      this.previousPlayerLife.set(player.id, player.life);
    }

    const isCombatConcluded =
      this.previousCombatPairs.length > 0 && currentPairs.length === 0;

    if (isCombatConcluded) {
      for (const pair of this.previousCombatPairs) {
        const attackerActor = this.zoneManager.getActor(pair.attacker.id);
        if (!attackerActor) continue;

        const startPos = attackerActor.getTargetPosition();
        startPos.y += 0.5;

        let targetPos: THREE.Vector3 | null = null;
        let color = 0xff2200;

        if (pair.is_blocked && pair.blockers && pair.blockers.length > 0) {
          const blockerActor = this.zoneManager.getActor(pair.blockers[0].id);
          if (blockerActor) {
            targetPos = blockerActor.getTargetPosition();
            color = 0xffcc00;

            const blockerPos = targetPos.clone();
            const stepDir = startPos
              .clone()
              .sub(blockerPos)
              .normalize()
              .multiplyScalar(0.5);
            const steppedPos = blockerPos.clone().add(stepDir);
            blockerActor.setTarget(steppedPos, blockerActor.group.rotation, blockerActor.group.scale.x, 0.2);

            const zm = this.zoneManager;
            setTimeout(() => {
              if (zm.getActor(pair.blockers[0].id)) {
                blockerActor.setTarget(blockerPos, blockerActor.group.rotation, blockerActor.group.scale.x, 0.4);
              }
            }, 800);
          }
        }

        if (!targetPos && pair.target_kind === 'unit' && pair.target_card_id) {
          const targetActor = this.zoneManager.getActor(pair.target_card_id);
          if (targetActor) targetPos = targetActor.getTargetPosition();
        }

        if (!targetPos && pair.target_player_id) {
          const targetIndex = players.findIndex((p: any) => p.id === pair.target_player_id);
          if (targetIndex >= 0) {
            const targetSeatAngle = (targetIndex / players.length) * Math.PI * 2;
            const edgeRadius = 6.5;
            targetPos = new THREE.Vector3(
              Math.cos(targetSeatAngle) * edgeRadius,
              0.0,
              Math.sin(targetSeatAngle) * edgeRadius,
            );
          }
        }

        if (targetPos) {
          const tm = this.tableManager;
          const zm = this.zoneManager;
          const capturedTargetPos = targetPos;
          setTimeout(() => {
            tm.fireProjectile(startPos, capturedTargetPos, color);

            const originalPos = attackerActor.getTargetPosition();
            const lungeDir = capturedTargetPos
              .clone()
              .sub(originalPos)
              .normalize()
              .multiplyScalar(0.8);
            const lungePos = originalPos.clone().add(lungeDir);
            attackerActor.setTarget(lungePos, attackerActor.group.rotation, attackerActor.group.scale.x, 0.1);

            setTimeout(() => {
              if (zm.getActor(pair.attacker.id)) {
                attackerActor.setTarget(originalPos, attackerActor.group.rotation, attackerActor.group.scale.x, 0.3);
              }
            }, 200);
          }, Math.random() * 400);
        }
      }
    }

    this.previousCombatPairs = currentPairs;
  }

  public onBattlefieldUpdate(currentUnits: CardView[]): void {
    const currentIds = new Set(currentUnits.map((c) => c.id));

    const isInitialSync = this.previousCardIds.size === 0;

    // Unit creation and damage tracking
    for (const unit of currentUnits) {
      const isNew = !this.previousCardIds.has(unit.id);

      // A plain card-play (no token creator) — flash the landing spot so the
      // player can see where the new card appeared. Skip on the very first
      // sync so we don't flash the whole opening board.
      if (isNew && !unit.creator_id && !isInitialSync) {
        const unitActor = this.zoneManager.getActor(unit.id);
        if (unitActor) {
          const pos = unitActor.getTargetPosition();
          this.tableManager.addBurst(pos.x, pos.z, 0xffffff, pos.y + 0.15);
          setTimeout(() => {
            this.tableManager.addBurst(pos.x, pos.z, 0xfbbf24, pos.y + 0.15);
          }, 120);
        }
      }

      if (isNew && unit.creator_id) {
        const creatorActor = this.zoneManager.getActor(unit.creator_id);
        const unitActor = this.zoneManager.getActor(unit.id);
        
        if (unitActor) {
          // Find source position: either from current actor or saved position
          let sourcePos: THREE.Vector3 | null = null;
          if (creatorActor) {
            sourcePos = creatorActor.group.position.clone();
          } else {
            const saved = this.lastActorPositions.get(unit.creator_id);
            if (saved) sourcePos = saved.pos.clone();
          }

          if (sourcePos) {
            const targetPos = unitActor.getTargetPosition();
            const targetRot = unitActor.getTargetRotation();
            const targetScale = unitActor.group.scale.x;

            // Start unit at creator's position
            unitActor.group.position.copy(sourcePos);
            unitActor.group.scale.setScalar(0.1);
            
            // Fire a small burst at the creator
            this.tableManager.addBurst(sourcePos.x, sourcePos.z, 0x00ffcc, sourcePos.y);

            // Animate to target
            setTimeout(() => {
              if (this.zoneManager.getActor(unit.id)) {
                unitActor.setTarget(targetPos, targetRot, targetScale, 0.08);
              }
            }, 50);
          }
        }
      }

      const prev = this.previousUnitStats.get(unit.id);
      if (prev) {
        const toughness = unit.toughness ?? 0;
        const shield = unit.shield ?? 0;
        const armor = unit.armor ?? 0;
        const unitDamage = (unit as any).damage ?? 0;
        const damageDelta = Math.max(0, unitDamage - prev.damage);
        const statDrop =
          Math.max(0, prev.toughness - toughness) +
          Math.max(0, prev.shield - shield) +
          Math.max(0, prev.armor - armor);
        const damage = damageDelta + statDrop;
        if (damage > 0) {
          const actor = this.zoneManager.getActor(unit.id);
          if (actor && (actor as any).takeDamage) {
            (actor as any).takeDamage(damage);
            const pos = actor.getTargetPosition();

            // Floating damage number above the unit
            this.tableManager.addFloatingText(
              pos.x,
              pos.y + 0.8,
              pos.z,
              `-${damage}`,
              0xff3333,
              1.0,
            );
            // Big hit effect
            this.tableManager.addBurst(pos.x, pos.z, 0xff0000, pos.y + 0.2);
            // Aftershock sparks
            setTimeout(() => {
              this.tableManager.addBurst(pos.x + (Math.random()-0.5)*0.8, pos.z + (Math.random()-0.5)*0.8, 0xffaa00, pos.y + Math.random()*0.5);
            }, 100);
            setTimeout(() => {
              this.tableManager.addBurst(pos.x + (Math.random()-0.5)*0.8, pos.z + (Math.random()-0.5)*0.8, 0xff5500, pos.y + Math.random()*0.5);
            }, 200);
          }
        }
      }
      this.previousUnitStats.set(unit.id, {
        toughness: unit.toughness ?? 0,
        shield: unit.shield ?? 0,
        armor: unit.armor ?? 0,
        damage: (unit as any).damage ?? 0,
      });
    }

    if (this.previousCardIds.size > 0) {
      for (const id of this.previousCardIds) {
        if (!currentIds.has(id)) {
          const lastState = this.lastActorPositions.get(id);
          if (lastState) {
            const scene = this.scene;
            const tm = this.tableManager;
            setTimeout(() => {
              tm.addBurst(lastState.pos.x, lastState.pos.z, 0xff2200);

              if (lastState.tex) {
                const shardCount = 8;
                const shardWidth = 0.75 / shardCount;
                const shards: THREE.Mesh[] = [];

                for (let i = 0; i < shardCount; i++) {
                  const shardGeo = new THREE.PlaneGeometry(shardWidth, 1.1);
                  const uvs = shardGeo.attributes.uv;
                  for (let j = 0; j < uvs.count; j++) {
                    const u = uvs.getX(j);
                    uvs.setX(j, (u + i) / shardCount);
                  }

                  const shardMat = new THREE.MeshBasicMaterial({
                    map: lastState.tex,
                    transparent: true,
                    opacity: 0.9,
                    color: 0xff3300,
                    blending: THREE.AdditiveBlending,
                    depthWrite: false,
                    side: THREE.DoubleSide,
                  });

                  const shard = new THREE.Mesh(shardGeo, shardMat);

                  // Position shard correctly relative to center
                  const offset = (i - (shardCount - 1) / 2) * shardWidth;
                  shard.position.copy(lastState.pos);
                  const localOffset = new THREE.Vector3(offset, 0, 0).applyEuler(
                    lastState.rot,
                  );
                  shard.position.add(localOffset);

                  shard.rotation.copy(lastState.rot);
                  shard.scale.copy(lastState.scale);
                  scene.add(shard);
                  shards.push(shard);

                  const vel = new THREE.Vector3(
                    (Math.random() - 0.5) * 0.15 + localOffset.x * 0.5,
                    0.1 + Math.random() * 0.2,
                    (Math.random() - 0.5) * 0.15 + localOffset.z * 0.5,
                  );
                  const rotVel = new THREE.Euler(
                    Math.random() * 0.2,
                    Math.random() * 0.2,
                    Math.random() * 0.2,
                  );

                  let life = 1.0;
                  const animateShard = () => {
                    life -= 0.02 + Math.random() * 0.02;
                    if (life <= 0) {
                      scene.remove(shard);
                      shardGeo.dispose();
                      shardMat.dispose();
                      return;
                    }

                    shard.position.add(vel);
                    shard.rotation.x += rotVel.x;
                    shard.rotation.y += rotVel.y;
                    shard.rotation.z += rotVel.z;
                    shardMat.opacity = life;
                    shard.scale.multiplyScalar(0.98);

                    requestAnimationFrame(animateShard);
                  };
                  animateShard();
                }
              }
            }, 600);
          }
        }
      }
    }

    this.previousCardIds = currentIds;
  }

  public saveActorPositions(): void {
    this.lastActorPositions.clear();
    for (const actor of this.zoneManager.getAllActors()) {
      this.lastActorPositions.set(actor.id, {
        pos: actor.group.position.clone(),
        rot: actor.group.rotation.clone(),
        scale: actor.group.scale.clone(),
        tex: (actor as any).frontMesh?.material?.map,
      });
    }
  }
}
