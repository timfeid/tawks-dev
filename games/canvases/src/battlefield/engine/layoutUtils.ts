export const computeRowLayout = (
  totalCards: number, 
  layoutRadius: number, 
  cardW: number, 
  cardH: number, 
  seatArc: number
) => {
  const rows = 5;
  const rowCounts = Array.from({ length: rows }, () => 0);
  const targetArc = seatArc * 0.68;
  let remaining = totalCards;

  // Row 0 sits just inside the yellow table-edge ring; successive rows step
  // inward by ~0.85 units each. The small 0.25 offset leaves just enough
  // clearance for the outer card tops to tuck under the ring without crossing it.
  const rowRadiusFor = (row: number) => layoutRadius - cardH * 0.5 - row * 0.85 - 0.25;

  for (let r = 0; r < rows; r += 1) {
    if (remaining <= 0) break;
    if (r < rows - 1) {
      const spacingAngle = (cardW + 0.32) / rowRadiusFor(r);
      const maxCols = Math.max(1, Math.floor(targetArc / spacingAngle) + 1);
      const take = Math.min(remaining, maxCols);
      rowCounts[r] = take;
      remaining -= take;
    } else {
      rowCounts[r] = remaining;
      remaining = 0;
    }
  }

  const rowStarts = rowCounts.reduce<number[]>((acc, count, i) => {
    acc[i] = (acc[i - 1] ?? 0) + (i > 0 ? rowCounts[i - 1] : 0);
    return acc;
  }, []);

  return { rowCounts, rowStarts, rowRadiusFor, targetArc };
};
