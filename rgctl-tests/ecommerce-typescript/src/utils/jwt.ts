import jwt from 'jsonwebtoken';

export interface JwtClaims {
  sub: string;
  email: string;
  role: string;
}

export function signToken(
  userId: string,
  email: string,
  role: string,
  secret: string,
): string {
  const claims: JwtClaims & { exp: number } = {
    sub: userId,
    email,
    role,
    exp: Math.floor(Date.now() / 1000) + 24 * 60 * 60,
  };
  return jwt.sign(claims, secret);
}

export function verifyToken(token: string, secret: string): JwtClaims {
  return jwt.verify(token, secret) as JwtClaims;
}
