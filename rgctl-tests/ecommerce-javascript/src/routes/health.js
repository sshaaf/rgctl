const { Router } = require('express');

const router = Router();

router.get('/health', (_req, res) => {
  res.json({ status: 'ok', service: 'ecommerce-javascript' });
});

module.exports = router;
