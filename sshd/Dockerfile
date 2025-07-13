FROM alpine:latest

ENV GIT_USER=git \
    REPO_NAME=gitnote.git

RUN apk add --no-cache \
    openssh \
    git \
    curl

WORKDIR /app

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

EXPOSE 22

CMD ["/entrypoint.sh"]