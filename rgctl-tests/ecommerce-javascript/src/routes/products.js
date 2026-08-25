const { Router } = require('express');
const { requireAuth } = require('../middleware/auth');
const { asyncHandler } = require('../middleware/errorHandler');
const productService = require('../services/productService');
const reviewRoutes = require('./reviews');

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
    const result = productService.getProduct(req.params.id);
    res.json(result);
  }),
);

router.use('/:id/reviews', reviewRoutes);

module.exports = router;
