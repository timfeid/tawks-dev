import * as THREE from 'three';

/** What the throw or swing is doing, for the overlay to draw. */
export type ThrowState = {
  label: string;
  /** The power swing or the timing needle, 0 to 1, and whether it is moving. */
  power: number;
  charging: boolean;
  /** A needle to stop in the middle, as for the bat, rather than power to fill. */
  needle: boolean;
  fuseLeft: number;
  fuse: number;
  /** How hard the wind blows, and which way across the view: 0 carries the throw on. */
  wind: number;
  windAngle: number;
  /** In place of the usual line under the label, and the wind left out, as for the vacuum. */
  hint?: string;
  calm?: boolean;
  walking?: boolean;
  cash?:boolean;
  score?: number;
  collected?: number;
  goal?: number;
  targetProgress?: number;
  time?: number;
  notice?: string;
  worldReticle?: boolean;
};

const W = 1024;
const H = 576;

/**
 * The overlay drawn over the view while you throw or swing in first person:
 * part of the scene, hung in front of the camera, so nothing on the page sits
 * under the mouse. A crosshair, the power ring round it (or the timing
 * needle under it), the wind, and the fuse burning down.
 */
export class ThrowHud {
  readonly mesh: THREE.Mesh<THREE.PlaneGeometry, THREE.MeshBasicMaterial>;
  private canvas = document.createElement('canvas');
  private ctx: CanvasRenderingContext2D;
  private texture: THREE.CanvasTexture;
  private walkScore = -1;
  private pickupUntil = 0;
  private pickupAmount = 0;
  private drawAt=0;

  constructor(camera: THREE.PerspectiveCamera, private viewport?:()=>{width:number;height:number}) {
    this.canvas.width = W;
    this.canvas.height = H;
    this.ctx = this.canvas.getContext('2d')!;
    this.texture = new THREE.CanvasTexture(this.canvas);
    this.texture.colorSpace = THREE.SRGBColorSpace;
    this.mesh = new THREE.Mesh(
      new THREE.PlaneGeometry(1, 1),
      new THREE.MeshBasicMaterial({ map: this.texture, transparent: true, depthTest: false, depthWrite: false }),
    );
    this.mesh.renderOrder = 999;
    this.mesh.frustumCulled = false;
    this.mesh.visible = false;
    camera.add(this.mesh);
  }

  /** Fills the view at the camera's current lens and shape; call each frame it shows. */
  private fit(camera: THREE.PerspectiveCamera) {
    const distance = 0.5;
    const height = 2 * Math.tan(THREE.MathUtils.degToRad(camera.fov / 2)) * distance;
    this.mesh.position.set(0, 0, -distance);
    this.mesh.scale.set(height * camera.aspect, height, 1);
  }

  hide() {
    this.mesh.visible = false;
    this.pickupUntil = 0;
    this.walkScore = -1;
  }

  refit(camera: THREE.PerspectiveCamera) {
    if (this.mesh.visible) this.fit(camera);
  }

