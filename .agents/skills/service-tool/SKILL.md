---
name: service-tool
description: Use `ser` whenever you need to run a server on localhost, such as an API, a web app, a dev server, or a notebook. Also use it when a localhost connection fails and the service may be down, or when a port is already in use.
---

# Service tool

`ser` runs and tracks servers on localhost. Run `ser --help` for the commands and `ser <command> --help` for their flags. The help is the source of truth.

## Rules

### Run servers through ser

Never start a server with a bare command, with `&`, or with `nohup`. Those leave processes nobody can see or stop. Use `ser run -d` so the server is tracked, logged, and given a free port.

### Never hardcode a port

The command must take its port from `{port}` or from the `PORT` environment variable. The port a service gets is not always the one requested. Ask `ser` for it every time.

### Read the log before guessing

When a service is down or unhealthy, run `ser show <name>` and then `ser log <name>` before restarting or changing anything. A restart does not fix a crash caused by the code or the environment.

### Do not touch the state directory

Never edit `ser.db` or kill service processes by hand. Use `stop`, `restart`, and `prune`.

### Stop what you started

Stop servers you started for a task when the task is done. Leave services you did not start alone.

### Names are scoped to a directory

The same name in two directories is two services. Run from the service's directory, or use the session id from `ser status`, when a name is ambiguous.

## Reading state

`ser status --json` lists services, their URLs, directories, and branches, plus ports that are listening but not managed by `ser`. Use it to find who owns a port before choosing another.

`ser port <name>` prints only the port of a running service. `ser run -d` with the same name in the same directory returns the existing session instead of starting a duplicate, so it is safe to call before every use.

`ser status -q` exits 0 only when something is running and all of it is up. `ser heal` restarts services that died.
