const { Router } = require('express');
const { asyncHandler } = require('../../middleware/errorHandler');

function createCartRouter(shoppingCartService) {
  const router = Router();

  router.get(
    '/:cartId',
    asyncHandler(async (req, res) => {
      res.json(shoppingCartService.getShoppingCart(String(req.params.cartId)));
    }),
  );

  router.post(
    '/checkout/:cartId',
    asyncHandler(async (req, res) => {
      res.json(shoppingCartService.checkOutShoppingCart(String(req.params.cartId)));
    }),
  );

  router.post(
    '/:cartId/:itemId/:quantity',
    asyncHandler(async (req, res) => {
      const quantity = Number.parseInt(String(req.params.quantity), 10);
      res.json(
        shoppingCartService.addItem(
          String(req.params.cartId),
          String(req.params.itemId),
          quantity,
        ),
      );
    }),
  );

  router.delete(
    '/:cartId/:itemId/:quantity',
    asyncHandler(async (req, res) => {
      const quantity = Number.parseInt(String(req.params.quantity), 10);
      res.json(
        shoppingCartService.deleteItem(
          String(req.params.cartId),
          String(req.params.itemId),
          quantity,
        ),
      );
    }),
  );

  return router;
}

module.exports = { createCartRouter };
