<script lang="ts">
  import {
    Shield,
    Banknote,
    Radio,
    CloudFog,
    Hand,
    Play,
    Eye,
    PlugZap,
    Vault,
    Truck,
    Siren,
    CircleHelp,
    ChevronRight,
    Dices,
    Check,
    X,
    CarFront,
    Footprints,
    Timer,
  } from "@lucide/svelte";
  import Handcuffs from "./cuffs.svelte";
  import Joystick from "../joystick.svelte";
  import type { StickValue } from "../input";
  import type { CityView, Member, ClientMsg } from "../protocol";
  import { isGameplayTyping } from "../typing";
  let {
    city,
    seat,
    members,
    send,
    onhold,
    onstick,
    onsuction,
    touch = false,
    yaw = 0,
  }: {
    city: CityView;
    seat: number | null;
    members: Member[];
    send: (msg: ClientMsg) => void;
    onhold: (held: boolean) => void;
    onstick: (value: StickValue) => void;
    onsuction: (held: boolean) => void;
    touch?: boolean;
    yaw?: number;
  } = $props();
  let help = $state(false);
  let lesson = $state(0);
  const mine = $derived(city.actors.find((a) => a.seat === seat));
  const police = $derived(mine?.side === "police");
  const seconds = $derived(Math.ceil(city.remaining));
  const briefing = $derived(city.briefing);
  const arrestProgress = $derived(
    police
      ? Math.max(
          0,
          ...city.actors
            .filter((a) => a.side === "crew" && a.x != null)
            .map((a) => a.cuffed),
        )
      : (mine?.cuffed ?? 0),
  );
  const target = $derived(
    city.targets.find((t) =>
      mine?.carrying ? t.label === "ESCAPE VAN" : t.label.startsWith("JAIL"),
    ) ??
      city.targets.find((t) =>
        city.power_seconds && city.power_seconds > 0
          ? t.label === "BANK VAULT"
          : t.label === "POWER SWITCH",
      ) ??
      city.targets[0],
  );
  const distance = $derived(
    target && mine?.x != null && mine?.z != null
      ? Math.hypot(target.x - mine.x, target.z - mine.z)
      : null,
  );
  const bearing = $derived(
    target && mine?.x != null && mine?.z != null
      ? yaw - Math.atan2(target.x - mine.x, target.z - mine.z) + Math.PI
      : 0,
  );
  const carDistance = $derived(
    mine?.car && mine.x != null && mine.z != null
      ? Math.hypot(mine.x - mine.car[0], mine.z - mine.car[1])
      : Infinity,
  );
  const strength = $derived(
    !city.scanner || city.scanner.startsWith("No")
      ? 0
      : city.scanner.startsWith("Strong")
        ? 3
        : 2,
  );
  const hint = $derived(
    mine?.jailed
      ? "Teammates can rescue you. Bail is counting down."
      : city.job_wait > 0
        ? "Regroup. The next bank job opens shortly."
        : police
          ? mine?.driving
            ? "Follow the blue street arrows. F to get out."
            : "Hold E close to a robber. Use 2 to scan."
          : mine?.carrying
            ? "Take the bag to your van. Hold E to deliver."
            : city.power_seconds && city.power_seconds > 0
              ? "Power is off. Hold E or right-click at the vault."
              : "Hold E at the power switch. Work with your crew.",
  );
  const lessons = $derived(
    police
      ? [
          {
            title: "Patrol the city",
            text: "You are the police. Drive the streets and look for the crew. The bank alarm and scanner give clues, not exact locations.",
            icon: CarFront,
          },
          {
            title: "Park. Pursue. Arrest.",
            text: "WASD drives. Press F to get out, then hold E close to a robber to cuff them. Walk back to your car and press F to drive again.",
            icon: Handcuffs,
          },
          {
            title: "Hold the city for six minutes",
            text: "Recover dropped bags or catch the whole crew to stop a job and score. Jobs keep coming. The night ends with a minigame.",
            icon: Timer,
          },
        ]
      : [
          {
            title: "You are the robber crew",
            text: "Work together in the same city. Cut the power, open the bank vault, then carry the bag to your private escape van.",
            icon: Banknote,
          },
          {
            title: "Follow the green objective arrow",
            text: "WASD moves. Hold E at the switch, vault, and van. One teammate can hold the power while another opens the vault. Right-click also vacuums the vault.",
            icon: PlugZap,
          },
          {
            title: "Keep the bags coming",
            text: "Each delivered bag scores. Use 1 for smoke and 2 to scan. Hold E at jail to rescue a teammate; bail releases them after 18 seconds. Play six minutes, then a minigame.",
            icon: Truck,
          },
        ],
  );
  const icon = (label: string) =>
    label.startsWith("POWER")
      ? PlugZap
      : label.startsWith("BANK VAULT")
        ? Vault
        : label.startsWith("ESCAPE")
          ? Truck
          : label.startsWith("JAIL")
            ? Handcuffs
            : label.startsWith("PATROL")
              ? CarFront
              : Siren;
  const name = (label: string) =>
    label === "POWER SWITCH"
      ? "Cut the power"
      : label === "BANK VAULT"
        ? "Open the vault"
        : label === "ESCAPE VAN"
          ? "Deliver the bag"
          : label.startsWith("JAIL")
            ? "Rescue your crew"
            : label === "PATROL CAR"
              ? "Return to your car"
              : label === "PATROL ROUTE"
                ? "Patrol the streets"
                : "Investigate the alarm";
  function key(event: KeyboardEvent) {
    if (
      event.code === "KeyH" &&
      !event.repeat &&
      !isGameplayTyping(event.target)
    ) {
      help = !help;
      event.preventDefault();
    }
  }
