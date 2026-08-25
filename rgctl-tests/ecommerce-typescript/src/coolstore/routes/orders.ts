import { Router } from 'express';
import { AppError } from '../../utils/errors';
import { asyncHandler } from '../../middleware/errorHandler';
import type { CoolstoreOrderService } from '../services/coolstoreOrderService';

export function createOrderRouter(orderService: CoolstoreOrderService): Router {
  const router = Router();

  router.get(
    '/',
    asyncHandler(async (_req, res) => {
      res.json(orderService.getOrders());
    }),
  );

  router.get(
    '/:orderId',
    asyncHandler(async (req, res) => {
      const orderId = Number.parseInt(String(req.params.orderId), 10);
      const order = orderService.getOrderById(orderId);
      if (!order) {
        throw new AppError('not found', 404);
      }
      res.json(order);
    }),
  );

  return router;
}
