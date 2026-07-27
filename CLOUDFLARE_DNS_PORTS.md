# Alvenqis Network - Cloudflare DNS si porturi VPS

## VPS principal

- IPv4 public: `144.91.81.81`
- Proxy HTTP intern: Nginx `gateway:8080`
- Cloudflare Tunnel: `alvenqis-control-plane`
- Domeniu principal: `dohotstudio.com`

Traficul web public foloseste Cloudflare Tunnel. Nginx nu expune porturi web direct
catre internet in configuratia standard. Nu se adauga Caddy si nu se configureaza
un al doilea reverse proxy pentru serviciile Alvenqis.

## Inregistrari administrate prin Cloudflare Tunnel

Scriptul `Blockchain-prototype/alvenqis-release/vps-control-plane/scripts/cloudflare-bootstrap.sh`
creeaza sau actualizeaza aceste inregistrari ca `CNAME`, cu proxy Cloudflare activ:

| Nume Cloudflare | Tip | Tinta | Proxy | Serviciu |
| --- | --- | --- | --- | --- |
| `dohotstudio.com` | `CNAME` | `<TUNNEL_ID>.cfargotunnel.com` | Proxied | Website principal |
| `www.dohotstudio.com` | `CNAME` | `<TUNNEL_ID>.cfargotunnel.com` | Proxied | Website principal |
| `explorer.dohotstudio.com` | `CNAME` | `<TUNNEL_ID>.cfargotunnel.com` | Proxied | Explorer web |
| `control.dohotstudio.com` | `CNAME` | `<TUNNEL_ID>.cfargotunnel.com` | Proxied | Master Admin Panel |
| `rpcnode.dohotstudio.com` | `CNAME` | `<TUNNEL_ID>.cfargotunnel.com` | Proxied | RPC public filtrat |
| `fleet.dohotstudio.com` | `CNAME` | `<TUNNEL_ID>.cfargotunnel.com` | Proxied | Fleet API |
| `grafana.dohotstudio.com` | `CNAME` | `<TUNNEL_ID>.cfargotunnel.com` | Proxied | Grafana |
| `prometheus.dohotstudio.com` | `CNAME` | `<TUNNEL_ID>.cfargotunnel.com` | Proxied | Prometheus |
| `pool.dohotstudio.com` | `CNAME` | `<TUNNEL_ID>.cfargotunnel.com` | Proxied | Pool HTTP API |

Nu introduce manual valoarea `<TUNNEL_ID>`. Ruleaza bootstrap-ul Cloudflare dupa
ce noul `gateway` este sanatos; scriptul citeste ID-ul tunelului si configureaza
ingress-ul automat.

## Inregistrari obligatoriu DNS-only

| Nume Cloudflare | Tip | Tinta | Proxy | Port public | Protocol |
| --- | --- | --- | --- | --- | --- |
| `node.dohotstudio.com` | `A` | `144.91.81.81` | DNS only | `20787` | P2P TCP |
| `stratum.dohotstudio.com` | `A` | `144.91.81.81` | DNS only | `3333` | Stratum TLS TCP |

Aceste doua inregistrari nu trebuie sa aiba norul portocaliu activ. Proxy-ul HTTP
Cloudflare standard nu transporta transparent protocoalele P2P sau Stratum TCP.
Clientii Stratum trebuie configurati exclusiv cu:

```text
stratum+tls://stratum.dohotstudio.com:3333
```

Conexiunile plaintext catre portul `3333` sunt respinse intentionat. Nu exista un
port Stratum plaintext public.

## Firewall VPS

Permite inbound numai:

- `22/tcp` din IP-uri administrative de incredere, pentru SSH.
- `20787/tcp` public, pentru nodurile P2P.
- `3333/tcp` public, pentru Stratum TLS.

Cloudflare Tunnel initiaza conexiuni outbound; serviciile web Alvenqis nu necesita
porturile `80` sau `443` expuse direct. Daca `80/443` sunt folosite de 1Panel sau
alte aplicatii protejate, acestea raman independente de reteaua Docker Alvenqis.

## Verificare dupa deploy

```bash
dig +short dohotstudio.com
dig +short explorer.dohotstudio.com
dig +short node.dohotstudio.com
dig +short stratum.dohotstudio.com
curl -fsS https://dohotstudio.com/healthz
curl -fsS https://explorer.dohotstudio.com/healthz
curl -fsS https://rpcnode.dohotstudio.com/health
openssl s_client -connect stratum.dohotstudio.com:3333 \
  -servername stratum.dohotstudio.com -brief
```

Pentru `node.dohotstudio.com` si `stratum.dohotstudio.com`, rezultatul DNS trebuie
sa fie direct `144.91.81.81`. Pentru domeniile web, raspunsul public este servit
de reteaua Cloudflare si nu trebuie sa expuna direct IP-ul VPS.
