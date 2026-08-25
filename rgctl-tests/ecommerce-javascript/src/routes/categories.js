const { Router } = require('express');
const { requireAuth } = require('../middleware/auth');
const { asyncHandler } = require('../middleware/errorHandler');
const categoryService = require('../services/categoryService');

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
    const result = categoryService.getCategory(req.params.id);
    res.json(result);
  }),
);

module.exports = router;
