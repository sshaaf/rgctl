const { Router } = require('express');
const { requireAuth } = require('../middleware/auth');
const { asyncHandler } = require('../middleware/errorHandler');
const orderService = require('../services/orderService');

const router = Router();

router.get(
  '/',
  requireAuth,
  asyncHandler(async (req, res) => {
    res.json(orderService.listOrders(req.user.userId));
  }),
);

router.post(
  '/',
  requireAuth,
  asyncHandler(async (req, res) => {
    const result = orderService.checkout(req.user.userId);
    res.status(201).json(result);
  }),
);

router.get(
  '/:id',
  requireAuth,
  asyncHandler(async (req, res) => {
    const result = orderService.getOrder(req.user.userId, req.params.id);
    res.json(result);
  }),
);

module.exports = router;
