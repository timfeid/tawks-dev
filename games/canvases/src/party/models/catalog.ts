import type { ImportedModel, ModelReport } from './types';

export type CatalogCharacter = {
  id: string;
  slot: 'model';
  name: string;
  price: number;
  icon: string;
  file: string;
  sourceUrl: string;
  license: string;
  report: ModelReport;
};

// The asset manifest and its GLBs are bundled together. Previewing a store
// character never requires an account, an entitlement, or an API model lookup.
const manifests=import.meta.glob('./catalog.json',{eager:true,import:'default'}) as Record<string,unknown>;
const files=import.meta.glob('./catalog/*.glb',{eager:true,query:'?url',import:'default'}) as Record<string,string>;
export const MODEL_CATALOG:CatalogCharacter[]=Object.values(manifests).flatMap(value=>Array.isArray(value)?value:[]);

/** Full URLs also work when the same asset is loaded from a Tauri webview. */
export function resolveCatalogModel(id:string,base=globalThis.location?.href??import.meta.url):ImportedModel|null {
  const entry=MODEL_CATALOG.find(character=>character.id===id);
  const asset=entry&&files[`./catalog/${entry.file}`];
  if(!entry||!asset)return null;
  return {id:entry.id,user_id:'bot',name:entry.name,url:new URL(asset,base).href,report:entry.report,created_at:'1970-01-01T00:00:00Z'};
}
