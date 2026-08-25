require('dotenv').config();

const cors = require('cors');
const express = require('express');
const { loadConfig } = require('./config');
const { closeDb, initDb } = require('./db');
const { errorHandler } = require('./middleware/errorHandler');
const routes = require('./routes');

const config = loadConfig();
initDb(config.databasePath);

const app = express();

app.use(cors());
app.use(express.json());
app.use(routes);
app.use(errorHandler);

const server = app.listen(config.port, () => {
  console.log(`ecommerce-javascript listening on port ${config.port}`);
});

process.on('SIGINT', () => {
  server.close();
  closeDb();
  process.exit(0);
});

process.on('SIGTERM', () => {
  server.close();
  closeDb();
  process.exit(0);
});

module.exports = app;
