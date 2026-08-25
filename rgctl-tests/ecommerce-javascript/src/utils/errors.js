class AppError extends Error {
  constructor(message, statusCode = 500) {
    super(message);
    this.name = 'AppError';
    this.statusCode = statusCode;
  }

  static notFound(message = 'not found') {
    return new AppError(message, 404);
  }

  static unauthorized(message = 'unauthorized') {
    return new AppError(message, 401);
  }

  static badRequest(message) {
    return new AppError(message, 400);
  }

  static conflict(message) {
    return new AppError(message, 409);
  }
}

module.exports = { AppError };
