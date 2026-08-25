import { Router } from 'express';
import { requireAuth } from '../middleware/auth';
import { asyncHandler } from '../middleware/errorHandler';
import * as orderService from '../services/orderService';

const router = Router();

router.get(
  '/',
  requireAuth,
  asyncHandler(async (req, res) => {
    res.json(orderService.listOrders(req.user!.userId));
  }),
);

router.post(
  '/',
  requireAuth,
  asyncHandler(async (req, res) => {
    const result = orderService.checkout(req.user!.userId);
    res.status(201).json(result);
  }),
);

router.get(
  '/:id',
  requireAuth,
  asyncHandler(async (req, res) => {
    const result = orderService.getOrder(req.user!.userId, String(req.params.id));
    res.json(result);
  }),
);

export default router;