  show(camera: THREE.PerspectiveCamera, state: ThrowState) {
    this.fit(camera);
    const size=this.viewport?.();
    const W=size?Math.max(320,Math.round(size.width)):1024,H=size?Math.max(240,Math.round(size.height)):576;
    const changed=this.canvas.width!==W||this.canvas.height!==H;
    if(changed){this.canvas.width=W;this.canvas.height=H;}
    const now=performance.now();
    if(this.mesh.visible&&!changed&&state.calm&&!state.needle&&now-this.drawAt<33)return;
    this.drawAt=now;
    this.mesh.visible = true;
    // Use the view's real aspect ratio so the reticle and edge furniture stay
    // round and inside the safe area, including a portrait phone.
    const squeeze = size?1:(W / H) / camera.aspect;
    const half = size?W/2:(H * camera.aspect) / 2;
    const compact = camera.aspect < 1.2;
    const inset = compact ? 20 : 32;
    const accent = state.calm ? '#67e8f9' : '#fcd34d';
    const grappling = /^grapple\b/i.test(state.label);
    const ctx = this.ctx;
    ctx.clearRect(0, 0, W, H);
    ctx.save();
    ctx.translate(W / 2, 0);
    ctx.scale(squeeze, 1);
    ctx.textBaseline = 'middle';
    ctx.shadowColor = 'rgba(2,6,23,0.85)';
    ctx.shadowBlur = 5;

    if(grappling) {
      ctx.textAlign='center';
      ctx.font='900 13px system-ui, sans-serif';
      ctx.fillStyle='#67e8f9';
      ctx.fillText('HOOK A RIVAL · STEAL UP TO 8 COINS',0,37,half*2-inset*2);
      ctx.font='600 11px system-ui, sans-serif';
      ctx.fillStyle='rgba(255,255,255,0.85)';
      ctx.fillText('Land near their feet. Reel in their coins.',0,55,half*2-inset*2);
    }

    if (state.walking) {
      const banner = Math.min(230, half - inset);
      ctx.fillStyle = 'rgba(2,6,23,0.78)';
      ctx.beginPath();ctx.moveTo(-banner, 23);ctx.lineTo(banner, 23);ctx.lineTo(banner - 10, 71);ctx.lineTo(-banner + 10, 71);ctx.closePath();ctx.fill();
      ctx.fillStyle = accent;ctx.fillRect(-banner + 1, 22, banner * 2 - 2, 2);
      ctx.textAlign = 'center';
      ctx.fillStyle = state.charging ? accent : '#fff';
      ctx.font = '900 14px system-ui, sans-serif';
      ctx.fillText(state.notice ?? (state.charging ? 'SUCTION ON' : 'HOLD CLICK / SPACE TO VACUUM'), 0, 38, half * 2 - inset * 2);
      ctx.font = '600 11px system-ui, sans-serif';
      ctx.fillStyle = 'rgba(255,255,255,0.85)';
      ctx.fillText(state.notice ? 'Keep aiming and hold suction while you wait.' : 'Aim at pedestrians to rob cash. Props and candys can be vacuumed too.', 0, 59, half * 2 - inset * 2);
      if (this.walkScore>=0 && (state.score ?? 0) > this.walkScore) {
        this.pickupAmount = (state.score ?? 0) - this.walkScore;
        this.pickupUntil = performance.now() + 850;
      }
      this.walkScore = state.score ?? 0;
    }
    if (state.notice && !state.walking) {
      ctx.textAlign = 'center';ctx.fillStyle = '#fcd34d';ctx.font = '800 12px system-ui, sans-serif';
      ctx.fillText(state.notice, 0, 38, half * 2 - inset * 2);
    }

    // A faint vignette supports the HUD without putting a panel over play.
    const shade = ctx.createLinearGradient(0, H - 125, 0, H);
    shade.addColorStop(0, 'rgba(2,6,23,0)');
    shade.addColorStop(1, 'rgba(2,6,23,0.65)');
    ctx.fillStyle = shade;
    ctx.fillRect(-half, H - 125, half * 2, 125);

    // Small open brackets keep the centre of the target visible.
    ctx.save();
    ctx.translate(0, H / 2);
    if (!state.worldReticle) {
    ctx.beginPath();
    for (const side of [-1, 1]) {
      ctx.moveTo(side * 10, -8);
      ctx.lineTo(side * 16, -8);
      ctx.lineTo(side * 16, 8);
      ctx.lineTo(side * 10, 8);
    }
    ctx.strokeStyle = 'rgba(2,6,23,0.7)';ctx.lineWidth = 5;ctx.stroke();
    ctx.strokeStyle = state.calm && state.charging ? accent : 'rgba(255,255,255,0.95)';ctx.lineWidth = 2;ctx.stroke();
    ctx.fillStyle = accent;
    ctx.fillRect(-1.5, -1.5, 3, 3);
    }

    if (state.walking && state.charging && !state.worldReticle) {
      // Air streams make a held nozzle visible even before a target is hit.
      // Only authoritative target progress fills the gold collection ring.
      const phase = (state.time ?? 0) * 1.8;
      ctx.lineWidth = 2;
      for (let i = 0; i < 8; i++) {
        const angle = i * Math.PI / 4 + phase * 0.3;
        const t = (phase + i * 0.17) % 1;
        const r = 64 - t * 40;
        ctx.beginPath();
        ctx.moveTo(Math.cos(angle) * r, Math.sin(angle) * r);
        ctx.lineTo(Math.cos(angle + 0.06) * (r - 11), Math.sin(angle + 0.06) * (r - 11));
        ctx.strokeStyle = `rgba(2,6,23,${0.15 + t * 0.45})`;ctx.lineWidth = 4;ctx.stroke();
        ctx.strokeStyle = `rgba(103,232,249,${0.3 + t * 0.65})`;ctx.lineWidth = 2;ctx.stroke();
      }
      const progress = Math.max(0, Math.min(1, state.targetProgress ?? 0));
      if (progress > 0) {
        ctx.strokeStyle = '#fcd34d';ctx.lineWidth = 4;
        ctx.beginPath();ctx.arc(0, 0, 29, -Math.PI / 2, -Math.PI / 2 + progress * Math.PI * 2);ctx.stroke();
      }
    }
    if (state.walking && performance.now() < this.pickupUntil) {
      ctx.textAlign = 'center';ctx.font = '900 18px system-ui, sans-serif';ctx.fillStyle = '#fcd34d';
      ctx.fillText(state.cash?`+$${this.pickupAmount}`:`+${this.pickupAmount} COIN${this.pickupAmount === 1 ? '' : 'S'}`, 0, 84);
    }

    if (state.needle) {
      const w = Math.min(240, half * 1.3);
      const y = 48;
      const band = (from: number, to: number, colour: string) => {
        ctx.fillStyle = colour;
        ctx.fillRect(-w / 2 + w * from, y, w * (to - from), 8);
      };
      band(0, 1, 'rgba(2,6,23,0.7)');
      band(0.075, 0.925, 'rgba(148,163,184,0.55)');
      band(0.25, 0.75, '#fb923c');
      band(0.425, 0.575, '#fcd34d');
      ctx.fillStyle = '#fff';
      ctx.fillRect(-w / 2 + w * state.power - 1.5, y - 5, 3, 18);
      ctx.font = '800 10px system-ui, sans-serif';
      ctx.textAlign = 'center';
      ctx.fillStyle = 'rgba(255,255,255,0.7)';
      ctx.fillText('TIME YOUR SWING', 0, y + 26);
    } else if (!state.calm) {
      ctx.lineWidth = 4;
      ctx.strokeStyle = 'rgba(2,6,23,0.5)';
      ctx.beginPath();
      ctx.arc(0, 0, 34, -Math.PI / 2, Math.PI * 1.5);
      ctx.stroke();
      if (state.charging) {
        ctx.strokeStyle = accent;
        ctx.beginPath();
        ctx.arc(0, 0, 34, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * state.power);
        ctx.stroke();
      }
    }
    ctx.restore();

    // Equipment reads like a game's loadout, tucked against the lower edge.
    const left = -half + inset;
    const base = H - 76;
    ctx.textAlign = 'left';
    ctx.fillStyle = accent;
    ctx.fillRect(left, base - 16, 3, 48);
    ctx.font = '800 10px system-ui, sans-serif';
    ctx.fillText(state.walking ? 'ON THE MOVE' : 'EQUIPPED', left + 13, base - 10);
    ctx.font = `italic 900 ${compact ? 22 : 28}px system-ui, sans-serif`;
    ctx.fillStyle = '#fff';
    ctx.fillText(state.label.replace(/ in hand$/i, '').toUpperCase(), left + 12, base + 13, half * (compact ? 0.9 : 1.15));

    // A segmented battery/fuse and a large readable clock, no enclosing box.
    const right = half - inset;
    const catches = state.collected ?? state.score ?? 0;
    const goal = Math.max(1, state.goal ?? 3);
    const fraction = state.walking && !state.cash ? catches/goal : Math.max(0, Math.min(1, state.fuseLeft / Math.max(0.1, state.fuse)));
    const danger = !state.walking && state.fuseLeft < 3;
    ctx.textAlign = 'right';
    ctx.font = '800 10px system-ui, sans-serif';
    ctx.fillStyle = danger ? '#fda4af' : accent;
    ctx.fillText(state.cash ? 'CASH' : state.walking ? 'COLLECTED' : state.calm ? 'BATTERY' : 'TIME LEFT', right, base - 10);
    ctx.font = `900 ${compact ? 30 : 38}px system-ui, sans-serif`;
    ctx.fillStyle = danger ? '#fb7185' : '#fff';
    ctx.fillText(state.cash ? `$${state.score??0}` : state.walking ? String(state.score??0).padStart(2,'0') : state.fuseLeft.toFixed(1), right, base + 19);
    const segments = state.walking && !state.cash ? goal : 10;
    const width = compact ? 70 : 100;
    for (let i = 0; i < segments; i++) {
      ctx.fillStyle = i / segments < fraction ? (danger ? '#fb7185' : accent) : 'rgba(255,255,255,0.15)';
      ctx.fillRect(right - width + i * (width / segments), base + 44, width / segments - 3, 4);
    }

    if (!state.needle && !state.calm) {
      ctx.save();
      ctx.translate(left + 12, 38);
      ctx.rotate(-state.windAngle);
      ctx.fillStyle = '#7dd3fc';
      ctx.beginPath();
      ctx.moveTo(0, -12);
      ctx.lineTo(7, 4);
      ctx.lineTo(2, 2);
      ctx.lineTo(2, 12);
      ctx.lineTo(-2, 12);
      ctx.lineTo(-2, 2);
      ctx.lineTo(-7, 4);
      ctx.closePath();
      ctx.fill();
      ctx.restore();
      ctx.font = '800 11px system-ui, sans-serif';
      ctx.textAlign = 'left';
      ctx.fillStyle = '#bae6fd';
      ctx.fillText(`WIND  ${state.wind.toFixed(1)}`, left + 30, 38);
    }

    // One short control line, below the action rather than across the view.
    ctx.font = '600 11px system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillStyle = 'rgba(255,255,255,0.75)';
    ctx.fillText(
      state.hint ?? (state.needle
        ? 'CLICK / SPACE  Swing     Q  Shoulder     ESC  Cursor'
        : 'HOLD CLICK / SPACE  Charge · release to throw     Q  Shoulder     ESC  Cursor'),
      0, H - 15, half * 2 - inset * 2,
    );
    ctx.restore();
    this.texture.needsUpdate = true;
  }

  dispose() {
    this.mesh.removeFromParent();
    this.mesh.geometry.dispose();
    this.mesh.material.dispose();
    this.texture.dispose();
  }
}
