import { Router } from 'express';
import { asyncHandler } from '../../middleware/errorHandler';
import type { CoolstoreProductService } from '../services/coolstoreProductService';

export function createProductRouter(productService: CoolstoreProductService): Router {
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
