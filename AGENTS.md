# Service-tool

`ser` keeps track of served processes and lets you run them headless. API, web apps - anything that you are serving up on localhost. It replaces a tmux pane per server.

The problem(s) it solves:
- tracking servers started in multiple different projects
- port conflicts
- multiple worktrees with different services coming from them

## CMD interface
- `ser` (prefix for the tool)
    - running `ser` is the same as running `ser --help`
- `--help, -h, help` displays commands
    - there is a `-h` and `--help` option at every level
- `update` rebuilds and reinstalls `ser` from the source checkout it was built from
- `run` when running a service
    - `-c, --command` or first positional argument : command to start the server
        - needs a string replacment field for the port in the command instead of the port
    - `-p, --port` or second positional port to start the server on
    - Optional
    - `-d, --detatched` runs the server on another process
    - `-t` tag
    - `-n` name
    - `-f` force. Forces the use of the specifed port. If there is another service on that port, it is restarted on the next free port
    - `--health` curl command for checking health (i.e. a healthz endpoint)
- `stop`
    - `-a, --all` stops all services
    - can target by name, tag, id, or process id
- `status` displays all current services
- `heal` attempts to restart any dead service
- `restart` stops and restarts a given service, perfering it's port
- `log` prints the newest session's log
    - `n N` last N lines, default 100, `-f` follows
- `check <name, id>` reports the health check

## database schema
- table for sessions
    - a session is an instance of a service
    - id (uuid7), name, port, process, tags, created_at, command, log_path, stopped_at
- migrations are named `<YYMMDDHH>_<title>` and tracked in `migrations`

## technical layout
- CLI commands read and write SQLite directly and spawn deteacted processes
- `process` : spawns, signals, and checks liveness of a server process
- `db` : SQLlite state (sessions)
- `ports` : free port selection
    - when attempting to start a service, service-tool tries for the port the service requested. it checks if that port is free, and if it isn't incriments up until it finds a free port, then starts the service on that free port
    - it is never a guaruntee that the port used will be the port requested
- configs and logs go in
    - $SERVICE_TOOL_ROOT, by default ~/.config/ser/
    - config.toml
    - logs/
    - ser.db

## tech stack
- Rust for all CLI and interaction code
- local SQLite database for session state

## agent use
- stdout is data, diagnostics to stderr, exit codes meaningful
- `--json` formatting option
    
