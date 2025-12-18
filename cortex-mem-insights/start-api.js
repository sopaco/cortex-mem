import { app } from './src/server/index.js';

const port = process.env.PORT ? parseInt(process.env.PORT) : 15173;

app.listen(port, () => {
	console.log(`🚀 cortex-mem-insights API 运行在 http://localhost:${port}`);
});
