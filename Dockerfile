FROM rust:alpine AS builder
WORKDIR /usr/src/
COPY ./crates/repomng /usr/src/repomng
RUN apk add \
    musl-dev \
    openssl-dev \
    ;
RUN RUSTFLAGS="-C target-feature=-crt-static" cargo install --path ./repomng

FROM nginx:alpine

RUN apk --no-cache --update upgrade
RUN apk add --no-cache \
    fcgiwrap \
    highlight \
    git \
    git-daemon \
    git-gitweb \
    perl-cgi \
    spawn-fcgi \
    sudo \
    ;

RUN adduser git -h /var/lib/git -D
RUN adduser nginx git

RUN git config --system http.receivepack true
RUN git config --system http.uploadpack true
RUN git config --system user.email "gitserver@git.com"
RUN git config --system user.name "Git Server"

ADD ./scripts/entrypoint.sh /git-http-server-entrypoint.sh
ADD ./etc/gitweb.conf /etc/gitweb.conf
ADD ./etc/nginx/conf.d/* /etc/nginx/conf.d/
COPY --from=builder /usr/local/cargo/bin/repomng /usr/local/bin/repomng

ENTRYPOINT ["/git-http-server-entrypoint.sh"]
CMD ["nginx", "-g", "daemon off;"]
