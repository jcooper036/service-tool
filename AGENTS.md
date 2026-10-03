# Service-tool

`ser` keeps track of served processes. API, web apps, anything you are serving on localhost. It replaces a tmux pane per server.

The problems it solves:
- tracking servers started in multiple different projects
- port conflicts
- multiple worktrees with different services coming from them

## Scope

`ser` is not a supervisor, a container manager, or a connector to remote services. It does not guarantee that a service's environment is the same on a restart. If the environment changed and the service fails to start, that is an expected failure.

## Concepts

### Services are ephemeral

Nothing is registered. A service exists because someone ran it, and `ser` records what happened.

### A session is one run of a service

A session records the command template, the assigned and requested ports, the directory it ran from, the git branch and worktree at that moment, the process id and start time, the health URL, tags, and the log path. Sessions stay in the database after they stop, so the log of a service that died is still readable.

### Ports are assigned, not fixed

`ser` tries the requested port first, 3000 when none is given. If it is taken, it increments until it finds a free one. The port used is never guaranteed to be the port requested.

### Commands cannot hardcode the port

A service must take its port either from a `{port}` placeholder in the command or from the `PORT` environment variable, which `ser` always sets. A command with a hardcoded port is not supported.

### Liveness has four states

`up`: the process is alive and the port accepts connections. `unhealthy`: up, but the health URL fails. `unreachable`: alive, port closed. `dead`: the process is gone.

## CMD interface
- `ser` (prefix for the tool)
    - running `ser` is the same as running `ser --help`
- `--help, -h, help` displays commands
    - there is a `-h` and `--help` option at every level
- `update` rebuilds and reinstalls `ser` from the source checkout it was built from
- `run` starts a service in the foreground. Ctrl-C stops it, and it is tracked the whole time
    - `-c, --command` or first positional argument: command to start the server
    - `-p, --port` or second positional argument: port to prefer
    - `-d, --detached` runs the server on another process
    - `-t` tag, repeatable or comma separated
    - `-n` name, defaulting to the directory name
    - `--health` http(s) URL containing `{port}`. A 2xx response is healthy
    - `--wait` seconds to wait for the port to open
    - `--json` prints the session instead of the port
    - foreground streams the service's output and logs it. Detached prints only the port to stdout
    - running the same name from the same directory while it is up returns the existing session instead of starting a duplicate
- `stop`
    - `-a, --all` stops all services
    - targets by name, tag, id, or process id
- `status` displays all current services with a clickable URL, plus ports that are listening but not managed by `ser`
    - `--json`, `--all` (include stopped sessions), `-q, --quiet` (exit 0 only if at least one service is running and all are up)
- `heal` restarts dead services
- `restart` stops and restarts a given service, preferring its port
- `log` prints the newest session's log
    - `-n N` last N lines, default 100, `-f` follows
- `check <name, id>` reports the health check. Exit 0 healthy, 1 not
- `show <name, id>` one-call diagnosis: session, directory, branch, live health check, log tail. `--json`
- `port <name>` prints only the running port. Non-zero exit if not running
- `set <name, id>` changes a running service
    - `--health` sets the health URL, replacing any existing one
    - `-t` adds tags
    - both carry through `restart` and `heal`
- `prune` deletes stopped sessions and their logs

### Name scope

A name is unique among active sessions in one directory. The same name in two directories is two services.

### Targets resolve to one session

A target is tried as a session id, a name, a process id, a short id (the last 8 characters shown by `status`), and for `stop` a tag. Only a tag can match several sessions. A name that matches services in several directories is resolved by the current directory, and is an error when that does not pick exactly one.

### Heal only revives what died

`heal` restarts sessions that have no `stopped_at` and whose process is gone. A session ended by `stop` is never revived. A service healed 3 times in a row is given up on until `restart` resets it, and a session whose directory no longer exists is skipped and reported. Healed services run detached.

### Stop checks the start time

Before signalling, `ser` compares the process start time to the one recorded in the session. A mismatch means the pid was reused, and `ser` does not signal it.

## database schema
- table for sessions
    - a session is an instance of a service
    - active sessions are rows with `stopped_at` null
    - id is a uuid7
    - the command is stored as the template, with the placeholder, so a restart can pick a new port
- migrations are named `<YYMMDDHH>_<title>` and tracked in `migrations`

## technical layout
- CLI commands read and write SQLite directly and spawn detached processes
- `process`: spawns, signals, and checks liveness of a server process (own process group)
- `db`: SQLite state (sessions)
- `ports`: free port selection and listener discovery
- port selection and the session insert happen in one `BEGIN IMMEDIATE` transaction, so two concurrent `ser run` calls cannot claim the same port. Migrations take the same lock
- a port counts as free only if it can be bound on `127.0.0.1`, `0.0.0.0`, and `::1`
- state lives in `$SER_HOME`, by default `~/.ser/`
    - logs/
    - ser.db

## tech stack
- Rust for all CLI and interaction code
- local SQLite database for session state

## agent use
- stdout is data, diagnostics to stderr, exit codes meaningful
- `--json` formatting option
- `.agents/skills/service-tool/` holds the skill that tells agents to use `ser` instead of backgrounding servers with `&`
- when something is wrong: `ser show <name>`, then `ser log <name>`

Version: 1.2
Date: 2026-10-02
END OF DOCUMENT
