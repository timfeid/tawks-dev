export interface MoveState {
  forward: boolean;
  back: boolean;
  left: boolean;
  right: boolean;
  up: boolean;
  down: boolean;
}

export interface KeyboardCallbacks {
  toggleManualCamera: () => void;
  togglePacks: () => void;
  toggleHelp: () => void;
  toggleFullscreen: () => void;
  rotateRing: (delta: number) => void;
  rotateKindFilters: (delta: number) => void;
  revealCard: (index: number) => void;
  rotateCamera: () => void;
  closeActionMenu: () => void;
  exitPacks: () => void;
  exitHelp: () => void;
  exitCardBrowser: () => void;
  inspectNavigate: (delta: number) => void;
  inspectSelected: () => void;
  confirmSelection: () => void;
  packInspectNavigate: (delta: number) => void;
  cancelTargeting: () => void;
  cancelOptionalTriggerTarget: () => void;
  advanceTurn: () => void;
}

export interface KeyboardStateGetter {
  showPacks: () => boolean;
  showHelp: () => boolean;
  packPhase: () => string;
  packFlyInCount: () => number;
  packFocusIndex: () => number;
  setPackFocusIndex: (index: number) => void;
  isInspecting: () => boolean;
  deckBuilderMode: () => boolean;
  cardBrowserMode: () => boolean;
  deckSelectMode: () => boolean;
  manualCamera: () => boolean;
  hasActionMenu: () => boolean;
  playerCount: () => number;
  isTargeting: () => boolean;
  hasCancellableTriggerTarget: () => boolean;
}

export class KeyboardController {
  public moveState: MoveState = {
    forward: false,
    back: false,
    left: false,
    right: false,
    up: false,
    down: false,
  };

  private _handleKeyDown: (e: KeyboardEvent) => void;
  private _handleKeyUp: (e: KeyboardEvent) => void;

  constructor(
    private getState: KeyboardStateGetter,
    private callbacks: KeyboardCallbacks,
  ) {
    this._handleKeyDown = (e: KeyboardEvent) => this.handleKeyDown(e);
    this._handleKeyUp = (e: KeyboardEvent) => this.handleKeyUp(e);
  }

  private handleKeyDown(event: KeyboardEvent): void {
    if (event.repeat) return;
    const key = event.key.toLowerCase();
    const s = this.getState;

    if (key === 'c') {
      this.callbacks.toggleManualCamera();
      this.moveState.forward = false;
      this.moveState.back = false;
      this.moveState.left = false;
      this.moveState.right = false;
      this.moveState.up = false;
      this.moveState.down = false;
      return;
    }

    if (key === 'escape') {
      if (s.showPacks()) { this.callbacks.exitPacks(); return; }
      if (s.showHelp()) { this.callbacks.exitHelp(); return; }
      if (s.isTargeting()) { this.callbacks.cancelTargeting(); return; }
      if (s.hasCancellableTriggerTarget()) {
        this.callbacks.cancelOptionalTriggerTarget();
        return;
      }
      if (s.cardBrowserMode()) { this.callbacks.exitCardBrowser(); return; }
      if (document.fullscreenElement) { document.exitFullscreen(); return; }
      if (s.hasActionMenu()) { this.callbacks.closeActionMenu(); return; }
      return;
    }

    if (key === '?' || (key === '/' && event.shiftKey)) {
      this.callbacks.toggleHelp();
      return;
    }

    if (key === 'p' && !event.ctrlKey && !event.metaKey) {
      this.callbacks.togglePacks();
      return;
    }

    if (key === 'enter' && event.altKey) {
      this.callbacks.toggleFullscreen();
      return;
    }

    const ringMode = (s.deckBuilderMode() || s.deckSelectMode() || s.cardBrowserMode()) && !s.manualCamera();
    if (ringMode) {
      if (key === 'a' || key === 'arrowleft') {
        if (s.isInspecting()) this.callbacks.inspectNavigate(-1);
        else this.callbacks.rotateRing(-1);
        return;
      } else if (key === 'd' || key === 'arrowright') {
        if (s.isInspecting()) this.callbacks.inspectNavigate(1);
        else this.callbacks.rotateRing(1);
        return;
      } else if (event.code === 'Space') {
        this.callbacks.inspectSelected();
        event.preventDefault();
        return;
      } else if (key === 'enter') {
        this.callbacks.confirmSelection();
        event.preventDefault();
        return;
      }
    }

    if (s.showPacks() && s.packPhase() === 'reveal') {
      const launched = s.packFlyInCount();
      if (launched > 0 && (key === 'a' || key === 'arrowleft')) {
        if (s.isInspecting()) {
          this.callbacks.packInspectNavigate(1);
        } else {
          s.setPackFocusIndex((s.packFocusIndex() + 1) % launched);
        }
        event.preventDefault();
        return;
      }
      if (launched > 0 && (key === 'd' || key === 'arrowright')) {
        if (s.isInspecting()) {
          this.callbacks.packInspectNavigate(-1);
        } else {
          s.setPackFocusIndex((s.packFocusIndex() - 1 + launched) % launched);
        }
        event.preventDefault();
        return;
      }
      if (event.code === 'Space') {
        this.callbacks.revealCard(s.packFocusIndex());
        event.preventDefault();
        return;
      }
    }

    if (key === 'w') this.moveState.forward = true;
    else if (key === 's') this.moveState.back = true;
    else if (key === 'a') this.moveState.left = true;
    else if (key === 'd') this.moveState.right = true;
    else if (event.code === 'Space') {
      if (!ringMode && !(s.showPacks() && s.packPhase() === 'reveal')) {
        this.callbacks.advanceTurn();
      } else {
        this.moveState.up = true;
      }
    } else if (event.key === 'Shift') this.moveState.down = true;
    else if (key === 'v') this.callbacks.rotateCamera();
  }

  private handleKeyUp(event: KeyboardEvent): void {
    const key = event.key.toLowerCase();
    if (key === 'w') this.moveState.forward = false;
    else if (key === 's') this.moveState.back = false;
    else if (key === 'a') this.moveState.left = false;
    else if (key === 'd') this.moveState.right = false;
    else if (event.code === 'Space') this.moveState.up = false;
    else if (event.key === 'Shift') this.moveState.down = false;
  }

  public attach(): void {
    window.addEventListener('keydown', this._handleKeyDown);
    window.addEventListener('keyup', this._handleKeyUp);
  }

  public dispose(): void {
    window.removeEventListener('keydown', this._handleKeyDown);
    window.removeEventListener('keyup', this._handleKeyUp);
  }
}
