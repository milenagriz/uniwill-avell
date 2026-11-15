# daemon

`avelld` (root system service) and `avellctl` (CLI), in Rust. One crate: `src/lib.rs` plus two binaries.

| Module | What |
|---|---|
| `port` | The system port: kernel control files under a root, the power-profile service, the GPU probe, the clock, desktop sessions |
| `real` | The real port: `/`, power-profiles-daemon, `nvidia-smi` (only when the card is already awake), logind |
| `status` | The machine state, read through the port |
| `api` | The control API, `org.avell.Control1` at `/org/avell/Control1` on the system bus |
| `client` | The one client of the API, used by `avellctl` and by the tests |

## Control API

| Method | Who | What |
|---|---|---|
| `GetStatus() → s` | anyone | The whole state as JSON (see `avellctl --json status`) |
| `SetProfile(s)` | the active local desktop user | power-saver, balanced or performance, through power-profiles-daemon |

Callers that are not the user of an active, local, seat-attached logind session get `org.freedesktop.DBus.Error.AccessDenied`.
No password prompt. `dist/org.avell.Control1.conf` lets only root own the name and anyone call it.

## CLI

```
avellctl [--json] status | profile | profile set <name>
```

Every command accepts `--json`. Errors come out as `{"error": "..."}` with exit code 1. When the daemon is not running,
the CLI says so.

## Tests

```bash
cargo test
```

They drive the daemon through the control API over a private peer-to-peer D-Bus connection, against a fake system port:
a temporary directory laid out like this laptop's `/sys`, plus fake power-profile service, GPU probe, clock and sessions
(`tests/common/mod.rs`). No real hardware is touched.

Installed by `scripts/install.sh` (builds release, installs `/usr/local/bin/avelld`, `/usr/local/bin/avellctl`, the service and the D-Bus policy).
