const { Router } = require('express');
const { CoolstoreOrderService } = require('../services/coolstoreOrderService');
const { CoolstoreProductService } = require('../services/coolstoreProductService');
const { PromoService } = require('../services/promoService');
const { ShippingService } = require('../services/shippingService');
const { ShoppingCartService } = require('../services/shoppingCartService');
const { createCartRouter } = require('./cart');
const { createOrderRouter } = require('./orders');
const { createProductRouter } = require('./products');

const productService = new CoolstoreProductService();
const promoService = new PromoService();
const shippingService = new ShippingService();
const orderService = new CoolstoreOrderService();
const shoppingCartService = new ShoppingCartService(
  productService,
  promoService,
  shippingService,
  orderService,
);

const router = Router();

router.use('/services/products', createProductRouter(productService));
router.use('/services/cart', createCartRouter(shoppingCartService));
router.use('/services/orders', createOrderRouter(orderService));

module.exports = router;
