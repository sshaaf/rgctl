const { AppError } = require('../utils/errors');
const jwt = require('../utils/jwt');

function requireAuth(req, _res, next) {
  try {
    const header = req.headers.authorization;
    if (!header || !header.startsWith('Bearer ')) {
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

module.exports = { requireAuth };
