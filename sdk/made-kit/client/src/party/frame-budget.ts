/** Adjust render resolution slowly, keeping CSS/HUD size and input unchanged. */
export class FrameBudget {
  private previous=0;private frames=0;private elapsed=0;private changedAt=0;
  ratio:number;private maximum:number;private minimum:number;
  constructor(ratio:number,maximum=ratio,minimum=Math.min(.75,ratio)){this.ratio=ratio;this.maximum=maximum;this.minimum=minimum;}
  sample(now:number):number|null{
    const delta=now-this.previous;this.previous=now;
    if(delta<=0||delta>1000){this.frames=0;this.elapsed=0;return null;}
    this.frames++;this.elapsed+=delta;if(this.elapsed<1800||this.frames<20)return null;
    const mean=this.elapsed/this.frames;this.frames=0;this.elapsed=0;
    let next=this.ratio;
    if(mean>19&&now-this.changedAt>1800)next=Math.max(this.minimum,this.ratio-.125);
    else if(mean<14.5&&now-this.changedAt>10000)next=Math.min(this.maximum,this.ratio+.125);
    if(next===this.ratio)return null;this.ratio=next;this.changedAt=now;return next;
  }
}
