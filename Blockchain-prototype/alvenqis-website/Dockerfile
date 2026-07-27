FROM node:22-alpine AS build

WORKDIR /app
ARG VITE_API_BASE_URL=https://api.dohotstudio.com
ARG VITE_ALVENQIS_RPC_URL=https://rpcnode.dohotstudio.com
ARG VITE_ALVENQIS_POOL_URL=https://pool.dohotstudio.com
ARG VITE_ALVENQIS_EXPLORER_URL=https://explorer.dohotstudio.com
ARG VITE_ALVENQIS_WEBSITE_URL=https://dohotstudio.com
ENV VITE_API_BASE_URL=$VITE_API_BASE_URL \
    VITE_ALVENQIS_RPC_URL=$VITE_ALVENQIS_RPC_URL \
    VITE_ALVENQIS_POOL_URL=$VITE_ALVENQIS_POOL_URL \
    VITE_ALVENQIS_EXPLORER_URL=$VITE_ALVENQIS_EXPLORER_URL \
    VITE_ALVENQIS_WEBSITE_URL=$VITE_ALVENQIS_WEBSITE_URL
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM nginxinc/nginx-unprivileged:1.27-alpine

COPY nginx.conf /etc/nginx/nginx.conf
COPY --from=build /app/dist /usr/share/nginx/html

EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget -qO- http://127.0.0.1:8080/healthz >/dev/null || exit 1

ENTRYPOINT []
CMD ["nginx", "-g", "daemon off;"]
