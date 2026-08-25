export interface User {
  id: string;
  email: string;
  password_hash: string;
  name: string;
  role: string;
  created_at: string;
}

export interface UserPublic {
  id: string;
  email: string;
  name: string;
  role: string;
  created_at: string;
}

export function toPublicUser(user: User): UserPublic {
  const { password_hash: _, ...publicUser } = user;
  return publicUser;
}
