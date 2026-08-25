import { Router } from 'express';
import { CoolstoreOrderService } from '../services/coolstoreOrderService';
import { CoolstoreProductService } from '../services/coolstoreProductService';
import { PromoService } from '../services/promoService';
import { ShippingService } from '../services/shippingService';
import { ShoppingCartService } from '../services/shoppingCartService';
import { createCartRouter } from './cart';
import { createOrderRouter } from './orders';
import { createProductRouter } from './products';

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

export default router;
