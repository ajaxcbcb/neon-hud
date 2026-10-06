const colors = [[130, 245, 162], [255, 209, 102], [255, 102, 142]];
export function stressPercent(value: number | null, inverse = false): number | null {
  if (value === null || !Number.isFinite(value)) return null;
  const bounded = Math.max(0, Math.min(100, value));
  return inverse ? 100 - bounded : bounded;
}
export function stressColor(value: number | null, inverse = false): string {
  const stress = stressPercent(value, inverse);
  if (stress === null) return 'var(--muted)';
  const segment = stress <= 70 ? 0 : 1;
  const amount = segment === 0 ? stress / 70 : (stress - 70) / 30;
  return `rgb(${colors[segment].map((c, index) => Math.round(c + (colors[segment + 1][index] - c) * amount)).join(', ')})`;
}
export function stressGradient(value: number | null, inverse = false, vertical = false): string {
  const stress = stressPercent(value, inverse);
  if (stress === null) return 'var(--muted)';
  const start = inverse ? stressColor(stress) : stressColor(0);
  const end = stressColor(inverse ? Math.min(100, stress + 15) : stress);
  const middle = !inverse && stress > 70 ? `, ${stressColor(70)} ${70 / stress * 100}%` : '';
  return `linear-gradient(${vertical ? 'to top' : 'to right'}, ${start}${middle}, ${end})`;
}
