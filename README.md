# docker-git-http-server

Docker image for a Git HTTP server on Nginx.

Hosting git-http-backend & gitweb.

## Usage

Launch git-http-server with `docker`:

```
$ docker build . -t git-http-server
$ docker run \
  -d \
  -v $(pwd)/repos:/srv/git \
  -p "8080:80" \
  git-http-server
```

Launch git-http-server with `docker-compose`:

```console
$ docker-compose up -d
```
