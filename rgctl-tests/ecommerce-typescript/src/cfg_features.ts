/**
 * CFG feature probes for rgctl expected-facts (TypeScript lowering coverage).
 */

export function cfgShortCircuit(a: boolean, b: boolean): number {
  if (a && b) {
    return 1;
  }
  return 0;
}

export function cfgForLoop(xs: number[]): number {
  let total = 0;
  for (let i = 0; i < xs.length; i++) {
    total += xs[i];
  }
  return total;
}

export function cfgTryCatch(): number {
  try {
    throw new Error("boom");
  } catch {
    return 1;
  }
}

export function cfgSwitch(x: number): number {
  switch (x) {
    case 1:
      return 10;
    case 2:
      return 20;
    default:
      return 0;
  }
}
