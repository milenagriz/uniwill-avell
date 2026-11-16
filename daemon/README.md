# daemon

`avelld` (root system service) and `avellctl` (CLI), in Rust. One crate: `src/lib.rs` plus two binaries.

| Module | What |
|---|---|
| `port` | The system port: kernel control files under a root, the power-profile service, the GPU probe, the clock, desktop sessions, the service manager, the configuration file |
| `real` | The real port: `/`, power-profiles-daemon, `nvidia-smi` (only when the card is already awake), logind, systemd, `/etc/avelld/config.toml` |
| `config` | The configuration: one bundle per Power profile, and its validation |
| `engine` | One task that owns the state: every second it reads the active Power profile and applies its bundle when the profile changed or the machine resumed |
| `status` | The machine state, read through the port |
| `api` | The control API, `org.avell.Control1` at `/org/avell/Control1` on the system bus |
| `client` | The one client of the API, used by `avellctl` and by the tests |

## Control API

| Method | Who | What |
|---|---|---|
| `GetStatus() → s` | anyone | The whole state as JSON (see `avellctl --json status`) |
| `SetProfile(s)` | the active local desktop user | power-saver, balanced or performance, through power-profiles-daemon; its bundle is applied at once |
| `GetConfig() → s` | anyone | The configuration in force, as TOML |
| `SetConfig(s)` | the active local desktop user | Replaces the configuration (TOML). Invalid: refused with the reason, the previous one stays. Valid: saved and applied at once |

Callers that are not the user of an active, local, seat-attached logind session get `org.freedesktop.DBus.Error.AccessDenied`.
No password prompt. `dist/org.avell.Control1.conf` lets only root own the name and anyone call it.

## CLI

```
avellctl [--json] status | profile | profile set <name> | config | config set <key> <value>
```

Every command accepts `--json`. Errors come out as `{"error": "..."}` with exit code 1. When the daemon is not running,
the CLI says so.

## Profile bundles

Each Power profile applies one bundle. power-profiles-daemon stays the single source of truth for the active profile
(power menu, Fn key and `avellctl profile set` all go through it); the daemon follows it within a second.

`/etc/avelld/config.toml` (the built-in values apply when the file is missing; edit it with `avellctl config set`, or by
hand and restart `avelld`):

```toml
[profiles.power-saver]
tgp_offset_w = 0        # TGP offset, 0 to 20 W (the firmware table's maximum)
dynamic_boost = false   # Dynamic Boost: nvidia-powerd.service running or not
cpu_long_term_w = 20    # CPU power limits through the kernel's power capping (intel-rapl), 5 to 120 W
cpu_short_term_w = 30   # not below the long-term limit

[profiles.balanced]
tgp_offset_w = 10
dynamic_boost = true
cpu_long_term_w = 45
cpu_short_term_w = 65

[profiles.performance]
tgp_offset_w = 20
dynamic_boost = true
cpu_long_term_w = 80
cpu_short_term_w = 80
```

The bundle is applied again after resume. On a clean stop the daemon sets the TGP offset to 0 and the CPU limits back to
what they were when it started.

## Tests

```bash
cargo test
```

They drive the daemon through the control API over a private peer-to-peer D-Bus connection, against a fake system port:
a temporary directory laid out like this laptop's `/sys`, plus fake power-profile service, GPU probe, clock, sessions and service manager
(`tests/common/mod.rs`). No real hardware is touched.

Installed by `scripts/install.sh` (builds release, installs `/usr/local/bin/avelld`, `/usr/local/bin/avellctl`, the service and the D-Bus policy).
