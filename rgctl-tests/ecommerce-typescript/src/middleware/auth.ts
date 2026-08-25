import { NextFunction, Request, Response } from 'express';
import { AppError } from '../utils/errors';
import * as jwt from '../utils/jwt';

export interface AuthUser {
  userId: string;
  email: string;
  role: string;
}

declare global {
  namespace Express {
    interface Request {
      user?: AuthUser;
    }
  }
}

export function requireAuth(req: Request, _res: Response, next: NextFunction): void {
  try {
    const header = req.headers.authorization;
    if (!header?.startsWith('Bearer ')) {
      throw AppError.unauthorized();
    }

    const token = header.slice(7);
    const secret = process.env.JWT_SECRET ?? 'dev-secret-change-me';
    const claims = jwt.verifyToken(token, secret);

    req.user = {
      userId: claims.sub,
      email: claims.email,
      role: claims.role,
    };
    next();
  } catch (err) {
    if (err instanceof AppError) {
      next(err);
    } else {
      next(AppError.unauthorized());
    }
  }
}
