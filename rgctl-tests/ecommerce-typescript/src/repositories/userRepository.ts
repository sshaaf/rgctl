import Database from 'better-sqlite3';
import { User } from '../models/user';

export function createUser(db: Database.Database, user: User): User {
  db.prepare(
    `INSERT INTO users (id, email, password_hash, name, role, created_at)
     VALUES (?, ?, ?, ?, ?, ?)`,
  ).run(user.id, user.email, user.password_hash, user.name, user.role, user.created_at);
  return user;
}

export function findUserByEmail(db: Database.Database, email: string): User | undefined {
  return db
    .prepare(
      'SELECT id, email, password_hash, name, role, created_at FROM users WHERE email = ?',
    )
    .get(email) as User | undefined;
}

export function findUserById(db: Database.Database, id: string): User | undefined {
  return db
    .prepare(
      'SELECT id, email, password_hash, name, role, created_at FROM users WHERE id = ?',
    )
    .get(id) as User | undefined;
}
