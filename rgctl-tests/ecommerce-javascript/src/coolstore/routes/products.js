const { Router } = require('express');
const { asyncHandler } = require('../../middleware/errorHandler');

function createProductRouter(productService) {
  const router = Router();

  router.get(
    '/',
    asyncHandler(async (_req, res) => {
      res.json(productService.getProducts());
    }),
  );

  router.get(
    '/:itemId',
    asyncHandler(async (req, res) => {
      const product = productService.getProductByItemId(String(req.params.itemId));
      res.json(product ?? null);
    }),
  );

  return router;
}

module.exports = { createProductRouter };
