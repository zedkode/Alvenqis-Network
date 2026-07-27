export const openApiDocument = {
  openapi: '3.0.3',
  info: {
    title: 'Alvenqis Network API',
    version: '0.5.0',
    description: 'API for the Alvenqis website, CMS, admin panel and read-only Alvenqis Mainnet Candidate RPC adapter.',
  },
  servers: [
    { url: 'http://localhost:4000', description: 'Local development' },
  ],
  tags: [
    { name: 'Auth' },
    { name: 'Content' },
    { name: 'Network' },
    { name: 'Admin' },
  ],
  components: {
    securitySchemes: {
      bearerAuth: {
        type: 'http',
        scheme: 'bearer',
        bearerFormat: 'JWT',
      },
    },
  },
  paths: {
    '/health': {
      get: {
        tags: ['Auth'],
        summary: 'Health check',
        responses: { 200: { description: 'Server is healthy' } },
      },
    },
    '/auth/login': {
      post: {
        tags: ['Auth'],
        summary: 'Login with email and password',
        requestBody: {
          required: true,
          content: {
            'application/json': {
              schema: {
                type: 'object',
                required: ['email', 'password'],
                properties: {
                  email: { type: 'string', format: 'email' },
                  password: { type: 'string', minLength: 8 },
                },
              },
            },
          },
        },
        responses: {
          200: { description: 'Returns user and access token; sets refresh cookie' },
          401: { description: 'Invalid credentials' },
          429: { description: 'Too many login attempts' },
        },
      },
    },
    '/auth/refresh': {
      post: {
        tags: ['Auth'],
        summary: 'Rotate refresh cookie and return a new access token',
        responses: { 200: { description: 'Session refreshed' }, 401: { description: 'Invalid refresh token' } },
      },
    },
    '/auth/logout': {
      post: {
        tags: ['Auth'],
        summary: 'Revoke refresh token and clear cookie',
        responses: { 204: { description: 'Logged out' } },
      },
    },
    '/auth/me': {
      get: {
        tags: ['Auth'],
        summary: 'Return authenticated admin user',
        security: [{ bearerAuth: [] }],
        responses: { 200: { description: 'Current user' }, 401: { description: 'Unauthenticated' } },
      },
    },
    '/api/content/{page_slug}': {
      get: {
        tags: ['Content'],
        summary: 'Read public CMS content for one page',
        parameters: [
          { name: 'page_slug', in: 'path', required: true, schema: { type: 'string' } },
          { name: 'lang', in: 'query', schema: { type: 'string', default: 'en' } },
        ],
        responses: { 200: { description: 'Page content blocks merged into sections' } },
      },
    },
    '/api/network/blocks': {
      get: {
        tags: ['Network'],
        summary: 'List Mainnet Candidate blocks',
        parameters: [
          { name: 'limit', in: 'query', schema: { type: 'integer', default: 20, maximum: 100 } },
          { name: 'offset', in: 'query', schema: { type: 'integer', default: 0 } },
        ],
        responses: { 200: { description: 'Candidate block list with mode=mainnet_candidate' } },
      },
    },
    '/api/network/blocks/{height}': {
      get: {
        tags: ['Network'],
        summary: 'Read one Mainnet Candidate block',
        parameters: [{ name: 'height', in: 'path', required: true, schema: { type: 'integer' } }],
        responses: { 200: { description: 'Candidate block with mode=mainnet_candidate' }, 404: { description: 'Block not found' } },
      },
    },
    '/api/network/stats': {
      get: {
        tags: ['Network'],
        summary: 'Read Mainnet Candidate stats',
        responses: { 200: { description: 'Candidate stats with mode=mainnet_candidate' } },
      },
    },
    '/api/admin/dashboard': {
      get: {
        tags: ['Admin'],
        summary: 'Admin KPI dashboard',
        security: [{ bearerAuth: [] }],
        responses: { 200: { description: 'Dashboard KPIs and latest audit logs' } },
      },
    },
    '/api/admin/users': {
      get: {
        tags: ['Admin'],
        summary: 'List users',
        security: [{ bearerAuth: [] }],
        responses: { 200: { description: 'Users list' } },
      },
      post: {
        tags: ['Admin'],
        summary: 'Create user',
        security: [{ bearerAuth: [] }],
        responses: { 201: { description: 'User created' } },
      },
    },
    '/api/admin/network-params': {
      get: {
        tags: ['Admin'],
        summary: 'List network parameters',
        security: [{ bearerAuth: [] }],
        responses: { 200: { description: 'Network params' } },
      },
    },
  },
}
