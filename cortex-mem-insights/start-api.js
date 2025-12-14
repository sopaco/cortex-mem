import { app } from './src/server/index.js';

const port = process.env.PORT ? parseInt(process.env.PORT) : 3001;

app.listen(port, () => {
  console.log(`🚀 cortex-mem-insights API 运行在 http://localhost:${port}`);
});