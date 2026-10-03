# Service Tool

`ser` keeps track of the servers you run on localhost. APIs, web apps, dev servers, notebooks: anything you serve on a local port.

It replaces a tmux pane per server. Run a service through `ser` and it gets a free port, a log, and a record of where and from which branch it was started. `ser status` is the lookup for what is running, where, and at which link.

A companion tool, [tunnel-tool](https://github.com/jcooper036/tunnel-tool), does the same job for connections to services that are not on your machine, such as Cloud SQL proxies, `kubectl port-forward`, and ssh tunnels. `ser` is for what you host, and `tun` is for what you connect to.

## Install

Clone the repository, then run `cargo install --path .` from its root. This builds the binary and puts `ser` in `~/.cargo/bin`.

If you have not used cargo before, add `~/.cargo/bin` to your `PATH` in your shell profile.

Run `ser --help` for the commands. Every subcommand has its own `--help`.

### Installing for agents

Part of the usefulness of `ser` is that agents can use it instead of starting servers in the background and forgetting them. The agent skill lives in `.agents/skills/service-tool`. Use whatever skill manager you like. A symlink from your global agent skills folder to that directory works, and it picks up skill changes whenever you pull.

### Updating

`ser update` rebuilds and reinstalls `ser` from the checkout it was built from. Pull in that checkout first to get new changes.

## Security

`ser` runs the shell commands you give it. Anything that can call `ser run` can run any command as you.

- Services keep running after the terminal or agent session that started them ends. Check `ser status`
- `ser` does not authenticate local connections. Any local process can reach a service port
- Logs in `~/.ser/logs` contain whatever the service prints, which can include credentials
- `ser status` lists every listening port on the machine, with the process name that owns it
- Make services listen on `127.0.0.1` unless you mean to expose them

Provided as is, without warranty. See LICENSE.

## Concepts

### Services are ephemeral

Nothing is registered ahead of time. A service exists because someone ran it, and `ser` records what happened. There is no spec file to write and no registry to maintain.

### A session is one run of a service

Each run is a session in `~/.ser/ser.db`. It records the command, the ports, the directory, the git branch and whether it is a linked worktree, the process, the health URL, tags, and the log path. The output of each session goes to `~/.ser/logs`. Stopped sessions stay in the database, so the log of a service that died is still readable. `ser prune` clears them.

### Ports are assigned, not fixed

`ser` tries the port you ask for, or 3000 if you give none. If it is taken, it moves up until it finds a free one. A port counts as free only if nothing is listening on it and it can be bound on every loopback and wildcard address. Starting services at the same time never hands out the same port twice.

Anything that needs a service's port asks `ser`. Do not remember a port from an earlier run.

### Commands cannot hardcode the port

A service has to take its port from a `{port}` placeholder in the command or from the `PORT` environment variable, which `ser` always sets. A server with a hardcoded port cannot be moved, so `ser` does not support it. If a service never opens its assigned port, `ser` says so and points at its log.

### A service is a name in a directory

The name defaults to the directory name. Running the same name from the same directory while it is up returns the existing session instead of starting a duplicate. To start another instance, give it another name. The same name in two directories is two services, which is how the same app can run from two worktrees at once.

A name that matches services in several directories is resolved by the directory you are in. If that does not pick exactly one, `ser` lists the matches and you use the session id from `ser status`.

### Foreground by default, detached on request

`ser run` stays in the foreground, streams the service's output, and stops it on Ctrl-C. The service is tracked the whole time. With `-d` it runs on another process, prints only its port, and keeps going after you leave.

### Liveness has four states

`up`: the process is alive, the port accepts connections, and the health check, if any, passes. `unhealthy`: up, but the health check fails. `unreachable`: the process is alive and the port is closed, which is also what a service looks like while it is still starting. `dead`: the process is gone.

### Health checks are URLs

A health check is an http or https URL containing `{port}`. A 2xx response is healthy. Set it with `ser run --health`, or add it to a service that is already running with `ser set`, which also adds tags.

### Heal only revives what died

`ser heal` restarts services that were running and whose process is gone. A service you stopped is never revived. A service that has been healed three times in a row is given up on until you restart it by hand, and a service whose directory no longer exists is skipped and reported.

### Stopping is safe against reused process ids

`ser` records when each process started and checks it before signalling. If a process id has been reused by something else, `ser` leaves it alone.

### Unmanaged ports are visible

`ser status` also lists ports that are listening but were not started by `ser`. When a port is taken, that is where you find out who has it.

## Scope

`ser` is not a supervisor, a container manager, or a connector to remote services. It does not guarantee that a service's environment is the same on a restart. If the environment changed and the service fails to start, that is an expected failure.

## Output for scripts and agents

Stdout is data, diagnostics go to stderr, and exit codes are meaningful. A detached `ser run` prints only the port. `ser port <name>` prints only the port of a running service. `ser status -q` prints nothing and exits 0 only when something is running and all of it is up. Commands that report state take `--json`.

When something is wrong, run `ser show <name>` for a one-call diagnosis, then `ser log <name>`.

## State location

`~/.ser` by default. Override it with `--home` or `SER_HOME`. Pointing `SER_HOME` at a fresh directory gives you an empty, isolated set of services, which is useful for trying things out.

## Development

`cargo test` runs the suite. The tests start real servers with `python3`, read git state with `git`, and list listeners with `lsof`, so those need to be installed. Design notes are in `AGENTS.md`.

Version: 1.0
Date: 2026-10-02
END OF DOCUMENT
