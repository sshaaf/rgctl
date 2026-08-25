const { Router } = require('express');
const { AppError } = require('../../utils/errors');
const { asyncHandler } = require('../../middleware/errorHandler');

function createOrderRouter(orderService) {
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

module.exports = { createOrderRouter };
