/**
 * Intentional call graph for rgctl expected-facts checks.
 * Prefer direct function calls so extraction does not depend on DI.
 */

export function correctnessLeaf(): number {
  return 42;
}

export function correctnessMid(): number {
  return correctnessLeaf() + 1;
}

export function correctnessRoot(flag: boolean): number {
  const value = correctnessMid();
  if (flag) {
    return value * 2;
  }
  return value;
}
