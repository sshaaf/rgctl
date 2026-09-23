/** Named-arrow fixture for extraction / blast-radius gates. */

export function declaredAdd(a: number, b: number): number {
  return a + b;
}

export const arrowAdd = (a: number, b: number): number => a + b;

export const arrowHelper = (): number => 1;

export const api = {
  fetchAll: async (): Promise<number> => 0,
};

export function callArrows(): number {
  return arrowAdd(1, 2) + arrowHelper();
}

[1].map((x) => x + 1);
