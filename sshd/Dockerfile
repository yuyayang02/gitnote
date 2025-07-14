FROM alpine:latest

RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.aliyun.com/g' /etc/apk/repositories
RUN apk update 
RUN apk add --no-cache git openssh curl

WORKDIR /app

COPY entrypoint.sh /entrypoint.sh
COPY update /update
RUN chmod +x /entrypoint.sh
RUN chmod +x /update

EXPOSE 22

CMD ["/entrypoint.sh"]