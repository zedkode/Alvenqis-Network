# Alvenqis Network Website

React + Vite + Tailwind CSS + Framer Motion + React Three Fiber prototype for the Alvenqis Network public website.

This is a fresh website project, built separately from previous website experiments.

## Production container

Build directly from this directory:

```bash
docker build -t alvenqis-website .
docker run --rm -p 8080:8080 --read-only --tmpfs /tmp:rw,noexec,nosuid,size=16m alvenqis-website
```

The Nginx runtime is unprivileged, serves the SPA on port `8080`, writes runtime files only below `/tmp`, and exposes `GET /healthz`.

Compose service:

```yaml
alvenqis-website:
  build:
    context: ./Blockchain-prototype/alvenqis-website
  read_only: true
  tmpfs:
    - /tmp:rw,noexec,nosuid,size=16m
  ports:
    - "8080:8080"
```

## Commands

```bash
npm install
npm run dev
npm run build
npm run server:test
```

## Backend Commands

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

From the repository root you can also run:

```bash
npm run server:cms:migrate
```

Frontend CMS config:

```bash
cp .env.example .env
```

`VITE_API_BASE_URL` should point to the Node API. If the API is down, public pages fall back to `src/data/content.js` so the website does not render empty.

## Pages

- `/` - premium ion/plasma landing page
- `/network` - layer architecture
- `/protocol` - economics and open protocol decisions
- `/core` - core protocol modules and engineering gates
- `/mining` - PoW mining, reward schedule and pool path
- `/wallet` - wallet UX and ownership surfaces
- `/explorer` - explorer product surface and API preview
- `/developers` - SDK, standards and developer stack
- `/tokenomics` - supply, rewards, halving and open token decisions
- `/faq` - readiness and product FAQ
- `/passport` - Alvenqis Passport proof layer
- `/ecosystem` - wallet, explorer, Passport, SDK and product layer
- `/whitepaper` - cinematic whitepaper preview
- `/roadmap` - phase roadmap
- `/docs` - documentation hub preview
- `/status` - honest readiness matrix

## Scope

This website follows the attached Alvenqis Source Info:

- public name: Alvenqis Network;
- ticker: ALVE;
- direction: Rust-based mineable Layer 1;
- block time: 60 seconds;
- max supply: 60,000,000 ALVE;
- public claims stay honest while protocol decisions are still open;
- palette direction: deep graphite, ion cyan, electric blue and violet plasma.
- includes smooth wheel scrolling, scroll progress, animated global background effects, multiple R3F scenes and scroll-driven roadmap progress.
- connects Explorer, Mining, Status and Tokenomics pages directly to the Alvenqis Mainnet Candidate Rust RPC.
- includes a lazy-loaded `/admin` panel for dashboard, users, content, network params, roadmap, FAQ and audit log management.

## Admin

Open `/admin` after the backend is running. The seeded superadmin can log in with the credentials from `server/.env`.

In development, `/admin` also has a local dev bypass when `VITE_ADMIN_DEV_BYPASS="true"`. It lets you open the admin UI without PostgreSQL/backend and includes local mock data for every admin menu. Real production data still needs the backend API and database.

Admin modules:

- Dashboard KPIs
- Users and permissions
- Content block editor with live preview
- Network parameters with emission chart
- Roadmap management with drag-and-drop ordering
- FAQ management
- Audit log viewer

## API Docs And Deploy

- OpenAPI JSON: `/openapi.json`
- Swagger UI: `/api/docs`
- Deploy notes: `DEPLOYMENT.md`
