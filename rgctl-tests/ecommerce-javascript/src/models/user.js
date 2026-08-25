function toPublicUser(user) {
  const { password_hash, ...publicUser } = user;
  return publicUser;
}

module.exports = { toPublicUser };
