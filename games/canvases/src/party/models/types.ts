/** Uploaded whole models keep their own materials and are independent of
 * the base character's shirts, pants and other fitted store accessories. */
export type ModelPose = 'idle' | 'ride' | 'cheer' | 'sad' | 'fall' | 'sit' | 'wave';
export type ModelClipMap = Partial<Record<ModelPose, string | null>>;
export type ModelAnimation = { name: string; duration: number };
export type ModelReport = {
  bytes: number;
  vertices: number;
  triangles: number;
  meshes: number;
  textures: number;
  rigged: boolean;
  playable: boolean;
  animations: ModelAnimation[];
  clip_map: ModelClipMap;
  kind: 'custom_model';
};
export type ImportedModel = {
  id: string;
  user_id: string;
  name: string;
  url: string;
  report: ModelReport;
  created_at: string;
};
export type ModelImportApi = {
  list: () => Promise<ImportedModel[]>;
  upload: (file: File, name: string, clips: ModelClipMap) => Promise<ImportedModel>;
  remove: (id: string) => Promise<void>;
};
export const MODEL_LIMITS = {
  bytes: 10 * 1024 * 1024,
  vertices: 60_000,
  triangles: 30_000,
  nodes: 128,
  textureSize: 2048,
  textures: 8,
  bones: 96,
} as const;
