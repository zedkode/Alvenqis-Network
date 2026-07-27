# Alvenqis Network Server

Node.js + Express + PostgreSQL + Prisma backend foundation for Alvenqis Network.

## Scope

Implemented Phase 1 foundation:

- Prisma/PostgreSQL schema for users, audit logs, network parameters, content blocks, roadmap items and FAQ items.
- JWT auth with 15 minute access tokens.
- Refresh token rotation with 7 day httpOnly cookie.
- Login rate limiting: 5 attempts per 15 minutes.
- Role middleware for protected admin routes.
- Seed script for superadmin and default network parameters.

Implemented Phase 2 CMS foundation:

- Public content API backed by `content_blocks`.
- Admin CRUD for content blocks, roadmap items and FAQ items.
- Zod validation on CMS payloads.
- Audit log entries for admin mutations.
- Migration script from the current React `src/data/content.js` file.

Implemented Mainnet Candidate network adapter:

- Read-only Rust RPC provider behind the `INetworkProvider` abstraction.
- No synthetic block worker and no generated network history.
- Candidate block, supply and reward data read from `ALVENQIS_RPC_URL`.
- Public explorer/mining/status API labeled as `mainnet_candidate`.

Implemented Phase 4 admin backend foundation:

- User CRUD, role assignment and deactivation.
- Audit log API with filters.
- Network parameters API with confirmation for critical economics updates.
- Admin dashboard KPI API.
- `/auth/me` session validation endpoint for the admin guard.

## Setup

```bash
cd server
cp .env.example .env
npm install
npm run prisma:generate
npm run prisma:migrate
npm run seed
npm run cms:migrate
npm run dev
```

## Tests

```bash
npm run test
```

From the repository root:

```bash
npm run server:dev
```

## Default Seed Values

- `block_time_seconds`: `60`
- `max_supply`: `60000000`
- `halving_interval`: `1576800`
- `current_reward`: `19.02587519`
- `network_mode`: `mainnet_candidate`
- `ticker`: `ALVE`

## Auth Endpoints

```text
POST /auth/login
POST /auth/refresh
POST /auth/logout
GET  /health
GET  /auth/me
```

## CMS Endpoints

```text
GET    /api/content/:page_slug?lang=en
GET    /api/admin/content
POST   /api/admin/content
PUT    /api/admin/content/:id
DELETE /api/admin/content/:id

GET    /api/roadmap
GET    /api/admin/roadmap
POST   /api/admin/roadmap
PUT    /api/admin/roadmap/:id
DELETE /api/admin/roadmap/:id

GET    /api/faq?lang=en
GET    /api/admin/faq
POST   /api/admin/faq
PUT    /api/admin/faq/:id
DELETE /api/admin/faq/:id
```

Admin CMS routes require a Bearer access token and the `content_editor` role. `superadmin` is always allowed by the role middleware.

## Mainnet Candidate Network Endpoints

```text
GET /api/network/blocks?limit=20&offset=0
GET /api/network/blocks/:height
GET /api/network/stats
```

All responses include `mode: "mainnet_candidate"` and are backed by the Rust RPC. Candidate status is not a public mainnet launch claim.

## Admin Operations Endpoints

```text
GET    /auth/me

GET    /api/admin/dashboard

GET    /api/admin/users
POST   /api/admin/users
PUT    /api/admin/users/:id

GET    /api/admin/audit-log?action=&userId=&from=&to=

GET    /api/admin/network-params
PUT    /api/admin/network-params/:key
```

User and audit endpoints require `superadmin`. Network parameter endpoints require `network_operator` or `superadmin`.

## API Docs

```text
GET /openapi.json
GET /api/docs
```

The server is a backend foundation only. It does not claim or expose a real blockchain network.
