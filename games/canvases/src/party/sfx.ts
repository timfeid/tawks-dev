import { watchSettings, type Settings } from './settings';
import { glide } from './audio-param';
import{CityAmbience}from'./city/ambience';

/** Kenney's CC0 audio packs, named `<sound>-<n>.ogg`; see sounds/README.md. */
const FILES = import.meta.glob('./sounds/*.ogg', { query: '?url', import: 'default', eager: true }) as Record<string, string>;
/** Each sound's files: a sound with several plays one of them at random. */
const BANK = new Map<string, string[]>();
for (const [path, url] of Object.entries(FILES)) {
  const name = path.replace(/^.*\//, '').replace(/-\d+\.ogg$/, '');
  BANK.set(name, [...(BANK.get(name) ?? []), url]);
}

/** What a token walks on, for its footsteps. */
export type Ground = 'grass' | 'stone' | 'snow' | 'wood';

/**
 * Party sounds: recorded ones from the sounds folder, with synthesized ones
 * for whatever has no recording, or until the recordings have loaded.
 * Nothing plays until the page gets a tap or key press.
 */
export class PartySfx {
  private ctx: AudioContext | null = null;
  private ambience:CityAmbience|null=null;
  private ambienceOut:GainNode|null=null;
  /** Everything plays into this, which goes out dry and through the cave's echo. */
  private out: GainNode | null = null;
  private echo: GainNode | null = null;
  private buffers = new Map<string, AudioBuffer>();
  private cave = 0;
  private dripIn = 2;
  private volume: Settings['sound'] = { master: 1, effects: 1, jingles: 1, ambience:.7 };
  private stopWatching = watchSettings((next) => {
    this.volume = next.sound;
    if (this.out) this.out.gain.value = (this.muted||this.suspended)?0:next.sound.master * next.sound.effects;
    if(this.ambienceOut)this.ambienceOut.gain.value=(this.muted||this.suspended)?0:next.sound.master;
  });
  muted = false;
  private suspended=false;
  private cityListener={x:0,z:0,yaw:0};
  private engines=new Map<number,{osc:OscillatorNode;harmonic:OscillatorNode;gain:GainNode;pan:StereoPannerNode;distance:number}>();
  private crawlerBreath=0;
  private hornAt=new Map<number,number>();
  private pedestrianSteps=new Map<number,number>();

  private unlock = () => {
    if (!this.ctx) {
      try {
        this.ctx = new AudioContext();
      } catch {
        return;
      }
      this.wire(this.ctx);
      void this.load(this.ctx);
    }
    void this.ctx.resume();
  };

  private wire(ctx: AudioContext) {
    this.out = ctx.createGain();
    this.out.gain.value = (this.muted||this.suspended)?0:this.volume.master * this.volume.effects;
    this.out.connect(ctx.destination);
    this.ambienceOut=ctx.createGain();this.ambienceOut.gain.value=(this.muted||this.suspended)?0:this.volume.master;this.ambienceOut.connect(ctx.destination);this.ambience=new CityAmbience(ctx,this.ambienceOut);
    // A tunnel's echo: a couple of seconds of decaying noise as the room.
    const reverb = ctx.createConvolver();
    const length = Math.floor(ctx.sampleRate * 2.2);
    const room = ctx.createBuffer(2, length, ctx.sampleRate);
    for (let channel = 0; channel < 2; channel++) {
      const data = room.getChannelData(channel);
      for (let i = 0; i < length; i++) data[i] = (Math.random() * 2 - 1) * (1 - i / length) ** 3;
    }
    reverb.buffer = room;
    this.echo = ctx.createGain();
    this.echo.gain.value = 0;
    this.out.connect(reverb).connect(this.echo).connect(ctx.destination);
  }

  private async load(ctx: AudioContext) {
    await Promise.all(
      Object.values(FILES).map(async (url) => {
        try {
          const response = await fetch(url);
          this.buffers.set(url, await ctx.decodeAudioData(await response.arrayBuffer()));
        } catch {
          // A browser that cannot play Ogg keeps the synthesized sounds.
        }
      }),
    );
  }

  /** Plays one of a sound's recordings, a touch off pitch each time; false if it cannot. */
  private sample(name: string, volume: number, rate = 1, delay = 0) {
    const ctx = this.ready();
    const urls = (BANK.get(name) ?? []).filter((url) => this.buffers.has(url));
    if (!ctx || !this.out || !urls.length) return false;
    const source = ctx.createBufferSource();
    source.buffer = this.buffers.get(urls[Math.floor(Math.random() * urls.length)])!;
    source.playbackRate.value = rate * (0.95 + Math.random() * 0.1);
    const gain = ctx.createGain();
    // Jingles have their own level, set apart from the effects.
    gain.gain.value = name.startsWith('jingle') ? (volume * this.volume.jingles) / Math.max(0.05, this.volume.effects) : volume;
    source.connect(gain).connect(this.out);
    source.start(ctx.currentTime + delay);
    return true;
  }

  constructor() {
    try {
      this.muted = localStorage.getItem('party-muted') === '1';
    } catch {
      this.muted = false;
    }
    window.addEventListener('pointerdown', this.unlock);
    window.addEventListener('keydown', this.unlock);
  }

  setMuted(muted: boolean) {
    this.muted = muted;
    if(muted){this.stopEngines();this.ambience?.stop();}
    if(this.ambienceOut)this.ambienceOut.gain.value=(muted||this.suspended)?0:this.volume.master;
    if(this.out)this.out.gain.value=(muted||this.suspended)?0:this.volume.master*this.volume.effects;
    try {
      localStorage.setItem('party-muted', muted ? '1' : '0');
    } catch {
      // Remembering the choice is a convenience only.
    }
  }

  /** Parked parent views stay silent without changing the saved preference. */
  setSuspended(suspended:boolean){
    this.suspended=suspended;if(suspended){this.stopEngines();this.ambience?.stop();}
    if(this.ambienceOut)this.ambienceOut.gain.value=(suspended||this.muted)?0:this.volume.master;
    if(this.out)this.out.gain.value=(suspended||this.muted)?0:this.volume.master*this.volume.effects;
  }

  diagnostics(){return{state:this.ctx?.state??'locked',muted:this.muted,suspended:this.suspended,ambience:this.ambience?.stats()??{decoded:0,loops:0,events:0},engines:this.engines.size};}

  private ready() {
    return !this.muted && !this.suspended && this.ctx && this.ctx.state === 'running' ? this.ctx : null;
  }

  private tone(freqFrom: number, freqTo: number, duration: number, volume: number, type: OscillatorType, delay = 0, pan=0) {
    const ctx = this.ready();
    if (!ctx || volume<=.0001) return;
    const start = ctx.currentTime + delay;
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = type;
    osc.frequency.setValueAtTime(freqFrom, start);
    osc.frequency.exponentialRampToValueAtTime(Math.max(20, freqTo), start + duration);
    gain.gain.setValueAtTime(0.0001, start);
    gain.gain.exponentialRampToValueAtTime(volume, start + 0.012);
    gain.gain.exponentialRampToValueAtTime(0.0001, start + duration);
    const stereo=ctx.createStereoPanner();stereo.pan.value=pan;
    osc.connect(gain).connect(stereo).connect(this.out ?? ctx.destination);
    osc.start(start);
    osc.stop(start + duration + 0.02);
  }

  heistAlarm(){for(let i=0;i<4;i++)this.tone(i%2?760:1050,i%2?1050:760,.35,.07,'triangle',i*.3);}
  bump(strength: number) {
    const s = Math.min(1, strength);
    if (this.sample('punch', 0.3 + s * 0.5, 1.15 - s * 0.25)) return;
    this.tone(620 - s * 180, 150, 0.2 + s * 0.08, 0.12 + s * 0.2, 'sine');
    this.tone(1240 - s * 300, 400, 0.08, 0.04 + s * 0.05, 'triangle');
  }

  fall() {
    this.tone(700, 160, 0.55, 0.14, 'triangle');
  }

  splash() {
    const ctx = this.ready();
    if (!ctx) return;
    const length = Math.floor(ctx.sampleRate * 0.45);
    const buffer = ctx.createBuffer(1, length, ctx.sampleRate);
    const data = buffer.getChannelData(0);
    for (let i = 0; i < length; i++) data[i] = (Math.random() * 2 - 1) * (1 - i / length) ** 2;
    const source = ctx.createBufferSource();
    source.buffer = buffer;
    const filter = ctx.createBiquadFilter();
    filter.type = 'lowpass';
    filter.frequency.value = 1400;
    const gain = ctx.createGain();
    gain.gain.value = 0.35;
    source.connect(filter).connect(gain).connect(this.out ?? ctx.destination);
    source.start();
  }

  shot() {
    this.tone(760, 220, 0.09, 0.1, 'square');
    this.tone(180, 90, 0.12, 0.07, 'triangle');
  }

  splat() {
    if (this.sample('soft', 0.6)) return;
    this.tone(320, 120, 0.13, 0.14, 'sine');
  }

  down(mine: boolean) {
    this.sample('punch-heavy', mine ? 0.8 : 0.45);
    this.tone(mine ? 300 : 660, mine ? 90 : 990, 0.3, 0.1, 'triangle');
  }

  /** A boot on concrete. Volume carries how close the walker is. */
  step(volume: number) {
    const ctx = this.ready();
    if (!ctx || volume <= 0.02) return;
    if (this.sample('step-stone', 0.5 * Math.min(1, volume))) return;
    const length = Math.floor(ctx.sampleRate * 0.09);
    const buffer = ctx.createBuffer(1, length, ctx.sampleRate);
    const data = buffer.getChannelData(0);
    for (let i = 0; i < length; i++) data[i] = (Math.random() * 2 - 1) * (1 - i / length) ** 3;
    const source = ctx.createBufferSource();
    source.buffer = buffer;
    const filter = ctx.createBiquadFilter();
    filter.type = 'bandpass';
    filter.frequency.value = 430 + Math.random() * 120;
    filter.Q.value = 0.9;
    const gain = ctx.createGain();
    gain.gain.value = 0.22 * Math.min(1, volume);
    source.connect(filter).connect(gain).connect(this.out ?? ctx.destination);
    source.start();
  }

  /** The bomb's beep, tighter as the fuse runs down. */
  beep(urgency: number) {
    const pitch = 880 + urgency * 420;
    this.tone(pitch, pitch, 0.06, 0.08, 'square');
  }

  blast(volume = 1) {
    const ctx = this.ready();
    if (!ctx) return;
    this.sample('metal', 0.6 * volume, 0.7);
    const length = Math.floor(ctx.sampleRate * 1.1);
    const buffer = ctx.createBuffer(1, length, ctx.sampleRate);
    const data = buffer.getChannelData(0);
    for (let i = 0; i < length; i++) data[i] = (Math.random() * 2 - 1) * (1 - i / length) ** 1.6;
    const source = ctx.createBufferSource();
    source.buffer = buffer;
    const filter = ctx.createBiquadFilter();
    filter.type = 'lowpass';
    filter.frequency.setValueAtTime(900, ctx.currentTime);
    filter.frequency.exponentialRampToValueAtTime(120, ctx.currentTime + 0.9);
    const gain = ctx.createGain();
    gain.gain.value = 0.5 * volume;
    source.connect(filter).connect(gain).connect(this.out ?? ctx.destination);
    source.start();
    this.tone(120, 40, 0.8, 0.3 * volume, 'sine');
  }

  count(final: boolean) {
    this.tone(final ? 988 : 660, final ? 988 : 660, final ? 0.35 : 0.14, 0.16, 'square');
  }

  win() {
    if (this.sample('jingle-win', 0.6)) return;
    [523, 659, 784, 1047].forEach((f, i) => this.tone(f, f, 0.22, 0.12, 'triangle', i * 0.11));
  }

  // On the board.

  /** The dice shaken and thrown, two at once for a double roll. */
  dice(count: number) {
    this.sample('dice-shake', 0.55);
    this.sample(count > 1 ? 'dice-throw' : 'die-throw', 0.7, 1, 0.3);
  }

  hop(ground: Ground) {
    if(!this.sample(`step-${ground}`, 0.4, 1.1))this.cityNoise(.075,ground==='snow'?1100:650,.10,'lowpass');
  }

  coins(many = false) {
    this.sample(many ? 'coins' : 'chips', 0.55);
  }

  /** One coin of a handful arriving: a rising run of clinks, and the whole pile at the end. */
  coinLand(index: number, last: boolean) {
    this.sample('chips', 0.75, 1 + Math.min(index, 10) * 0.045);
    if (last) this.sample('coins', 0.95);
  }

  /** Knocked flat: a heavy hit and the fall. */
  knockout() {
    this.hurt();
    this.sample('thud', 1, 0.7, 0.12);
    this.sample('lose', 0.6, 0.9, 0.2);
  }

  lose() {
    this.sample('lose', 0.45);
    this.sample('chips', 0.3, 0.75, 0.08);
  }

  /** An item gained, and one used. */
  item() {
    this.sample('power', 0.45);
  }

  /** An item dropping into the sack, a little deeper as it fills. */
  bagIn(held: number) {
    this.sample('soft', 0.8, 1.15 - Math.min(held, 5) * 0.06);
    this.sample('power', 0.4, 1 + Math.min(held, 5) * 0.05, 0.05);
  }

  useItem() {
    this.sample('confirm', 0.5);
  }

  shop() {
    this.sample('door', 0.5);
  }

  cannon() {
    this.sample('plate', 0.7, 0.8);
    this.blast(0.35);
  }

  land() {
    this.sample('thud', 0.8);
  }

  hurt() {
    if(!this.sample('punch-heavy', 0.7)){this.cityNoise(.13,700,.25,'lowpass');this.tone(160,55,.12,.1,'triangle');}
  }

  slide() {
    this.sample('ice', 0.6);
  }

  trap() {
    this.sample('metal', 0.6, 1.3);
  }

  blocked() {
    this.sample('blocked', 0.45);
  }

  prize() {
    this.sample('bell', 0.5);
    this.sample('jingle-prize', 0.6, 1, 0.25);
  }

  /** Your turn, and a fork in the road to choose at. */
  turn() {
    this.sample('confirm', 0.55, 1.1);
  }

  choose() {
    this.sample('question', 0.5);
  }

  taunt() {
    this.sample('pluck', 0.6);
  }

  /** The storm moving: a roll of thunder. */
  thunder() {
    this.blast(0.4);
  }

  round() {
    this.sample('jingle-round', 0.45);
  }

  over() {
    this.sample('jingle-over', 0.65);
  }

  /**
   * How far the camera is down a tunnel, from 0 to 1, called each frame:
   * the echo follows it, and water drips now and then while it is down there.
   */
  underground(mix: number, dt: number) {
    if (mix === this.cave && mix === 0) return;
    this.cave = mix;
    if (this.echo && this.ctx) glide(this.echo.gain, mix * 0.55, this.ctx.currentTime, 0.1);
    if (mix < 0.5) return;
    this.dripIn -= dt;
    if (this.dripIn > 0) return;
    this.dripIn = 1.2 + Math.random() * 2.8;
    this.sample('drip', 0.18 + Math.random() * 0.12, 0.7 + Math.random() * 0.5);
  }

  /** Traffic and crawler audio follows world distance and camera-relative side.
   * Voices survive snapshots and stop immediately on mute, map exit or disposal. */
  cityFrame(dt:number,listener:{x:number;z:number;yaw:number},cars:{id:number;x:number;z:number;speed:number;braking:boolean}[],crawler:{x:number;z:number}|null,pedestrians:{id:number;x:number;z:number;walking:boolean;fleeing:boolean}[]=[],ambience=false){
    this.cityListener=listener;
    const ctx=this.ready();
    if(!ctx||!this.out){this.stopEngines();this.ambience?.stop();return;}
    this.ambience?.frame(dt,listener,ambience,this.volume.ambience);
    const live=new Set<number>();
    for(const car of cars){
      const space=this.citySpace(car.x,car.z);
      if(space.distance>38)continue;
      live.add(car.id);
      let voice=this.engines.get(car.id);
      if(!voice){
        const osc=ctx.createOscillator(),harmonic=ctx.createOscillator(),gain=ctx.createGain(),pan=ctx.createStereoPanner(),filter=ctx.createBiquadFilter(),mix=ctx.createGain();
        osc.type='triangle';harmonic.type='sawtooth';mix.gain.value=.16;filter.type='lowpass';filter.frequency.value=520;
        gain.gain.value=0;osc.connect(filter);harmonic.connect(mix).connect(filter);filter.connect(gain).connect(pan).connect(this.out);osc.start();harmonic.start();
        voice={osc,harmonic,gain,pan,distance:space.distance};this.engines.set(car.id,voice);
      }
      const approaching=Math.max(-1,Math.min(1,(voice.distance-space.distance)/Math.max(.016,dt)/12));
      const pitch=(48+Math.abs(car.speed)*7)*(1+approaching*.1);
      glide(voice.osc.frequency, pitch,ctx.currentTime,.12);glide(voice.harmonic.frequency, pitch*2.02,ctx.currentTime,.12);
      glide(voice.gain.gain, space.volume*(car.braking?.012:.035),ctx.currentTime,.12);glide(voice.pan.pan, space.pan,ctx.currentTime,.1);voice.distance=space.distance;
    }
    for(const [id,voice]of this.engines)if(!live.has(id)){voice.osc.stop();voice.harmonic.stop();voice.gain.disconnect();voice.pan.disconnect();this.engines.delete(id);}
    const people=new Set<number>();
    for(const person of pedestrians){
      people.add(person.id);const space=this.citySpace(person.x,person.z);
      let wait=(this.pedestrianSteps.get(person.id)??(.12+(person.id%4)*.11))-dt;
      if(wait<=0){wait=person.fleeing?.23:.48;if(person.walking&&space.distance<18)this.cityNoise(.065,1200,.045*space.volume,'lowpass',space.pan);}
      this.pedestrianSteps.set(person.id,wait);
    }
    for(const id of this.pedestrianSteps.keys())if(!people.has(id))this.pedestrianSteps.delete(id);
    this.crawlerBreath-=dt;
    if(crawler && this.citySpace(crawler.x,crawler.z).distance<14 && this.crawlerBreath<=0){
      this.crawlerBreath=2.6;
      const space=this.citySpace(crawler.x,crawler.z);
      this.cityNoise(.55,310,.07*space.volume,'lowpass',space.pan);
      this.tone(95,62,.7,.026*space.volume,'sawtooth',0,space.pan);
      this.cityNoise(.16,1550,.035*space.volume,'bandpass',space.pan);
    }
  }

  private citySpace(x:number,z:number){
    const dx=x-this.cityListener.x,dz=z-this.cityListener.z,distance=Math.hypot(dx,dz);
    return {distance,volume:Math.max(0,1-distance/38)**2,pan:Math.max(-.85,Math.min(.85,(dx*Math.cos(this.cityListener.yaw)-dz*Math.sin(this.cityListener.yaw))/Math.max(3,distance)))};
  }
  private cityNoise(duration:number,frequency:number,volume:number,type:BiquadFilterType,pan=0){
    const ctx=this.ready();if(!ctx||volume<=.001)return;
    const buffer=ctx.createBuffer(1,Math.ceil(ctx.sampleRate*duration),ctx.sampleRate),data=buffer.getChannelData(0);
    for(let i=0;i<data.length;i++)data[i]=(Math.random()*2-1)*(1-i/data.length)**2;
    const source=ctx.createBufferSource(),filter=ctx.createBiquadFilter(),gain=ctx.createGain(),stereo=ctx.createStereoPanner();source.buffer=buffer;filter.type=type;filter.frequency.value=frequency;gain.gain.value=volume;stereo.pan.value=pan;source.connect(filter).connect(gain).connect(stereo).connect(this.out??ctx.destination);source.start();
  }
  cityHorn(id:number,x:number,z:number){
    const ctx=this.ready();if(!ctx)return;
    if(ctx.currentTime-(this.hornAt.get(id)??-10)<1.5)return;
    this.hornAt.set(id,ctx.currentTime);
    const space=this.citySpace(x,z);
    for(const delay of [0,.24])for(const hz of [440,554])this.tone(hz,hz*.98,.16,.045*space.volume,'square',delay,space.pan);
  }
  cityRobbed(x:number,z:number){const s=this.citySpace(x,z);this.cityNoise(.25,2600,.09*s.volume,'bandpass',s.pan);this.tone(390,620,.12,.04*s.volume,'triangle',0,s.pan);this.tone(620,830,.12,.04*s.volume,'triangle',.10,s.pan);}
  cityBrake(x:number,z:number){const s=this.citySpace(x,z);this.cityNoise(.42,2400,.14*s.volume,'bandpass',s.pan);}
  cityCrash(x:number,z:number){const s=this.citySpace(x,z);this.cityNoise(.65,800,.5*s.volume,'lowpass',s.pan);this.tone(130,35,.4,.18*s.volume,'triangle',0,s.pan);this.cityNoise(.25,3100,.2*s.volume,'bandpass',s.pan);}
  private stopEngines(){for(const voice of this.engines.values()){voice.osc.stop();voice.harmonic.stop();voice.gain.disconnect();voice.pan.disconnect();}this.engines.clear();}

  dispose() {
    this.stopEngines();this.ambience?.dispose();this.ambienceOut?.disconnect();
    this.stopWatching();
    window.removeEventListener('pointerdown', this.unlock);
    window.removeEventListener('keydown', this.unlock);
    void this.ctx?.close();
  }
}
