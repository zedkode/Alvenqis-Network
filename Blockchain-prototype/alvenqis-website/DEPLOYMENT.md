# Alvenqis Network Deployment

This project is split into three deployable parts:

1. Static React website and admin panel
2. Node.js API server
3. PostgreSQL database

The public site reads the Alvenqis Mainnet Candidate RPC. This is candidate-chain data and must not be described as a live public mainnet until launch gates pass.

## Production Website Runtime (1Panel)

Create a Node.js 24 website/runtime in 1Panel from this directory. The runtime
publishes port `18081` only on host loopback and joins `1panel-network`;
Cloudflare Tunnel reaches it by the container name
`alvenqis-website-runtime`.

Install/build command:

```bash
npm ci
npm run build
```

Start command:

```bash
npm run start
```

The control-plane automation creates or updates the runtime through the
official 1Panel API and restores the API to its previous disabled state:

```bash
cd /opt/alvenqis/repo/Blockchain-prototype/alvenqis-release/vps-control-plane
./scripts/configure-1panel-website-runtime.sh
```

Runtime environment:

```text
NODE_ENV=production
HOST=0.0.0.0
PORT=18081
VITE_ALVENQIS_RPC_URL=https://rpcnode.dohotstudio.com
VITE_ALVENQIS_POOL_URL=https://pool.dohotstudio.com
```

`scripts/serve-production.mjs` serves `dist`, provides SPA fallback routing and
exposes `GET /healthz`. Do not use `vite preview` as the production runtime.

The `/admin` panel is part of the same build. Set
`VITE_API_BASE_URL=https://api.dohotstudio.com` only after the API runtime and
database are deployed; public content has a static fallback while it is absent.

## Cloudflare

The Docker control-plane tunnel owns both website hostnames:

```text
dohotstudio.com     -> http://alvenqis-website-runtime:18081
www.dohotstudio.com -> http://alvenqis-website-runtime:18081
```

Re-apply the tunnel configuration after every hostname/origin change:

```bash
cd /opt/alvenqis/vps-control-plane
scripts/cloudflare-bootstrap.sh --activate
```

The Stratum endpoint is deliberately different: `stratum.dohotstudio.com:3333`
is a DNS-only `A` record because standard Cloudflare Tunnel is not a transparent
proxy for arbitrary public TCP. TLS is terminated by the mining pool.

## Backend Hosting

Recommended runtime:

```text
Node.js 24
```

Build/start commands:

```bash
cd server
npm install
npm run prisma:generate
npm run prisma:migrate
npm run seed
npm run cms:migrate
npm run start
```

Production backend environment variables:

| Variable | Required | Example | Notes |
|---|---:|---|---|
| `DATABASE_URL` | yes | `postgresql://user:pass@host:5432/alvenqis?schema=public` | PostgreSQL connection string |
| `JWT_SECRET` | yes | long random secret | Access token signing secret |
| `JWT_REFRESH_SECRET` | yes | different long random secret | Refresh token signing secret |
| `PORT` | no | `4000` | API server port |
| `CORS_ORIGIN` | yes | `https://dohotstudio.com,https://www.dohotstudio.com` | Comma-separated allowlist |
| `RATE_LIMIT_WINDOW_MS` | no | `900000` | Default 15 minutes |
| `RATE_LIMIT_MAX` | no | `300` | Global request limit per window |
| `NODE_ENV` | yes | `production` | Enables secure refresh cookie |
| `ALVENQIS_RPC_URL` | yes | `http://127.0.0.1:10787` | Mainnet Candidate Rust RPC used by the read-only network adapter |
| `DEFAULT_ADMIN_EMAIL` | first deploy | `admin@alvenqis.network` | Used by seed |
| `DEFAULT_ADMIN_PASSWORD` | first deploy | strong password | Rotate after first login |

## PostgreSQL

Use a managed PostgreSQL service or a separately hosted database. The server expects normal Prisma migrations against PostgreSQL.

Operational notes:

- Run migrations before starting a new backend release.
- Keep database backups enabled.
- Rotate `DEFAULT_ADMIN_PASSWORD` after seed by creating a new superadmin or changing the user through the admin panel.
- Restrict direct DB access to the backend host/provider.

## API Documentation

After backend deploy:

```text
GET /openapi.json
GET /api/docs
```

## Verification Checklist

```bash
npm run build
npm run server:test
```

Manual checks:

- `http://127.0.0.1:18081/healthz` returns the website health payload on the VPS.
- `https://dohotstudio.com/` and `https://www.dohotstudio.com/` return the same current build.
- `/downloads`, `/explorer` and `/mining` load through SPA fallback after a direct request.
- `/explorer` reads the public Rust RPC.
- `/mining` reads the pool status API while work/share submission remains Stratum TLS only.
- `/api/network/stats` returns `mode: "mainnet_candidate"` and matches the Rust RPC height.
- `/admin` login works with seeded superadmin.
- CMS pages still render if the API is temporarily down because the frontend has static fallback content.
