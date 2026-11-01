export const BODY_MIN = 0.7;
export const BODY_MAX = 1.4;
export function bodyWidth(value?: number | null) {
  return typeof value === "number" && Number.isFinite(value)
    ? Math.max(BODY_MIN, Math.min(BODY_MAX, value))
    : 1;
}
