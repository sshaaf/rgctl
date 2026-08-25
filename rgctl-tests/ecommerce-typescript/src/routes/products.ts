import { Router } from 'express';
import { requireAuth } from '../middleware/auth';
import { asyncHandler } from '../middleware/errorHandler';
import * as productService from '../services/productService';
import reviewRoutes from './reviews';

const router = Router();

router.get(
  '/',
  asyncHandler(async (_req, res) => {
    res.json(productService.listProducts());
  }),
);

router.post(
  '/',
  requireAuth,
  asyncHandler(async (req, res) => {
    const result = productService.createProduct(req.body);
    res.status(201).json(result);
  }),
);

router.get(
  '/:id',
  asyncHandler(async (req, res) => {
    const result = productService.getProduct(String(req.params.id));
    res.json(result);
  }),
);

router.use('/:id/reviews', reviewRoutes);

export default router;
