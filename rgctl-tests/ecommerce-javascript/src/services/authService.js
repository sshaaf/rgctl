const { v4: uuidv4 } = require('uuid');
const { getDb } = require('../db');
const userRepository = require('../repositories/userRepository');
const { AppError } = require('../utils/errors');
const jwt = require('../utils/jwt');
const password = require('../utils/password');
const { nowIso } = require('../utils/time');

function register(req, jwtSecret) {
  const db = getDb();

  if (userRepository.findUserByEmail(db, req.email)) {
    throw AppError.conflict('email already registered');
  }

  const user = {
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

function login(req, jwtSecret) {
  const db = getDb();
  const user = userRepository.findUserByEmail(db, req.email);

  if (!user || !password.verifyPassword(req.password, user.password_hash)) {
    throw AppError.unauthorized();
  }

  const token = jwt.signToken(user.id, user.email, user.role, jwtSecret);
  return { token, user_id: user.id, email: user.email, name: user.name };
}

module.exports = { register, login };
