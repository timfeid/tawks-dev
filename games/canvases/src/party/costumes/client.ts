import type { Costume, JerseyDesign } from '../protocol';
import type { ImportedModel, ModelImportApi } from '../models/types';
/** `set_up` once a look has been saved; a Tawkathon asks for one before you play.
 * `jersey` is the owner's synced PuckLab club design, for trying the PuckLab jersey on. */
export type Wardrobe = { costume: Costume; owned: string[]; set_up?: boolean; jersey?: JerseyDesign | null };
export type WardrobeClient = {
  load(): Promise<Wardrobe>;
  apply(costume: Costume): Promise<Wardrobe>;
  buy(id: string): Promise<Wardrobe>;
  models: ModelImportApi & {get(id: string): Promise<ImportedModel>};
  /** Opens the avatar settings, wherever the host app keeps them. */
  open?(): void;
};
let host: WardrobeClient | null = null;
export function setWardrobeClient(client: WardrobeClient) { host = client; }
export function wardrobeClient() { return host; }

// A look saved in settings reaches every game showing, popped-out windows included.
const watchers = new Set<(wardrobe: Wardrobe) => void>();
const channel = typeof BroadcastChannel === 'undefined' ? null : new BroadcastChannel('tawkathon-wardrobe');
channel?.addEventListener('message', (event) => watchers.forEach((watcher) => watcher(event.data as Wardrobe)));

/** Tells every game that the look changed. */
export function wardrobeSaved(wardrobe: Wardrobe) {
  watchers.forEach((watcher) => watcher(wardrobe));
  channel?.postMessage(JSON.parse(JSON.stringify(wardrobe)));
}

/** Calls `watcher` with each look saved from now on; returns the way to stop. */
export function onWardrobeSaved(watcher: (wardrobe: Wardrobe) => void) {
  watchers.add(watcher);
  return () => { watchers.delete(watcher); };
}
