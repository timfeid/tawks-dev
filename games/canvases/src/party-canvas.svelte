<script lang="ts">
  import { getContext, onMount } from 'svelte';
  import * as THREE from 'three';
  import { OutputPass } from 'three/examples/jsm/postprocessing/OutputPass.js';
  import type { BoutFrame, BoutHost, BoutHud, BoutModule, BoutPhase, BoutRenderer } from '@tawks/bout-sdk';
  import type { WorkflowResource } from '@tawks/workflow-types';
  import type { WorkflowStateProvider } from '@tawks/workflow-ui/workflow';
  import { submitServerAction } from '@tawks/workflow-ui/components/workflow';
  import { SceneManager } from './battlefield/engine/SceneManager';
  import { PartyConnection, type ConnectionStatus } from './party/connection';
  import type { CharacterDef, ClientMsg, CovenView, GameEvent, Item, PartyPass, RoomView } from './party/protocol';
  import { createPartyStage, WATER_Y } from './party/stage';
  import { buildCharacter } from './party/characters';
  import { loadBout } from './party/bouts/registry';
  import { ShooterControls } from './party/shooter/controls';
  import { PartyBoard } from './party/board/scene';
  import { CovenTable } from './party/coven/table';
  import CovenHud from './party/coven/coven-hud.svelte';
  import { LandlordTable } from './party/landlord/table';
  import LandlordHud from './party/landlord/landlord-hud.svelte';
  import type { LandlordView } from './party/landlord/rules';
  import { PartyInput, type StickValue } from './party/input';
  import { PartySfx } from './party/sfx';
  import RoomHud from './party/room-hud.svelte';
  import { observePartyViewport } from './party/viewport';

  let {
    workflow,
    watchPass = null,
  }: {
    workflow?: WorkflowResource;
    /** A recorder's watch-only pass, in place of joining through a workflow. */
    watchPass?: PartyPass | null;
  } = $props();
  const provider = getContext<WorkflowStateProvider>('workflow-provider');
  const filming = $derived(!!watchPass);

  const pass = $derived(watchPass ?? (workflow?.responses?.party as PartyPass | null) ?? null);

  let canvas: HTMLCanvasElement;
  let root: HTMLDivElement;
  let room = $state<RoomView | null>(null);
  let roster = $state<CharacterDef[]>([]);
  let me = $state<string | null>(null);
  let spectating = $state(false);
  let guest = $state(false);
  let status = $state<ConnectionStatus>('connecting');
  let errorMessage = $state<string | null>(null);
  let timeLeft = $state<number | null>(null);
  let showGo = $state(false);
  let muted = $state(false);
  let touch = $state(false);
  let overview = $state(false);
  let aimItem = $state<Item | null>(null);
  let pointerLocked = $state(false);
  let hud = $state<BoutHud | null>(null);
  let covenView = $state<CovenView | null>(null);
  let landlordView = $state<LandlordView | null>(null);
  let covenHit = $state<{ seat: number; index?: number; at: number } | null>(null);
  let nativeFullscreen = $state(false);
  let expanded = $state(false);
  const fullscreen = $derived(nativeFullscreen || expanded);
  const firstPerson = $derived(room?.bouts.find((bout) => bout.key === room?.minigame)?.controls === 'first_person');

  let connection: PartyConnection | null = null;
  let input: PartyInput | null = null;
  let sfx: PartySfx | null = null;
  let refreshing = false;
  let errorTimer: ReturnType<typeof setTimeout> | null = null;
  let goTimer: ReturnType<typeof setTimeout> | null = null;
  let restoredPick = false;
  let recamera: (() => void) | null = null;
  /** A dash or shot asked for in a top-down bout, sent on the next frame. */
  let actionPressed = false;
  let look: (dx: number, dy: number) => void = () => {};
  let trigger: (down: boolean) => void = () => {};
  let duck: (down: boolean) => void = () => {};
  let resizeRoot: (() => void) | null = null;

  async function toggleFullscreen() {
    if (nativeFullscreen) await document.exitFullscreen();
    else if (expanded) expanded = false;
    else {
      try {
        await root.requestFullscreen();
      } catch {
        // Browsers without the Fullscreen API can still fill the app viewport.
        expanded = true;
      }
    }
  }

  $effect(() => {
    if (!expanded) return;
    const previous = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    return () => {
      document.body.style.overflow = previous;
    };
  });

  $effect(() => {
    void fullscreen;
    resizeRoot?.();
  });

  function toggleOverview() {
    overview = !overview;
    recamera?.();
  }

  function refreshPass() {
    if (refreshing || filming || !workflow) return;
    refreshing = true;
    void submitServerAction(provider, workflow.instance_id, { lobby_id: (workflow.responses?.lobby_id as string) ?? null }, 'games.party.join');
    setTimeout(() => (refreshing = false), 3000);
  }

  function send(msg: ClientMsg) {
    connection?.send(msg);
  }

  function showError(message: string) {
    errorMessage = message;
    if (errorTimer) clearTimeout(errorTimer);
    errorTimer = setTimeout(() => (errorMessage = null), 3500);
  }

  function phaseChanged(previous: RoomView | null, next: RoomView) {
    const before = previous?.phase;
    const after = next.phase;
    if (after.name === 'countdown' && (before?.name !== 'countdown' || before.seconds !== after.seconds)) {
      sfx?.count(false);
    }
    if (after.name === 'playing' && before?.name === 'countdown') {
      sfx?.count(true);
      showGo = true;
      if (goTimer) clearTimeout(goTimer);
      goTimer = setTimeout(() => (showGo = false), 800);
    }
    if (after.name === 'results' && before?.name !== 'results') {
      const seat = next.members.find((m) => m.user_id === me)?.seat;
      if (seat !== undefined && after.placements[0]?.includes(seat)) sfx?.win();
    }
    // Bring back the character this person picked last time, if it is free.
    const mine = next.members.find((m) => m.user_id === me);
    if (!restoredPick && mine && after.name === 'lobby') {
      restoredPick = true;
      try {
        const favorite = localStorage.getItem('party-character');
        if (favorite && favorite !== mine.character && !next.members.some((m) => m.character === favorite)) {
          send({ type: 'pick', character: favorite });
        }
      } catch {
        // No stored favorite.
      }
    }
  }

  $effect(() => {
    if (pass && connection) connection.setPass(pass.ws_url, pass.token);
  });

  onMount(() => {
    const viewport = observePartyViewport(root, () => fullscreen);
    resizeRoot = viewport.resize;
    const onFullscreenChange = () => (nativeFullscreen = document.fullscreenElement === root);
    const onEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') expanded = false;
    };
    document.addEventListener('fullscreenchange', onFullscreenChange);
    document.addEventListener('keydown', onEscape);
    touch = window.matchMedia('(pointer: coarse)').matches;
    const mobile = touch || window.innerWidth < 760;
    const manager = new SceneManager({ canvas, useBloom: false, useSmaa: !mobile });
    manager.composer.addPass(new OutputPass());
    const { scene, camera, renderer } = manager;
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, mobile ? 1.5 : 2));
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.05;
    camera.fov = 40;
    camera.far = 400;

    const stage = createPartyStage(scene, { mobile });
    // First-person bouts hang their view models off the camera.
    scene.add(camera);
    const partyBoard = new PartyBoard(scene);
    const covenTable = new CovenTable(scene);
    const landlordTable = new LandlordTable(scene);
    const controls = new ShooterControls(0.0022, (locked) => (pointerLocked = locked));
    controls.attach(canvas);
    input = new PartyInput();
    sfx = new PartySfx();
    muted = sfx.muted;
    let rosterMap = new Map<string, CharacterDef>();

    const host: BoutHost = {
      scene,
      camera,
      canvas,
      mobile,
      waterY: WATER_Y,
      sounds: sfx,
      character: (id) => rosterMap.get(id),
      rider: (character) => buildCharacter(character as CharacterDef),
      send: (message) => connection?.send(message),
    };
    let bout: { key: string; module: BoutModule; renderer: BoutRenderer } | null = null;
    let loading = '';
    let stopped = false;
    let lastView: BoutFrame<unknown, unknown> | null = null;
    const swapBout = (key: string) => {
      if ((bout?.key ?? '') === key) return;
      bout?.renderer.dispose();
      bout = null;
      hud = null;
      recamera?.();
      if (!key) {
        loading = '';
        return;
      }
      if (loading === key) return;
      loading = key;
      loadBout(key)?.then((module) => {
        if (stopped || loading !== key) return;
        loading = '';
        bout = { key, module, renderer: module.create(host) };
        recamera?.();
      });
    };

    // Where the pointer is over the arena, for bouts that aim with the mouse.
    const pointer = new THREE.Vector2();
    let aimYaw = 0;
    const groundPlane = new THREE.Plane(new THREE.Vector3(0, 1, 0), -0.9);
    const aimRay = new THREE.Raycaster();
    const aimPoint = new THREE.Vector3();
    const onPointerMove = (event: MouseEvent) => {
      const box = canvas.getBoundingClientRect();
      pointer.set(((event.clientX - box.left) / box.width) * 2 - 1, -((event.clientY - box.top) / box.height) * 2 + 1);
    };
    canvas.addEventListener('mousemove', onPointerMove);
    let controlsKind = 'move';
    const askAction = () => {
      if (controlsKind === 'move_action' || controlsKind === 'move_aim') actionPressed = true;
    };
    const onActionKey = (event: KeyboardEvent) => {
      if (event.code === 'Space') askAction();
    };
    canvas.addEventListener('mousedown', askAction);
    window.addEventListener('keydown', onActionKey);
    look = (dx, dy) => controls.look(dx, dy);
    trigger = (down) => {
      controls.pullTrigger(down);
      if (down) askAction();
    };
    duck = (down) => controls.setCrouch(down);

    const cameraOffset = new THREE.Vector3();
    const frameHalf = new THREE.Vector2(13, 10);
    const follow = new THREE.Vector3();
    const target = new THREE.Vector3();
    let lookHeight = -0.8;
    let onBoard = false;
    let atTable = false;
    const onTableClick = (event: MouseEvent) => {
      if (!atTable) return;
      onPointerMove(event);
      aimRay.setFromCamera(pointer, camera);
      const found = room?.minigame === 'landlord' ? landlordTable.pick(aimRay) : covenTable.pick(aimRay);
      if (found) covenHit = { ...found, at: performance.now() };
    };
    canvas.addEventListener('click', onTableClick);
    const fit = () => {
      camera.aspect = canvas.clientWidth / Math.max(1, canvas.clientHeight);
      camera.updateProjectionMatrix();
    };
    /** Keeps a box of `wide` by `tall` in view from above at `pitchDegrees`. */
    const frameBox = (wide: number, tall: number, pitchDegrees: number) => {
      const portrait = camera.aspect < 0.8;
      const halfV = THREE.MathUtils.degToRad(camera.fov / 2);
      const halfH = Math.atan(Math.tan(halfV) * camera.aspect);
      frameHalf.set(wide, tall);
      const distance = Math.max(wide / Math.tan(halfH), tall / Math.tan(halfV)) * (portrait ? 1.0 : 1.02);
      const pitch = THREE.MathUtils.degToRad(pitchDegrees);
      cameraOffset.set(0, Math.sin(pitch) * distance, Math.cos(pitch) * distance);
    };
    const aimBoard = () => {
      const board = partyBoard.bounds();
      if (overview) target.copy(board.center);
      else {
        target.copy(partyBoard.focus(room?.board?.turn_seat ?? null)).setY(0);
        // Keep the shot over the island rather than sliding out to sea.
        const reachX = Math.max(0, board.halfW + 3 - frameHalf.x);
        const reachZ = Math.max(0, board.halfD + 3 - frameHalf.y);
        target.x = THREE.MathUtils.clamp(target.x, board.center.x - reachX, board.center.x + reachX);
        target.z = THREE.MathUtils.clamp(target.z, board.center.z - reachZ, board.center.z + reachZ);
      }
    };
    let snapCamera = true;
    recamera = () => {
      snapCamera = true;
    };
    fit();
    const resize = new ResizeObserver(fit);
    resize.observe(canvas.parentElement ?? canvas);

    connection = new PartyConnection({
      welcome: (userId, list, isGuest) => {
        me = userId;
        guest = isGuest;
        roster = list;
      },
      room: (view, watching) => {
        const previous = room ? $state.snapshot(room) : null;
        room = view;
        spectating = watching;
        phaseChanged(previous as RoomView | null, view);
      },
      events: (events: GameEvent[]) => {
        const playing = bout;
        // Effects wait for the interpolated view to reach the moment they happened.
        setTimeout(() => {
          if (!playing || playing !== bout || !lastView) return;
          for (const event of events) playing.renderer.effect?.(event, lastView);
        }, 100);
      },
      table: (game, view) => {
        if (game === 'coven') covenView = view as CovenView;
        else if (game === 'landlord') landlordView = view as LandlordView;
      },
      error: showError,
      status: (next) => (status = next),
      passExpired: refreshPass,
    });
    if (pass) connection.setPass(pass.ws_url, pass.token);
    const passFallback = setTimeout(() => {
      if (!pass) refreshPass();
    }, 4000);

    let frame = 0;
    let last = performance.now();
    let lastSent: StickValue = { x: 0, z: 0 };
    let lastSendAt = 0;
    let rosterKey = '';
    let wasUp = false;
    let hudKey = '';

    const loop = (now: number) => {
      frame = requestAnimationFrame(loop);
      const dt = Math.min(0.05, (now - last) / 1000);
      last = now;
      const time = now / 1000;
      if (!connection) return;

      if (rosterKey !== roster.map((c) => c.id).join()) {
        rosterKey = roster.map((c) => c.id).join();
        rosterMap = new Map(roster.map((c) => [c.id, c]));
      }
      const members = room?.members ?? [];
      const mine = members.find((m) => m.user_id === me) ?? null;
      const phase = room?.phase.name ?? 'lobby';
      const boardPhase = phase === 'board' || phase === 'vote' || phase === 'game_over';
      if (boardPhase !== onBoard) {
        onBoard = boardPhase;
        overview = boardPhase && phase === 'game_over';
        recamera?.();
      }
      const tableKey = (room?.table_games ?? []).some(([key]) => key === room?.minigame) ? (room?.minigame ?? '') : '';
      const tableGame = tableKey !== '';
      if (tableGame !== atTable) {
        atTable = tableGame;
        recamera?.();
      }
      covenTable.visible = tableKey === 'coven';
      landlordTable.visible = tableKey === 'landlord';
      if (tableKey === 'coven') {
        covenTable.sync(covenView, members, rosterMap);
        covenTable.update(time, dt);
      } else if (tableKey === 'landlord') {
        landlordTable.sync(landlordView, members, rosterMap);
        landlordTable.update(time);
      }
      if (room?.phase.name === 'lobby') {
        if (covenView) covenView = null;
        if (landlordView) landlordView = null;
      }
      partyBoard.visible = boardPhase && !tableGame;
      const aiming = boardPhase && aimItem && room?.board ? aimItem : null;
      partyBoard.setAim(
        aiming && room?.board
          ? room.board.aim.map((mark) => ({
              seat: mark.seat,
              spaces: mark.spaces,
              reachable: mark.spaces <= (room?.board?.reach.find((r) => r.item === aiming)?.spaces ?? 255),
            }))
          : [],
      );
      if (room?.board) {
        partyBoard.sync(room.board, members, rosterMap);
        partyBoard.update(time, dt);
      }

      const spec = room?.bouts.find((b) => b.key === room?.minigame) ?? null;
      swapBout(spec && !boardPhase ? spec.key : '');
      controlsKind = spec?.controls ?? 'move';
      const fpBout = controlsKind === 'first_person';
      const playing = phase === 'playing';
      const inArena = playing || phase === 'countdown';
      const meInRound = !!mine?.in_round && !spectating;
      controls.enabled = !!bout && fpBout && meInRound && inArena;
      if (fpBout) controls.readGamepad(dt);
      if (input) input.enabled = meInRound && inArena;
      const stick = input?.enabled ? input.read() : { x: 0, z: 0 };

      let view: BoutFrame<unknown, unknown> | null = null;
      if (bout) {
        const sampled = connection.sample(bout.module);
        const snapshot = sampled && sampled.game === bout.key ? sampled : null;
        view = {
          key: bout.key,
          snapshot,
          setup: room?.setup ?? null,
          phase: (inArena || phase === 'results' ? phase : 'lobby') as BoutPhase,
          seats: members,
          mySeat: mine?.seat ?? null,
          winners: room?.phase.name === 'results' ? (room.phase.placements[0] ?? []) : [],
          playing: meInRound,
          stick,
          look: { yaw: controls.yaw, pitch: controls.pitch, jump: controls.jump, crouch: controls.crouch },
          time,
          dt,
        };
        if (fpBout) {
          const facing = bout.renderer.facing?.(view) ?? null;
          // Coming back in points you at the game, not at a wall.
          if (facing !== null && !wasUp) {
            controls.yaw = facing;
            controls.pitch = 0;
            view.look = { ...view.look, yaw: facing, pitch: 0 };
          }
          wasUp = facing !== null;
        }
        bout.renderer.frame(view);
        const nextHud = bout.renderer.hud?.(view) ?? null;
        const nextHudKey = JSON.stringify(nextHud);
        if (nextHudKey !== hudKey) {
          hudKey = nextHudKey;
          hud = nextHud;
        }
        if (bout.renderer.aim) {
          aimRay.setFromCamera(pointer, camera);
          if (aimRay.ray.intersectPlane(groundPlane, aimPoint)) {
            aimYaw = bout.renderer.aim({ x: aimPoint.x, z: aimPoint.z }, view) ?? aimYaw;
          }
        }
        const clock = snapshot as { time?: number; limit?: number } | null;
        const nextTime = playing && clock?.limit !== undefined ? Math.max(0, Math.ceil(clock.limit - (clock.time ?? 0))) : null;
        if (nextTime !== timeLeft) timeLeft = nextTime;
      } else if (timeLeft !== null) {
        timeLeft = null;
      }
      lastView = view;
      if (scene.fog instanceof THREE.Fog) {
        const [near, far] = bout?.renderer.fog ?? [45, 120];
        scene.fog.near = near;
        scene.fog.far = far;
      }
      stage.update(time);

      const portrait = camera.aspect < 0.8;
      const shot = bout && view && !atTable ? bout.renderer.shot(view) : null;
      const shake = bout?.renderer.shake?.() ?? 0;
      if (shot?.kind === 'eye') {
        camera.position.set(shot.x, shot.y, shot.z);
        camera.rotation.set(shot.pitch, shot.yaw, 0, 'YXZ');
        camera.position.x += (Math.random() - 0.5) * shake;
        camera.position.y += (Math.random() - 0.5) * shake;
      } else {
        if (atTable) {
          frameBox(10.5, 10.5, portrait ? 60 : 44);
          lookHeight = 0.6;
          target.set(0, 0, 0);
        } else if (onBoard) {
          const board = partyBoard.bounds();
          if (overview) frameBox(board.halfW + 1, board.halfD + 1, portrait ? 66 : 58);
          else frameBox(13, 10, portrait ? 60 : 50);
          lookHeight = 0;
          aimBoard();
        } else if (shot) {
          frameBox(shot.half[0], shot.half[1], portrait ? (shot.portraitPitch ?? shot.pitch) : shot.pitch);
          lookHeight = shot.lookHeight ?? (portrait ? -0.6 : -1.6);
          target.set(shot.target?.[0] ?? 0, 0, shot.target?.[1] ?? 0);
        } else {
          frameBox(11.5, 9.5, portrait ? 52 : 38);
          lookHeight = portrait ? -0.6 : -1.6;
          target.set(0, 0, 0);
        }
        if (snapCamera) follow.copy(target);
        else follow.lerp(target, 1 - Math.exp(-dt * 3.4));
        snapCamera = false;
        camera.rotation.set(0, 0, 0);
        if (atTable) {
          // Your own seat sits at the bottom of the screen, whichever it is.
          const seatedPeople = [...members].sort((a, b) => a.seat - b.seat);
          const tableView = room?.minigame === 'landlord' ? landlordView : covenView;
          const mySeat = tableView ? tableView.me : seatedPeople.findIndex((m) => m.user_id === me);
          const table = room?.minigame === 'landlord' ? landlordTable : covenTable;
          const yaw = table.seatAngle(mySeat === null || mySeat < 0 ? null : mySeat);
          camera.position.copy(follow).add(cameraOffset.clone().applyAxisAngle(new THREE.Vector3(0, 1, 0), yaw));
        } else {
          camera.position.copy(follow).add(cameraOffset);
        }
        camera.position.x += (Math.random() - 0.5) * shake;
        camera.position.y += (Math.random() - 0.5) * shake;
        camera.position.z += (Math.random() - 0.5) * shake;
        camera.lookAt(follow.x, lookHeight, follow.z);
      }
      manager.render();

      if (!input || !bout) return;
      if (fpBout) {
        // Where you are looking changes constantly, so send at a steady rate.
        if (now - lastSendAt > 45) {
          connection.send({ type: 'input', x: stick.x, z: stick.z, yaw: controls.yaw, pitch: controls.pitch, jump: controls.jump, crouch: controls.crouch });
          lastSent = stick;
          lastSendAt = now;
        }
        if (controls.takeShot(now)) {
          connection.send({ type: 'fire', yaw: controls.yaw, pitch: controls.pitch });
        }
        return;
      }
      if (actionPressed) {
        actionPressed = false;
        if (input.enabled) connection.send({ type: 'fire', yaw: aimYaw, pitch: 0 });
      }
      const changed = Math.abs(stick.x - lastSent.x) + Math.abs(stick.z - lastSent.z) > 0.04;
      const moving = stick.x !== 0 || stick.z !== 0;
      // A turret keeps turning even when the driver does not.
      const turning = !!bout.renderer.aim && input.enabled && now - lastSendAt > 60;
      if ((changed && now - lastSendAt > 33) || (moving && now - lastSendAt > 250) || turning) {
        connection.send({ type: 'input', x: stick.x, z: stick.z, yaw: aimYaw });
        lastSent = stick;
        lastSendAt = now;
      }
    };
    frame = requestAnimationFrame(loop);

    return () => {
      stopped = true;
      cancelAnimationFrame(frame);
      document.removeEventListener('fullscreenchange', onFullscreenChange);
      document.removeEventListener('keydown', onEscape);
      resizeRoot = null;
      viewport.destroy();
      clearTimeout(passFallback);
      if (errorTimer) clearTimeout(errorTimer);
      if (goTimer) clearTimeout(goTimer);
      resize.disconnect();
      canvas.removeEventListener('mousedown', askAction);
      canvas.removeEventListener('click', onTableClick);
      canvas.removeEventListener('mousemove', onPointerMove);
      window.removeEventListener('keydown', onActionKey);
      recamera = null;
      connection?.close();
      connection = null;
      input?.dispose();
      sfx?.dispose();
      bout?.renderer.dispose();
      bout = null;
      controls.dispose();
      partyBoard.dispose();
      covenTable.dispose();
      landlordTable.dispose();
      stage.dispose();
      manager.dispose();
    };
  });
