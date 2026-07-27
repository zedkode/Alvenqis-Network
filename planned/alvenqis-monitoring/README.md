# alvenqis-monitoring

Operational metrics, dashboards and alerts for Alvenqis Network live under the
Docker control plane package:

**`alvenqis-release/vps-control-plane/monitoring/`**

That tree includes:

- Prometheus scrape config and alert rules
- Blackbox exporter modules
- Grafana provisioning + dashboard JSON (`alvenqis-docker-overview`, `alvenqis-host`)
- Loki / Alloy log pipeline
- Companion service `docker/metrics-exporter` (RPC/pool JSON → Prometheus)

See [vps-control-plane/monitoring/README.md](../../Blockchain-prototype/alvenqis-release/vps-control-plane/monitoring/README.md)
for Prometheus jobs, metric names, datasource UIDs and operator import steps.

Status: Mainnet Candidate / Prototype (not Mainnet Live).
