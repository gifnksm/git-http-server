#!/bin/sh

set -eux

readonly GIT_PROJECT_ROOT="/srv/git"
readonly HTTP_USER=nginx
readonly HTTP_GROUP=nginx
readonly GIT_USER=nginx
readonly GIT_GROUP=nginx

readonly FCGI_SOCKET=/var/run/fcgiwrap.sock
readonly FCGI_PROGRAM=/usr/bin/fcgiwrap

env -i /usr/bin/spawn-fcgi \
  -s "${FCGI_SOCKET}" \
  -F 4 \
  -u "${HTTP_USER}" \
  -g "${HTTP_GROUP}" \
  -U "${HTTP_USER}" \
  -G "${HTTP_GROUP}" \
  -- \
  "${FCGI_PROGRAM}"

exec /docker-entrypoint.sh "$@"
