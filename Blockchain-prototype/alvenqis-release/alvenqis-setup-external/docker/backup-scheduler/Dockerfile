FROM curlimages/curl:8.12.1
USER root
COPY docker/backup-scheduler/backup-loop.sh /usr/local/bin/backup-loop
RUN chmod 0755 /usr/local/bin/backup-loop
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/backup-loop"]
