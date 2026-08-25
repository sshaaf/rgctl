import { v4 as uuidv4 } from 'uuid';
import { getDb } from '../db';
import { User } from '../models/user';
import * as userRepository from '../repositories/userRepository';
import { AppError } from '../utils/errors';
import * as jwt from '../utils/jwt';
import * as password from '../utils/password';
import { nowIso } from '../utils/time';

export interface RegisterRequest {
  email: string;
  password: string;
  name: string;
}

export interface LoginRequest {
  email: string;
  password: string;
}

export interface AuthResponse {
  token: string;
  user_id: string;
  email: string;
  name: string;
}

export function register(req: RegisterRequest, jwtSecret: string): AuthResponse {
  const db = getDb();

  if (userRepository.findUserByEmail(db, req.email)) {
    throw AppError.conflict('email already registered');
  }

  const user: User = {
    id: uuidv4(),
    email: req.email,
    password_hash: password.hashPassword(req.password),
    name: req.name,
    role: 'customer',
    created_at: nowIso(),
  };

  userRepository.createUser(db, user);

  const token = jwt.signToken(user.id, user.email, user.role, jwtSecret);
  return { token, user_id: user.id, email: user.email, name: user.name };
}

export function login(req: LoginRequest, jwtSecret: string): AuthResponse {
  const db = getDb();
  const user = userRepository.findUserByEmail(db, req.email);

  if (!user || !password.verifyPassword(req.password, user.password_hash)) {
    throw AppError.unauthorized();
  }

  const token = jwt.signToken(user.id, user.email, user.role, jwtSecret);
  return { token, user_id: user.id, email: user.email, name: user.name };
}
