# Deploy to Heroku

Heroku runs containers via the container stack. Backers works, but with an
important caveat: **Heroku has no persistent filesystem**, so `data.db` and
uploads are wiped on every dyno restart.

Good for demos and evaluation; for production data use [Docker](with-docker.md),
[Render](paas-render.md), or a VM.

## Container deploy

Heroku builds from a `heroku.yml` at the repo root — add this file:

```yaml
build:
  docker:
    web: Dockerfile
run:
  web: backers serve
```

Then:

```sh
heroku create my-backers
heroku stack:set container -a my-backers
git push heroku main
heroku ps:scale web=1 -a my-backers
heroku open -a my-backers
```

Heroku assigns `$PORT` dynamically — Backers reads `PORT`, so it binds correctly
with no changes.

## Environment variables

```sh
heroku config:set JWT_SECRET="$(openssl rand -hex 32)" -a my-backers
```

`BACKERS_DATA` defaults to `~/.backers` inside the dyno — ephemeral. Optionally
set it explicitly for clarity:

```sh
heroku config:set BACKERS_DATA=/app/data -a my-backers
```

## Surviving restarts

Since the filesystem is ephemeral, schedule backups to durable storage:

- `GET /api/backup` with an admin token → upload the snapshot to S3/R2 on a
  schedule (e.g. Heroku Scheduler + a small script).
- On boot, restore by downloading the snapshot to `$BACKERS_DATA/data.db`.

## Alternative: buildpack

There is no official Rust buildpack guarantee; the container stack above is the
reliable path. If you prefer buildpacks, the community
[`heroku-buildpack-rust`](https://github.com/emk/heroku-buildpack-rust) can
compile the workspace, but you'll still fight the ephemeral-disk problem.
