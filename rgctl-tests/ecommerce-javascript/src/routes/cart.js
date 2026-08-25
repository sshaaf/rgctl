const { Router } = require('express');
const { requireAuth } = require('../middleware/auth');
const { asyncHandler } = require('../middleware/errorHandler');
const cartService = require('../services/cartService');

const router = Router();

router.get(
  '/',
  requireAuth,
  asyncHandler(async (req, res) => {
    res.json(cartService.listCart(req.user.userId));
  }),
);

router.post(
  '/items',
  requireAuth,
  asyncHandler(async (req, res) => {
    const result = cartService.addCartItem(req.user.userId, req.body);
    res.status(201).json(result);
  }),
);

router.delete(
  '/items/:productId',
  requireAuth,
  asyncHandler(async (req, res) => {
    cartService.removeCartItem(req.user.userId, req.params.productId);
    res.status(204).send();
  }),
);

module.exports = router;