</script>

<div
  bind:this={root}
  class="party-root w-full overflow-hidden bg-sky-300 {expanded ? 'is-expanded fixed inset-0 z-[100]' : 'relative'}"
>
  <canvas bind:this={canvas} class="block h-full w-full touch-none" aria-label="Tawkathon arena"></canvas>
  {#if hud?.award}
    {#key hud.award.id}
      <div class="target-award pointer-events-none absolute left-1/2 top-[38%] z-30 -translate-x-1/2 text-center" aria-live="polite">
        <div class="text-5xl font-black tabular-nums text-amber-300 drop-shadow-[0_4px_0_rgba(15,23,42,0.9)]">+{hud.award.points}</div>
        <div class="mt-1 rounded-full border border-white/20 bg-slate-950/75 px-3 py-1 text-xs font-bold uppercase tracking-[0.16em] text-emerald-200 backdrop-blur">
          {hud.award.caption}
        </div>
      </div>
    {/key}
  {/if}
  <RoomHud
    {room}
    {roster}
    {me}
    {spectating}
    {status}
    hasPass={!!pass}
    {timeLeft}
    {showGo}
    {muted}
    {filming}
    {guest}
    {touch}
    {overview}
    {toggleOverview}
    {fullscreen}
    {toggleFullscreen}
    {pointerLocked}
    {hud}
    {firstPerson}
    onlook={(dx, dy) => look(dx, dy)}
    onfire={(down) => trigger(down)}
    oncrouch={(down) => duck(down)}
    onaim={(item) => (aimItem = item)}
    {errorMessage}
    {send}
    toggleMute={() => {
      muted = !muted;
      sfx?.setMuted(muted);
    }}
    onstick={(value) => input?.setTouch(value)}
  />
  {#if room?.minigame === 'coven'}
    <CovenHud view={covenView} {room} hit={covenHit} {send} />
  {:else if room?.minigame === 'landlord'}
    <LandlordHud view={landlordView} {room} hit={covenHit} {send} />
  {/if}
</div>

<style>
  /* The lobby page keeps about 10rem of header, tabs and footer around
     the party, and anything taller is cut off at the bottom. */
  .party-root {
    height: calc(100dvh - 10.25rem);
    max-height: 70rem;
    min-height: 26rem;
    contain: layout paint;
  }

  .party-root.is-expanded,
  .party-root:fullscreen {
    width: 100vw;
    height: 100dvh;
    max-height: none;
    min-height: 0;
  }

  @media (max-width: 640px), (max-height: 640px) {
    .party-root {
      height: calc(100dvh - 10.25rem);
      min-height: 0;
    }
  }

  .target-award {
    animation: target-award 1.25s cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }

  @keyframes target-award {
    0% { opacity: 0; transform: translate(-50%, 18px) scale(0.7); }
    16% { opacity: 1; transform: translate(-50%, 0) scale(1.12); }
    72% { opacity: 1; transform: translate(-50%, -8px) scale(1); }
    100% { opacity: 0; transform: translate(-50%, -28px) scale(0.94); }
  }
</style>
