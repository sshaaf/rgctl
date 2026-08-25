import cors from 'cors';
import dotenv from 'dotenv';
import express from 'express';
import { loadConfig } from './config';
import { closeDb, initDb } from './db';
import { errorHandler } from './middleware/errorHandler';
import routes from './routes';

dotenv.config();

const config = loadConfig();
initDb(config.databasePath);

const app = express();

app.use(cors());
app.use(express.json());
app.use(routes);
app.use(errorHandler);

const server = app.listen(config.port, () => {
  console.log(`ecommerce-typescript listening on port ${config.port}`);
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

export default app;
