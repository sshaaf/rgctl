export class AppError extends Error {
  constructor(
    message: string,
    public readonly statusCode: number = 500,
  ) {
    super(message);
    this.name = 'AppError';
  }

  static notFound(message = 'not found'): AppError {
    return new AppError(message, 404);
  }

  static unauthorized(message = 'unauthorized'): AppError {
    return new AppError(message, 401);
  }

  static badRequest(message: string): AppError {
    return new AppError(message, 400);
  }

  static conflict(message: string): AppError {
    return new AppError(message, 409);
  }
}
