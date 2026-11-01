<script lang="ts">
  import {Coins,Candy,Heart} from '@lucide/svelte';
  import type {BoardView,CharacterDef,Member} from '../protocol';
  import{visibleTeamFor,type VisibleTeam}from'../team-visibility';
  import {boardStandings,placeLabel} from './standings-state';
  let {board,members,roster,me,teams=[]}:{board:BoardView;members:Member[];roster:CharacterDef[];me:string|null;teams?:VisibleTeam[]}=$props();
  const characters=$derived(new Map(roster.map(character=>[character.id,character])));
  const players=$derived(boardStandings(board,members));
</script>

<aside class="hud-keep standings" aria-label="Board standings">
  {#each [0,1,2,3] as corner}
    <div class="standings__corner corner-{corner}">
      {#each players.filter(player=>player.corner===corner) as player (player.member.user_id)}
        {@const member=player.member}
        {@const token=player.token}
        {@const character=characters.get(member.character)}
        {@const team=visibleTeamFor(teams,member.seat)}
        <section class="player-card" class:current={player.current} class:disconnected={!member.connected}
          style="--player-color:{team?.color??character?.body??'#a3e635'}" aria-label="{member.name}{team?` · ${team.label}`:''} · {placeLabel(player.place)} place" aria-current={player.current?'step':undefined}>
          <div class="portrait" aria-hidden="true">
            {#if member.avatar_url}<img src={member.avatar_url} alt="" referrerpolicy="no-referrer"/>{:else}<span>{member.name.slice(0,1).toUpperCase()}</span>{/if}
          </div>
          <div class="player-card__body">
            <div class="player-card__heading"><strong>{member.name}</strong>{#if member.user_id===me}<small>YOU</small>{/if}<b class="place" class:leader={player.place===1}>{placeLabel(player.place)}</b></div>
            {#if team}<div class="player-card__team" style={`--team-color:${team.color}`}><i></i>{team.label}{#if team.seats.some(seat=>members.find(member=>member.seat===seat)?.user_id===me)}<b>YOUR PAIR</b>{/if}</div>{/if}
            <div class="player-card__totals">
              <span class="candy" aria-label="{token.prizes} candy"><Candy/>{token.prizes}</span>
              <span class="coins" aria-label="{token.coins??'Hidden'} coins"><Coins/>{token.coins??'—'}</span>
              <span class="hearts" aria-label="{token.health} health">{#each [1,2,3] as heart}<Heart class={heart<=token.health?'full':'empty'}/>{/each}</span>
            </div>
            <div class="player-card__turn">{#if player.current}<i></i> {member.user_id===me?'YOUR TURN':'PLAYING'}{:else}TURN {player.turn}{/if}{#if token.resting>0}<span>RESTING</span>{/if}</div>
          </div>
        </section>
      {/each}
    </div>
  {/each}
</aside>

<style>
.standings{position:absolute;inset:0;z-index:12;color:#fff;pointer-events:none;font-family:inherit;filter:drop-shadow(0 3px 5px #02061766)}
.standings__corner{position:absolute;display:flex;flex-direction:column;gap:.5rem;width:clamp(10rem,19vw,14rem)}
.corner-0,.corner-2{left:1rem}.corner-1,.corner-3{right:1rem}.corner-0,.corner-1{top:4.5rem}.corner-2,.corner-3{bottom:5.5rem}
.player-card{display:flex;align-items:center;gap:.6rem;min-width:0;padding:.55rem .7rem .45rem .45rem;background:linear-gradient(110deg,#07101fec,#102039ba);border-left:3px solid var(--player-color);clip-path:polygon(0 0,calc(100% - 10px) 0,100% 10px,100% 100%,10px 100%,0 calc(100% - 10px))}
.player-card.current{background:linear-gradient(110deg,#122235f5,#152438eb);box-shadow:inset 0 2px var(--player-color),inset 0 -2px var(--player-color)}.player-card.disconnected{opacity:.5}
.portrait{display:grid;place-items:center;flex-shrink:0;width:2.5rem;height:2.5rem;background:color-mix(in srgb,var(--player-color) 40%,#102039);border:2px solid var(--player-color);border-radius:50%;overflow:hidden}.portrait img{width:100%;height:100%;object-fit:cover}.portrait>span{font-size:1.35rem;font-weight:950;color:#fff;text-shadow:0 2px 0 #0005}
.player-card__body{min-width:0;flex:1}.player-card__heading{display:flex;align-items:center;gap:.3rem;line-height:1.15}.player-card__heading strong{font-size:.68rem;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.player-card__heading small{font-size:.4rem;color:#cbd5e1;letter-spacing:.05em}.place{margin-left:auto;font-size:.95rem;font-style:italic;color:#cbd5e1;white-space:nowrap}.place.leader{color:#fde68a}
.player-card__team{display:flex;align-items:center;gap:.25rem;color:var(--team-color);font-size:.48rem;font-weight:850;line-height:1.2;margin-top:.2rem}.player-card__team>i{width:.3rem;height:.3rem;background:currentColor;border-radius:50%}.player-card__team>b{margin-left:auto;font-size:.36rem;color:#f8fafc;letter-spacing:.04em}.player-card__totals{display:flex;gap:.7rem;align-items:center;margin-top:.3rem;font-size:1.05rem;line-height:1;font-weight:950;font-variant-numeric:tabular-nums}.player-card__totals>span{display:flex;align-items:center;gap:.22rem}.player-card__totals :global(svg){width:.92rem;height:.92rem}.candy{color:#f9a8d4}.coins{color:#fde68a}.hearts{margin-left:auto;gap:.04rem!important}.hearts :global(svg){width:.5rem;height:.55rem}.hearts :global(.full){fill:#fb7185;color:#fb7185}.hearts :global(.empty){fill:#ffffff15;color:#ffffff30}
.player-card__turn{display:flex;align-items:center;gap:.25rem;margin-top:.32rem;font-size:.42rem;letter-spacing:.1em;color:#ffffff70;font-weight:850}.current .player-card__turn{color:var(--player-color)}.player-card__turn i{width:.25rem;height:.25rem;border-radius:50%;background:var(--player-color)}.player-card__turn>span{margin-left:auto;color:#fda4af}
@media(max-width:640px){.standings__corner{width:calc(50% - 1.2rem);max-width:11rem;gap:.3rem}.corner-0,.corner-2{left:.5rem}.corner-1,.corner-3{right:.5rem}.corner-0,.corner-1{top:6rem}.corner-2,.corner-3{bottom:9rem}.player-card{gap:.35rem;padding:.45rem .45rem .35rem .3rem}.portrait{width:1.9rem;height:1.9rem}.portrait>span{font-size:1rem}.player-card__heading strong{font-size:.59rem}.player-card__heading small{display:none}.place{font-size:.8rem}.player-card__totals{font-size:.9rem;gap:.5rem}.player-card__totals :global(svg){width:.72rem;height:.72rem}.hearts :global(svg){width:.4rem;height:.4rem}.player-card__turn{font-size:.36rem;margin-top:.25rem}}
@media(pointer:coarse){.corner-2,.corner-3{bottom:9rem}}
@media(max-height:520px){.standings__corner{width:10rem;gap:.2rem}.corner-0,.corner-1{top:3.5rem}.corner-2,.corner-3{bottom:4.2rem}.player-card{padding:.25rem .45rem;gap:.35rem}.portrait{width:1.6rem;height:1.6rem}.player-card__turn{display:none}.player-card__totals{margin-top:.15rem;font-size:.85rem}.player-card__heading strong{font-size:.55rem}.place{font-size:.72rem}}
</style>
