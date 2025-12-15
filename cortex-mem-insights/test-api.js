import { Elysia } from 'elysia';

const app = new Elysia()
  .get('/', () => 'Hello from Elysia')
  .get('/health', () => ({ status: 'healthy' }))
  .listen(3002, () => {
    console.log('🚀 Test API running on http://localhost:3002');
  });