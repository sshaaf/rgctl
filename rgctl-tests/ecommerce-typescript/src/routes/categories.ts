import { Router } from 'express';
import { requireAuth } from '../middleware/auth';
import { asyncHandler } from '../middleware/errorHandler';
import * as categoryService from '../services/categoryService';

const router = Router();

router.get(
  '/',
  asyncHandler(async (_req, res) => {
    res.json(categoryService.listCategories());
  }),
);

router.post(
  '/',
  requireAuth,
  asyncHandler(async (req, res) => {
    const result = categoryService.createCategory(req.body);
    res.status(201).json(result);
  }),
);

router.get(
  '/:id',
  asyncHandler(async (req, res) => {
    const result = categoryService.getCategory(String(req.params.id));
    res.json(result);
  }),
);

export default router;
