/** The yaw that turns a party character, which faces +z, toward travel along (x, z). */
export const headingOf = (x: number, z: number) => Math.atan2(x, z);

/** Eases a yaw toward another the short way round. */
export function turnToward(from: number, to: number, dt: number, rate = 16) {
  const delta = Math.atan2(Math.sin(to - from), Math.cos(to - from));
  return from + delta * (1 - Math.exp(-rate * dt));
}
