const { Router } = require('express');
const authRoutes = require('./auth');
const cartRoutes = require('./cart');
const categoryRoutes = require('./categories');
const healthRoutes = require('./health');
const orderRoutes = require('./orders');
const productRoutes = require('./products');
const coolstoreRoutes = require('../coolstore/routes');

const router = Router();

router.use(healthRoutes);
router.use('/api/auth', authRoutes);
router.use('/api/categories', categoryRoutes);
router.use('/api/products', productRoutes);
router.use('/api/cart', cartRoutes);
router.use('/api/orders', orderRoutes);
router.use(coolstoreRoutes);

module.exports = router;