</script>

<svelte:window onkeydown={key} />
<div class="city-hud" aria-label="City Heist HUD" class:police>
  <header>
    <div class="role">
      {#if police}<Shield size={20} />{:else}<Banknote size={20} />{/if}<b
        >{police ? "POLICE" : "CREW"}</b
      ><small>NIGHT {city.round}/{city.jobs}</small>
    </div>
    <div
      class="clock"
      class:urgent={seconds < 30}
      aria-label="Heist time remaining"
    >
      <Timer size={15} />{Math.floor(seconds / 60)}:{String(
        seconds % 60,
      ).padStart(2, "0")}
    </div>
    <div
      class="score"
      aria-label={`Jobs stopped ${city.night_score[0]}, bags delivered ${city.night_score[1]}`}
    >
      <span
        ><Shield size={15} /><b>{city.night_score[0]}</b><small>STOPPED</small
        ></span
      ><i></i><span
        ><Banknote size={15} /><b>{city.night_score[1]}</b><small
          >DELIVERED</small
        ></span
      >
    </div>
  </header>
  {#if !briefing && !city.resolved}
    <div class="objective-line" aria-live="polite">
      <strong
        >{mine?.jailed
          ? `Bail in ${Math.ceil(mine.jail_seconds)}s`
          : target
            ? name(target.label)
            : "Search the city"}</strong
      ><span>{hint}</span>
    </div>
  {/if}
  {#if city.ready > 0 && !briefing}<div
      class="countdown"
      aria-label={`Starting in ${Math.ceil(city.ready)}`}
    >
      <strong>{Math.ceil(city.ready)}</strong>
    </div>{/if}
  {#if city.dispatch && !city.resolved}<div class="dispatch" role="status">
      {city.dispatch.split(" · ")[0]}<span
        >Next job · {Math.ceil(city.job_wait)}s</span
      >
    </div>{/if}
  {#if city.alarm && !city.resolved && !briefing}<div
      class="alarm"
      aria-label="Bank robbery alarm"
    >
      <Siren size={20} /><span>BANK ALARM</span>
    </div>{/if}
  {#if target && distance !== null && distance > 3 && !city.resolved && !mine?.jailed && !briefing && !city.job_wait}
    <div
      class="direction"
      aria-label={`${target.label}, ${Math.ceil(distance)} metres`}
    >
      <svg
        viewBox="0 0 80 90"
        style={`transform:rotate(${bearing}rad)`}
        aria-label="Objective direction"
        ><path
          d="M40 6 73 40H55V79H25V40H7Z"
          fill={police ? "#7acfff" : "#baff52"}
          stroke="#102131"
          stroke-width="5"
          stroke-linejoin="round"
        /></svg
      ><span>{Math.ceil(distance)}<small>m</small></span>
    </div>
  {/if}
  <section class="map-panel" aria-label="City map">
    <svg
      viewBox="-72 -72 144 144"
      aria-label="Streets, visible players and your objective"
    >
      <rect x="-71" y="-71" width="142" height="142" rx="9" fill="#08121fea" />
      {#each [-64, -40, -16, 16, 40, 64] as at}<path
          d={`M${at} -69V69M-69 ${at}H69`}
          stroke="#344454"
          stroke-width="3"
        />{/each}
      {#if police && city.patrol_route.length > 1}<polyline
          points={city.patrol_route.map((p) => p.join(",")).join(" ")}
          fill="none"
          stroke="#7acfff"
          stroke-width="1.5"
        />{/if}
      {#if target}<circle
          cx={target.x}
          cy={target.z}
          r="4.5"
          fill="none"
          stroke={police ? "#7acfff" : "#baff52"}
          stroke-width="1.5"
        />{/if}
      {#if mine?.car && !mine.driving}<rect
          x={mine.car[0] - 2}
          y={mine.car[1] - 3}
          width="4"
          height="6"
          rx="1"
          fill="#80cfff"
        />{/if}
      {#each city.actors.filter((a) => a.x != null && a.z != null && !a.jailed) as actor (actor.seat)}<circle
          cx={actor.x!}
          cy={actor.z!}
          r={actor.seat === seat ? 3 : 2}
          fill={actor.side === "police" ? "#7acfff" : "#ffce7b"}
          stroke={actor.seat === seat ? "white" : "#07111c"}
          stroke-width="1.1"
        />{/each}
    </svg>
    <div class="legend">
      <i></i>You <span></span>{#if target}<b>{name(target.label)}</b>{/if}
    </div>
  </section>
  <section class="objectives" aria-label="Crew objectives">
    {#each city.targets as point (point.label)}{@const Icon = icon(point.label)}
      <div
        class="goal"
        class:active={point === target}
        title={point.label}
        aria-label={`${point.label}, ${Math.round(point.progress * 100)} percent`}
      >
        <Icon size={20} /><progress value={point.progress} max="1"></progress>
      </div>
    {/each}
    {#if arrestProgress > 0}<div
        class="arrest"
        aria-label={police ? "Arresting" : "Being arrested"}
      >
        <Handcuffs size={20} /><progress value={arrestProgress} max="1"
        ></progress>
      </div>{/if}
  </section>
  {#if touch && !mine?.jailed && !mine?.automated && !city.resolved}<div
      class="city-stick"
    >
      <Joystick onchange={onstick} />
    </div>{/if}
  <aside class="tools">
    {#if city.scanner}<div class="scanner" aria-label={city.scanner}>
        <Radio size={18} />{#if city.scanner_bearing != null}<svg
            viewBox="0 0 24 24"
            width="20"
            height="20"
            style={`transform:rotate(${yaw - city.scanner_bearing + Math.PI}rad)`}
            aria-label="Coarse scanner direction"
            ><path d="m12 2 8 10h-5v10H9V12H4Z" fill="currentColor" /></svg
          >{/if}<span
          >{#each [1, 2, 3] as n}<i
              class:lit={strength >= n}
              style={`height:${6 + n * 4}px`}
            ></i>{/each}</span
        >
      </div>{/if}
    {#if police}<button
        class="wide vehicle"
        disabled={!mine ||
          mine.automated ||
          mine.jailed ||
          !!city.resolved ||
          (!mine.driving && carDistance > 4)}
        aria-label={mine?.driving ? "Exit patrol car" : "Enter patrol car"}
        onclick={() => send({ type: "city_vehicle" })}
        ><CarFront size={20} /><span>{mine?.driving ? "Get out" : "Drive"}</span
        ><kbd>F</kbd></button
      >{/if}
    <button
      class="wide"
      disabled={!mine ||
        mine.jailed ||
        mine.automated ||
        mine.driving ||
        !!city.resolved ||
        !!briefing}
      aria-label="Hold to interact"
      onpointerdown={(event) => {
        event.currentTarget.setPointerCapture(event.pointerId);
        onhold(true);
      }}
      onpointerup={() => onhold(false)}
      onpointercancel={() => onhold(false)}
      ><Hand size={20} /><span
        >{police ? "Arrest / recover" : "Hold to use"}</span
      ><kbd>E</kbd></button
    >
    <button
      disabled={!city.tools[0] || mine?.jailed || !!city.resolved || !!briefing}
      aria-label="Smoke bomb"
      title="1 · Smoke bomb"
      onclick={() => send({ type: "city_tool", tool: "smoke" })}
      ><CloudFog size={21} /><kbd>1</kbd><b>{city.tools[0]}</b></button
    >
    <button
      disabled={!city.tools[1] || mine?.jailed || !!city.resolved || !!briefing}
      aria-label="Police scanner"
      title="2 · Police scanner"
      onclick={() => send({ type: "city_tool", tool: "scanner" })}
      ><Radio size={21} /><kbd>2</kbd><b>{city.tools[1]}</b></button
    >
    {#if touch}<button
        class="wide"
        aria-label="Hold to vacuum"
        onpointerdown={(event) => {
          event.currentTarget.setPointerCapture(event.pointerId);
          onsuction(true);
        }}
        onpointerup={() => onsuction(false)}
        onpointercancel={() => onsuction(false)}
        ><CloudFog size={21} /><span>Vacuum</span></button
      >{/if}
    <button
      class="subtle"
      aria-label={mine?.automated ? "Take control" : "Let bot play my seat"}
      onclick={() => send({ type: "city_automate", on: !mine?.automated })}
      >{#if mine?.automated}<Play size={18} />{:else}<Eye
          size={18}
        />{/if}</button
    >
    <button
      class="subtle"
      aria-label="Controls and objective"
      title="H · Help"
      onclick={() => (help = !help)}><CircleHelp size={18} /></button
    >
  </aside>
  <div class="team" aria-label="City teams">
    {#each city.actors as actor (actor.seat)}{@const label =
        members.find((m) => m.seat === actor.seat)?.name ?? "Player"}<span
        class:cop={actor.side === "police"}
        class:jailed={actor.jailed}
        class:you={actor.seat === seat}
        title={`${label} · ${actor.side}`}
        aria-label={`${label}, ${actor.side}${actor.jailed ? ", jailed" : ""}`}
        >{#if actor.jailed}<Handcuffs
            size={14}
          />{:else if actor.carrying}<Banknote
            size={14}
          />{:else if actor.side === "police"}<Shield size={14} />{:else}<i
          ></i>{/if}<small>{label}</small></span
      >{/each}
  </div>
  {#if city.resolved}<div
      class="resolved"
      class:won={city.draw || city.resolved === mine?.side}
    >
      {#if city.resolved === "crew"}<Banknote size={40} />{:else}<Shield
          size={40}
        />{/if}<strong
        >{city.draw
          ? "NIGHT TIED"
          : city.resolved === mine?.side
            ? "NIGHT WON"
            : "NIGHT LOST"}</strong
      ><span
        >{city.night_score[0]} stopped · {city.night_score[1]} delivered</span
      >
      <div>
        <ChevronRight size={18} /><Dices size={24} /><b>Minigame next</b>
      </div>
    </div>{/if}
  {#if briefing && mine && !mine.automated}
    <section
      class="briefing"
      aria-label="Opening night briefing"
      aria-live="polite"
    >
      <div class="briefing-top">
        <span>NIGHT ONE · {police ? "POLICE" : "CREW"}</span><small
          >{Math.ceil(briefing.remaining)}s</small
        >
      </div>
      {#if briefing.you_ready}<Check size={40} />
        <h2>Ready for the night</h2>
        <p>Waiting for the other players. You can try moving while you wait.</p>
      {:else}{@const LessonIcon = lessons[lesson].icon}<LessonIcon size={40} />
        <h2>{lessons[lesson].title}</h2>
        <p>{lessons[lesson].text}</p>
        <div class="lesson-dots" aria-label={`Lesson ${lesson + 1} of 3`}>
          {#each [0, 1, 2] as n}<span class:current={lesson === n}
            ></span>{/each}
        </div>{/if}
      <div class="briefing-controls">
        <span
          ><kbd>W</kbd><kbd>A</kbd><kbd>S</kbd><kbd>D</kbd>
          {police ? "drive / move" : "move"}</span
        ><span><kbd>E</kbd> hold to use</span>{#if police}<span
            ><kbd>F</kbd> enter / exit</span
          >{/if}
      </div>
      <footer>
        <small
          >{briefing.ready}/{briefing.total} players ready<br />The six-minute
          clock starts after this.</small
        >{#if !briefing.you_ready}<button
            onclick={() => {
              if (lesson < 2) lesson += 1;
              else send({ type: "city_ready" });
            }}
            >{lesson < 2 ? "Got it" : "Ready for the night"}<ChevronRight
              size={17}
            /></button
          >{/if}
      </footer>
    </section>
  {/if}
  {#if help}<section class="help">
      <button
        class="close"
        aria-label="Close help"
        onclick={() => (help = false)}><X size={20} /></button
      ><strong
        >{police ? "Patrol. Pursue. Protect." : "Power. Vault. Van."}</strong
      >
      <p>{city.objective}</p>
      <dl>
        <dt>WASD / left stick</dt>
        <dd>{mine?.driving ? "Drive" : "Move"}</dd>
        <dt>Middle mouse / right stick</dt>
        <dd>Camera</dd>
        <dt>F / vehicle button</dt>
        <dd>Enter or exit your patrol car</dd>
        <dt>Hold E / A</dt>
        <dd>
          {police
            ? "Arrest on foot, recover bags"
            : "Use stations, rescue, deliver"}
        </dd>
        <dt>Hold right mouse / RT</dt>
        <dd>Vacuum the vault or bag</dd>
        <dt>1 / 2</dt>
        <dd>Smoke / scanner</dd>
      </dl>
      <small
        >Play the full six-minute night. Delivered bags and stopped jobs score;
        the higher total wins. Ties are draws. Then everyone plays a minigame
        for extra gear.</small
      >
    </section>{/if}
</div>

<style>
  .city-hud {
    --side: #ffce7b;
    position: absolute;
    inset: 0;
    z-index: 38;
    pointer-events: none;
    color: #f3f8ff;
    user-select: none;
    font-family: inherit;
    text-shadow: 0 2px 4px #0007;
  }
  .city-hud.police {
    --side: #7acfff;
  }
  header {
    position: absolute;
    top: 4.2rem;
    left: 1rem;
    right: 1rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.65rem 1rem;
    background: linear-gradient(90deg, #08131eef, #0d1923d9);
    border-radius: 7px;
    border-bottom: 1px solid #ffffff18;
  }
  .role {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--side);
  }
  .role b {
    font-size: 0.75rem;
    letter-spacing: 0.09em;
  }
  .role small {
    color: #98acbf;
    font-size: 0.6rem;
    margin-left: 0.4rem;
  }
  .clock {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 1.45rem;
    font-weight: 900;
    font-variant-numeric: tabular-nums;
  }
  .clock.urgent {
    color: #ff879a;
  }
  .clock :global(svg) {
    opacity: 0.6;
  }
  .score,
  .score > span {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .score > span:first-child {
    color: #7acfff;
  }
  .score > span:last-child {
    color: #ffce7b;
  }
  .score small {
    font-size: 0.5rem;
    letter-spacing: 0.07em;
    color: #a8bacb;
  }
  .score b {
    font-size: 1rem;
    font-variant-numeric: tabular-nums;
  }
  .score i {
    width: 1px;
    height: 20px;
    background: #ffffff25;
    margin: 0 0.4rem;
  }
  .objective-line {
    position: absolute;
    left: 1.2rem;
    top: 8.1rem;
    display: grid;
    gap: 0.35rem;
    max-width: 24rem;
  }
  .objective-line strong {
    font-size: 1rem;
    color: var(--side);
  }
  .objective-line span {
    font-size: 0.73rem;
    color: #c3d0dd;
    line-height: 1.4;
  }
  .direction {
    position: absolute;
    left: 50%;
    top: 8.2rem;
    transform: translateX(-50%);
    display: grid;
    justify-items: center;
    filter: drop-shadow(0 3px 3px #0009);
  }
  .direction > svg {
    width: 44px;
    height: 51px;
  }
  .direction > span {
    display: flex;
    gap: 0.25rem;
    font-size: 0.7rem;
    font-weight: 900;
    background: #08131ee0;
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
  }
  .direction small {
    color: #b6c6d5;
  }
  .alarm {
    position: absolute;
    right: 1.4rem;
    top: 8.2rem;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    color: #ff93a5;
    font-size: 0.65rem;
    font-weight: 900;
  }
  .dispatch {
    position: absolute;
    top: 25%;
    left: 50%;
    transform: translateX(-50%);
    background: #0b1b29ee;
    border: 1px solid #86bad347;
    border-radius: 8px;
    padding: 0.8rem 1.3rem;
    font-size: 0.9rem;
    text-align: center;
    display: grid;
    gap: 0.4rem;
  }
  .dispatch span {
    font-size: 0.7rem;
    color: #bdd0dd;
  }
  .map-panel {
    position: absolute;
    left: 1rem;
    bottom: 4.9rem;
    width: 142px;
  }
  .map-panel > svg {
    display: block;
    width: 100%;
    border: 1px solid #ffffff20;
    border-radius: 10px;
  }
  .legend {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.55rem;
    color: #bccddd;
    margin-top: 0.35rem;
    white-space: nowrap;
  }
  .legend i {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--side);
    border: 1px solid white;
  }
  .legend span {
    width: 5px;
  }
  .legend b {
    font-weight: 500;
    color: var(--side);
  }
  .objectives {
    position: absolute;
    bottom: 3rem;
    left: 1rem;
    display: flex;
    gap: 0.55rem;
  }
  .goal,
  .arrest {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    color: #c0d0dd;
    background: #0a1724df;
    padding: 0.4rem 0.5rem;
    border-radius: 5px;
  }
  .goal.active {
    color: #baff83;
  }
  .goal progress,
  .arrest progress {
    width: 45px;
    height: 4px;
    accent-color: #baff83;
  }
  .arrest {
    color: #ff93a5;
  }
  .arrest progress {
    accent-color: #ff93a5;
  }
  .tools {
    position: absolute;
    right: 1rem;
    bottom: 3rem;
    width: 165px;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.35rem;
  }
  .tools button {
    position: relative;
    pointer-events: auto;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    min-height: 43px;
    padding: 0.5rem;
    border: 1px solid #6987a347;
    border-radius: 5px;
    background: #0b1a2aeb;
    color: #deebf6;
    cursor: pointer;
    touch-action: none;
  }
  .tools button:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .tools .wide {
    grid-column: span 2;
  }
  .tools .wide span {
    font-size: 0.65rem;
  }
  .tools .vehicle {
    border-color: #7acfff60;
    color: #a6dcff;
  }
  .tools kbd {
    font: inherit;
    font-size: 0.55rem;
    color: #a5b8ca;
    padding: 0.1rem 0.2rem;
    border: 1px solid #ffffff30;
    border-bottom-width: 2px;
    border-radius: 3px;
  }
  .tools button > b {
    font-size: 0.6rem;
    position: absolute;
    right: 0.4rem;
    bottom: 0.3rem;
  }
  .tools button.subtle {
    min-height: 29px;
    padding: 0.3rem;
    color: #9eb4c8;
    background: #09131dc9;
  }
  .scanner {
    grid-column: span 2;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.6rem;
    padding: 0.45rem;
    background: #0a342ae6;
    color: #adffd2;
    border-radius: 5px;
  }
  .scanner > span {
    display: flex;
    gap: 3px;
    align-items: flex-end;
    height: 18px;
  }
  .scanner i {
    width: 5px;
    background: #315a4c;
    border-radius: 1px;
  }
  .scanner i.lit {
    background: #9affca;
  }
  .team {
    position: absolute;
    bottom: 0.55rem;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    gap: 0.3rem;
  }
  .team > span {
    display: flex;
    gap: 0.3rem;
    align-items: center;
    padding: 0.3rem 0.45rem;
    background: #231e16e8;
    border-bottom: 2px solid #ffc775;
    border-radius: 3px;
    color: #ffd075;
  }
  .team small {
    font-size: 0.55rem;
    max-width: 60px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .team > span.cop {
    background: #102538e8;
    color: #7acfff;
    border-color: #7acfff;
  }
  .team > span.you {
    box-shadow: 0 0 0 1px #ffffff80 inset;
  }
  .team > span > i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: currentColor;
  }
  .team > span.jailed {
    opacity: 0.5;
  }
  .countdown,
  .resolved {
    position: absolute;
    top: 43%;
    left: 50%;
    transform: translate(-50%, -50%);
    text-align: center;
  }
  .countdown strong {
    font-size: 5rem;
    font-style: italic;
    font-weight: 950;
  }
  .resolved {
    display: grid;
    justify-items: center;
    gap: 0.6rem;
    padding: 1.4rem 2rem;
    background: #07111ef5;
    border-top: 3px solid #ff8196;
    border-radius: 7px;
    color: #ffb2bb;
  }
  .resolved.won {
    color: #c0ff8a;
    border-color: #aaff6e;
  }
  .resolved strong {
    font-size: 1.9rem;
    font-weight: 950;
  }
  .resolved > span {
    color: #d4e1ef;
    font-size: 0.75rem;
  }
  .resolved > div {
    display: flex;
    gap: 0.45rem;
    align-items: center;
    color: #d4e1ef;
    margin-top: 0.3rem;
    font-size: 0.85rem;
  }
  .briefing,
  .help {
    pointer-events: auto;
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    width: min(31rem, calc(100% - 2rem));
    padding: 1.4rem 1.6rem;
    background: linear-gradient(145deg, #122535fa, #07111cfa);
    border: 1px solid #809eb34d;
    border-top: 3px solid var(--side);
    border-radius: 12px;
    box-shadow: 0 20px 100px #0009;
    backdrop-filter: blur(10px);
  }
  .briefing-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    color: var(--side);
    font-size: 0.6rem;
    font-weight: 800;
    letter-spacing: 0.1em;
    margin-bottom: 1.4rem;
  }
  .briefing-top small {
    color: #b6c9d9;
    font-size: 0.7rem;
    font-variant-numeric: tabular-nums;
  }
  .briefing > :global(svg) {
    color: var(--side);
    margin-bottom: 0.4rem;
  }
  .briefing h2 {
    font-size: 1.5rem;
    font-weight: 850;
    line-height: 1.15;
    margin: 0.5rem 0 0.8rem;
  }
  .briefing p {
    font-size: 0.85rem;
    line-height: 1.65;
    color: #c2d2df;
    margin: 0;
    min-height: 5.7rem;
  }
  .lesson-dots {
    display: flex;
    gap: 0.35rem;
    margin: 1rem 0;
  }
  .lesson-dots span {
    width: 25px;
    height: 3px;
    background: #ffffff25;
    border-radius: 3px;
  }
  .lesson-dots .current {
    background: var(--side);
  }
  .briefing-controls {
    display: flex;
    flex-wrap: wrap;
    gap: 0.6rem;
    margin-top: 1.2rem;
    padding-top: 1rem;
    border-top: 1px solid #ffffff18;
    color: #8fa9bd;
    font-size: 0.6rem;
  }
  .briefing-controls > span {
    display: flex;
    align-items: center;
    gap: 0.2rem;
  }
  .briefing-controls kbd {
    display: inline-grid;
    place-items: center;
    min-width: 19px;
    height: 21px;
    background: #ffffff0b;
    border: 1px solid #ffffff2b;
    border-bottom-width: 2px;
    border-radius: 3px;
    color: #dbe7ef;
    font-size: 0.6rem;
  }
  footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.8rem;
    margin-top: 1.3rem;
  }
  footer small {
    font-size: 0.6rem;
    line-height: 1.6;
    color: #9ab1c4;
  }
  footer button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 0.7rem 0.9rem;
    background: var(--side);
    color: #07131d;
    border: 0;
    border-radius: 6px;
    font-size: 0.72rem;
    font-weight: 850;
    cursor: pointer;
    white-space: nowrap;
  }
  .help {
    max-height: calc(100% - 5rem);
    overflow: auto;
  }
  .help > strong {
    font-size: 1.1rem;
    color: var(--side);
  }
  .help .close {
    position: absolute;
    right: 0.6rem;
    top: 0.6rem;
    background: transparent;
    color: white;
    border: 0;
    cursor: pointer;
  }
  .help p {
    font-size: 0.8rem;
    line-height: 1.5;
    color: #c3d4df;
  }
  .help dl {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.6rem;
    font-size: 0.7rem;
  }
  .help dt {
    color: #91acc0;
  }
  .help dd {
    margin: 0;
  }
  .help > small {
    display: block;
    font-size: 0.7rem;
    line-height: 1.6;
    color: #b5c9d8;
    margin-top: 1rem;
  }
  .city-stick {
    pointer-events: auto;
    position: absolute;
    left: 1rem;
    bottom: 10rem;
  }
  @media (max-width: 640px) {
    header {
      top: 3.7rem;
      left: 0.5rem;
      right: 0.5rem;
      padding: 0.5rem;
      gap: 0.4rem;
    }
    .role {
      gap: 0.25rem;
    }
    .role b {
      font-size: 0.6rem;
    }
    .role small {
      display: none;
    }
    .clock {
      font-size: 1.2rem;
    }
    .score {
      gap: 0.2rem;
    }
    .score > span {
      gap: 0.2rem;
    }
    .score small {
      display: none;
    }
    .score i {
      margin: 0 0.15rem;
    }
    .objective-line {
      left: 0.7rem;
      top: 7.5rem;
      max-width: 60%;
    }
    .objective-line strong {
      font-size: 0.85rem;
    }
    .objective-line span {
      font-size: 0.65rem;
    }
    .direction {
      top: 10.6rem;
    }
    .alarm {
      top: 7.5rem;
      right: 0.7rem;
    }
    .alarm span {
      display: none;
    }
    .map-panel {
      width: 105px;
      left: 0.6rem;
      bottom: 5.2rem;
    }
    .legend {
      font-size: 0.48rem;
    }
    .tools {
      right: 0.6rem;
      width: 133px;
    }
    .tools .wide span {
      font-size: 0.6rem;
    }
    .team small {
      display: none;
    }
    .team > span {
      padding: 0.3rem;
    }
    .objectives {
      left: 0.6rem;
      bottom: 3.1rem;
      gap: 0.25rem;
    }
    .goal,
    .arrest {
      padding: 0.3rem;
    }
    .goal progress,
    .arrest progress {
      width: 20px;
    }
    .briefing {
      padding: 1rem 1.1rem;
    }
    .briefing h2 {
      font-size: 1.2rem;
    }
    .briefing p {
      font-size: 0.77rem;
      min-height: 6rem;
    }
    .briefing-top {
      margin-bottom: 0.7rem;
    }
    .briefing-controls {
      margin-top: 0.7rem;
      padding-top: 0.7rem;
    }
    footer {
      margin-top: 0.9rem;
    }
    .city-stick {
      bottom: 13rem;
    }
    .map-panel {
      bottom: 4.8rem;
    }
    .resolved strong {
      font-size: 1.4rem;
    }
  }
  @media (max-height: 540px) {
    .briefing {
      top: 54%;
      padding: 0.7rem 1rem;
    }
    .briefing-top {
      margin-bottom: 0.4rem;
    }
    .briefing > :global(svg) {
      display: none;
    }
    .briefing h2 {
      font-size: 1.1rem;
    }
    .briefing p {
      min-height: 0;
      font-size: 0.7rem;
    }
    .briefing-controls {
      margin-top: 0.5rem;
      padding-top: 0.5rem;
    }
    .lesson-dots {
      margin: 0.6rem 0;
    }
    footer {
      margin-top: 0.6rem;
    }
    .map-panel {
      width: 85px;
    }
    .objective-line {
      top: 7rem;
    }
  }
</style>
