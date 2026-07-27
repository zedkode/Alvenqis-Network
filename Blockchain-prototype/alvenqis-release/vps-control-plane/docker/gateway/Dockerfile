FROM nginx:1.29.8-alpine

RUN apk add --no-cache apache2-utils gettext

COPY gateway-entrypoint.sh /usr/local/bin/alvenqis-gateway-entrypoint
COPY nginx.conf.template /etc/alvenqis/nginx.conf.template

RUN chmod 0555 /usr/local/bin/alvenqis-gateway-entrypoint

ENTRYPOINT ["/usr/local/bin/alvenqis-gateway-entrypoint"]
