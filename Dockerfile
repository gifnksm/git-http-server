FROM nginx:alpine

RUN apk --update upgrade
RUN apk add \
    fcgiwrap \
    highlight \
    git \
    git-daemon \
    git-gitweb \
    perl-cgi \
    spawn-fcgi \
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

ENTRYPOINT ["/git-http-server-entrypoint.sh"]
CMD ["nginx", "-g", "daemon off;"]
