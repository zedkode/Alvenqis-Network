# Alvenqis Web Deployment

The public web layer uses two independent, static SPA containers:

1. `alvenqis-website` for `dohotstudio.com` and `www.dohotstudio.com`
2. `alvenqis-explorer` for `explorer.dohotstudio.com`

Both images are built directly from their repository directories, run unprivileged Nginx on internal port `8080`, expose `GET /healthz`, and support a read-only root filesystem when `/tmp` is mounted as `tmpfs`.

## Compose Services

The VPS Compose stack should build the services with these contracts:

```yaml
services:
  alvenqis-website:
    build:
      context: ../../alvenqis-website
      args:
        VITE_API_BASE_URL: https://api.dohotstudio.com
        VITE_ALVENQIS_RPC_URL: https://rpcnode.dohotstudio.com
        VITE_ALVENQIS_POOL_URL: https://pool.dohotstudio.com
        VITE_ALVENQIS_EXPLORER_URL: https://explorer.dohotstudio.com
        VITE_ALVENQIS_WEBSITE_URL: https://dohotstudio.com
    read_only: true
    tmpfs:
      - /tmp:rw,noexec,nosuid,size=16m
    expose:
      - "8080"
    restart: unless-stopped

  alvenqis-explorer:
    build:
      context: ../../alvenqis-explorer
      args:
        VITE_ALVENQIS_RPC_URL: https://rpcnode.dohotstudio.com
        VITE_ALVENQIS_WEBSITE_URL: https://dohotstudio.com
    read_only: true
    tmpfs:
      - /tmp:rw,noexec,nosuid,size=16m
    expose:
      - "8080"
    restart: unless-stopped
```

The exact relative contexts depend on the location of the VPS Compose file. They must resolve to:

```text
Blockchain-prototype/alvenqis-website
Blockchain-prototype/alvenqis-explorer
```

Do not publish either container port directly to the public host. Only the Docker gateway should reach port `8080`.

## Gateway Nginx

The gateway and both SPA services must share the same Docker network. Route HTTP traffic by hostname:

```nginx
upstream alvenqis_website {
    server alvenqis-website:8080;
    keepalive 32;
}

upstream alvenqis_explorer {
    server alvenqis-explorer:8080;
    keepalive 32;
}

server {
    listen 8080;
    server_name dohotstudio.com www.dohotstudio.com;

    location / {
        proxy_pass http://alvenqis_website;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-Proto $http_x_forwarded_proto;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}

server {
    listen 8080;
    server_name explorer.dohotstudio.com;

    location / {
        proxy_pass http://alvenqis_explorer;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-Proto $http_x_forwarded_proto;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

TLS may terminate at Cloudflare, but the gateway remains the only HTTP origin exposed by the VPS stack.

## Cloudflare Hostnames

Create proxied HTTPS records/routes for:

| Public hostname | Gateway origin | Docker destination |
|---|---|---|
| `dohotstudio.com` | gateway HTTP listener | `alvenqis-website:8080` |
| `www.dohotstudio.com` | gateway HTTP listener | `alvenqis-website:8080` |
| `explorer.dohotstudio.com` | gateway HTTP listener | `alvenqis-explorer:8080` |

`explorer.dohotstudio.com` must exist in DNS before the website's **Open web explorer** action can work publicly.

The RPC and API hostnames remain separate services:

```text
rpcnode.dohotstudio.com
api.dohotstudio.com
pool.dohotstudio.com
```

Stratum is not an HTTP SPA route. `stratum.dohotstudio.com:3333` must follow the mining stack's DNS-only/TCP policy rather than this gateway configuration.

## Backend API

The website API remains an independent Node.js/PostgreSQL service. The static website is compiled with `VITE_API_BASE_URL=https://api.dohotstudio.com`; when that API is unavailable, public content uses the bundled static fallback.

Required API environment includes:

| Variable | Requirement |
|---|---|
| `DATABASE_URL` | PostgreSQL connection string |
| `JWT_SECRET` | Strong access-token signing secret |
| `JWT_REFRESH_SECRET` | Different strong refresh-token secret |
| `CORS_ORIGIN` | `https://dohotstudio.com,https://www.dohotstudio.com` |
| `ALVENQIS_RPC_URL` | Internal or protected Mainnet Candidate RPC origin |
| `NODE_ENV` | `production` |

Run Prisma migrations before starting a new API release and rotate initial admin credentials after seed.

## Build and Verification

Local frontend gates:

```bash
cd Blockchain-prototype/alvenqis-website
npm ci
npm run build

cd ../alvenqis-explorer
npm ci
npm run lint
npm run build
```

Container gates:

```bash
docker compose build alvenqis-website alvenqis-explorer
docker compose up -d alvenqis-website alvenqis-explorer gateway
docker compose exec -T alvenqis-website wget -qO- http://127.0.0.1:8080/healthz
docker compose exec -T alvenqis-explorer wget -qO- http://127.0.0.1:8080/healthz
```

Public verification:

```bash
curl -fsS https://dohotstudio.com/healthz
curl -fsS https://www.dohotstudio.com/healthz
curl -fsS https://explorer.dohotstudio.com/healthz
curl -fsS https://rpcnode.dohotstudio.com/health
```

Also verify:

- direct SPA requests such as `/desktop`, `/blocks/1`, `/tx/<hash>`, and `/address/<address>` return the correct application;
- `/wallet`, `/mining`, and `/downloads` redirect to sections of `/desktop`;
- the explorer displays an explicit unavailable state when RPC/indexer requests fail;
- public block, transaction, and address links remain shareable after a browser refresh;
- website and explorer containers remain healthy with `read_only: true`.
