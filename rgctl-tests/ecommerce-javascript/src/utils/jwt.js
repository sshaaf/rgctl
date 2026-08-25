const jwt = require('jsonwebtoken');

function signToken(userId, email, role, secret) {
  const claims = {
    sub: userId,
    email,
    role,
    exp: Math.floor(Date.now() / 1000) + 24 * 60 * 60,
  };
  return jwt.sign(claims, secret);
}

function verifyToken(token, secret) {
  return jwt.verify(token, secret);
}

module.exports = { signToken, verifyToken };
