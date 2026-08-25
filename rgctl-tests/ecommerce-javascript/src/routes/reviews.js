const { Router } = require('express');
const { requireAuth } = require('../middleware/auth');
const { asyncHandler } = require('../middleware/errorHandler');
const reviewService = require('../services/reviewService');

const router = Router({ mergeParams: true });

router.get(
  '/',
  asyncHandler(async (req, res) => {
    const productId = req.params.id;
    res.json(reviewService.listReviews(productId));
  }),
);

router.post(
  '/',
  requireAuth,
  asyncHandler(async (req, res) => {
    const productId = req.params.id;
    const result = reviewService.createReview(req.user.userId, productId, req.body);
    res.status(201).json(result);
  }),
);

module.exports = router;
