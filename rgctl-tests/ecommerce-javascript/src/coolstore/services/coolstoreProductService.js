const { createCatalogProduct } = require('../models/catalogProduct');

class CoolstoreProductService {
  constructor() {
    this.catalog = new Map();
    this.seed('329299', 'Red Fedora', 'Official Red Hat Fedora', 34.99);
    this.seed('329199', 'Forge Laptop Sticker', 'JBoss Community sticker', 8.5);
    this.seed('165613', 'Solid Performance Polo', 'Moisture-wicking polo', 17.8);
    this.seed('165614', 'Ogios T-shirt', 'CoolStore tee', 11.5);
    this.seed('165954', 'Quarkus Stickers', 'Pack of stickers', 9.99);
  }

  seed(id, name, desc, price) {
    this.catalog.set(id, createCatalogProduct(id, name, desc, price));
  }

  getProducts() {
    return Array.from(this.catalog.values());
  }

  getProductByItemId(itemId) {
    return this.catalog.get(itemId);
  }
}

module.exports = { CoolstoreProductService };
