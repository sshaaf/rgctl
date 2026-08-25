import { Router } from 'express';
import { requireAuth } from '../middleware/auth';
import { asyncHandler } from '../middleware/errorHandler';
import * as reviewService from '../services/reviewService';

const router = Router({ mergeParams: true });

router.get(
  '/',
  asyncHandler(async (req, res) => {
    const productId = String(req.params.id);
    res.json(reviewService.listReviews(productId));
  }),
);

router.post(
  '/',
  requireAuth,
  asyncHandler(async (req, res) => {
    const productId = String(req.params.id);
    const result = reviewService.createReview(req.user!.userId, productId, req.body);
    res.status(201).json(result);
  }),
);

export default router;
