FROM python:3.12-slim-bookworm
ENV DEBIAN_FRONTEND=noninteractive \
    PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 10001 --create-home --shell /usr/sbin/nologin alvenqis
WORKDIR /app
COPY exporter.py /app/exporter.py
USER alvenqis
EXPOSE 9101
HEALTHCHECK --interval=15s --timeout=5s --retries=8 --start-period=10s \
  CMD curl -fsS http://127.0.0.1:9101/health || exit 1
CMD ["python3", "/app/exporter.py"]
