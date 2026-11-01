/** The space between hand cards so `count` cards of `cardWidth` fit in
 * `width`; negative once they have to overlap. */
export function handSpacing(width: number, count: number, cardWidth: number, gap = 6): number {
  if (count < 2 || width <= 0) return gap;
  return Math.min(gap, (width - count * cardWidth) / (count - 1));
}
