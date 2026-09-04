import type { CatalogProduct } from '../src/coolstore/models/catalogProduct';
import { nowIso } from '../src/utils/time';

export function useTypes(): string {
  const _product: CatalogProduct | null = null;
  return nowIso();
}
