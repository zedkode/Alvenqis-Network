FROM python:3.12-slim-bookworm
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends apache2-utils bash ca-certificates curl gnupg jq openssl rclone tar tzdata && install -m 0755 -d /etc/apt/keyrings && curl -fsSL https://download.docker.com/linux/debian/gpg -o /etc/apt/keyrings/docker.asc && chmod a+r /etc/apt/keyrings/docker.asc && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/debian bookworm stable" > /etc/apt/sources.list.d/docker.list && apt-get update && apt-get install -y --no-install-recommends docker-ce-cli docker-buildx-plugin docker-compose-plugin && python3 -m venv /opt/venv && /opt/venv/bin/pip install --no-cache-dir Flask==3.1.1 gunicorn==23.0.0 requests==2.32.4 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY docker/ops/app.py /app/app.py
COPY docker/ops/broker.py /app/broker.py
COPY docker/ops/templates /app/templates
COPY scripts /opt/alvenqis-scripts
RUN chmod 0755 /opt/alvenqis-scripts/*.sh
ENV PATH="/opt/venv/bin:${PATH}"
EXPOSE 8080 8090
ENTRYPOINT []
CMD ["gunicorn", "--bind", "0.0.0.0:8080", "--workers", "2", "--threads", "4", "--timeout", "900", "app:app"]
