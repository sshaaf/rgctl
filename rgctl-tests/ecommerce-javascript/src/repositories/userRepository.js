function createUser(db, user) {
  db.prepare(
    `INSERT INTO users (id, email, password_hash, name, role, created_at)
     VALUES (?, ?, ?, ?, ?, ?)`,
  ).run(user.id, user.email, user.password_hash, user.name, user.role, user.created_at);
  return user;
}

function findUserByEmail(db, email) {
  return db
    .prepare(
      'SELECT id, email, password_hash, name, role, created_at FROM users WHERE email = ?',
    )
    .get(email);
}

function findUserById(db, id) {
  return db
    .prepare(
      'SELECT id, email, password_hash, name, role, created_at FROM users WHERE id = ?',
    )
    .get(id);
}

module.exports = { createUser, findUserByEmail, findUserById };
