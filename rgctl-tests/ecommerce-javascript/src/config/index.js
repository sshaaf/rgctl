function loadConfig() {
  return {
    databasePath: process.env.DATABASE_PATH ?? 'ecommerce.db',
    jwtSecret: process.env.JWT_SECRET ?? 'dev-secret-change-me',
    port: parseInt(process.env.PORT ?? '3000', 10),
  };
}

module.exports = { loadConfig };
