import { createCatalogProduct, type CatalogProduct } from '../models/catalogProduct';

export class CoolstoreProductService {
  private readonly catalog = new Map<string, CatalogProduct>();

  constructor() {
    this.seed('329299', 'Red Fedora', 'Official Red Hat Fedora', 34.99);
    this.seed('329199', 'Forge Laptop Sticker', 'JBoss Community sticker', 8.5);
    this.seed('165613', 'Solid Performance Polo', 'Moisture-wicking polo', 17.8);
    this.seed('165614', 'Ogios T-shirt', 'CoolStore tee', 11.5);
    this.seed('165954', 'Quarkus Stickers', 'Pack of stickers', 9.99);
  }

  private seed(id: string, name: string, desc: string, price: number): void {
    this.catalog.set(id, createCatalogProduct(id, name, desc, price));
  }

  getProducts(): CatalogProduct[] {
    return Array.from(this.catalog.values());
  }

  getProductByItemId(itemId: string): CatalogProduct | undefined {
    return this.catalog.get(itemId);
  }
}
