import { Router } from 'express';
import { loadConfig } from '../config';
import { asyncHandler } from '../middleware/errorHandler';
import * as authService from '../services/authService';

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

export default router;
