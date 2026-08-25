/** Lightweight CoolStore catalog product (itemId keyed). */

function createCatalogProduct(itemId, name, desc, price) {
  return { itemId, name, desc, price };
}

module.exports = { createCatalogProduct };
