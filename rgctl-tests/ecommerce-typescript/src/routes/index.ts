import { Router } from 'express';
import authRoutes from './auth';
import cartRoutes from './cart';
import categoryRoutes from './categories';
import healthRoutes from './health';
import orderRoutes from './orders';
import productRoutes from './products';
import coolstoreRoutes from '../coolstore/routes';

const router = Router();

router.use(healthRoutes);
router.use('/api/auth', authRoutes);
router.use('/api/categories', categoryRoutes);
router.use('/api/products', productRoutes);
router.use('/api/cart', cartRoutes);
router.use('/api/orders', orderRoutes);
router.use(coolstoreRoutes);

export default router;
