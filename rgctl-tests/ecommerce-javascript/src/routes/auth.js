const { Router } = require('express');
const { loadConfig } = require('../config');
const { asyncHandler } = require('../middleware/errorHandler');
const authService = require('../services/authService');

const router = Router();

router.post(
  '/register',
  asyncHandler(async (req, res) => {
    const config = loadConfig();
    const result = authService.register(req.body, config.jwtSecret);
    res.status(201).json(result);
  }),
);

router.post(
  '/login',
  asyncHandler(async (req, res) => {
    const config = loadConfig();
    const result = authService.login(req.body, config.jwtSecret);
    res.json(result);
  }),
);

module.exports = router;
