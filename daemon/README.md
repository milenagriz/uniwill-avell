# daemon

`avelld` (root system service) and `avellctl` (CLI), in Rust. One crate: `src/lib.rs` plus two binaries.

| Module | What |
|---|---|
| `port` | The system port: kernel control files under a root, the power-profile service, the GPU probe, the clock, desktop sessions, the service manager, the configuration file |
| `real` | The real port: `/`, power-profiles-daemon, `nvidia-smi` (only when the card is already awake), logind, systemd, `/etc/avelld/config.toml` |
| `config` | The configuration: one bundle per Power profile, and its validation |
| `policy` | Pure decisions, no I/O and no clock: the fan speed for a temperature along a curve with hysteresis (the CPU temperature averaged over its last 3 readings), the TGP offset for the power source, and when game detection holds and releases |
| `engine` | One task that owns the state: every second it reads the active Power profile and applies its bundle when the profile changed or the machine resumed; every 2 s it writes the fan duty |
| `status` | The machine state, read through the port |
| `api` | The control API, `org.avell.Control1` at `/org/avell/Control1` on the system bus |
| `client` | The one client of the API, used by `avellctl` and by the tests |

## Control API

| Method | Who | What |
|---|---|---|
| `GetStatus() → s` | anyone | The whole state as JSON (see `avellctl --json status`) |
| `SetProfile(s)` | the active local desktop user | power-saver, balanced or performance, through power-profiles-daemon; its bundle is applied at once |
| `GetConfigJson() → s`, `SetConfigJson(s)` | as GetConfig / SetConfig | The same configuration as JSON, for the interfaces |
| `SetFan(s, u)` | the active local desktop user | Who runs the fans: `curve`, `fixed` (with a percentage), `max` or `automatic`. Lasts until the next call or a restart |
| `SetKeyboardBrightness(u)` | the active local desktop user | The keyboard backlight's level, 0 (off) to the keyboard's maximum (4 here) |
| `SetKeyboardColor(y, y, y)` | the active local desktop user | Red, green, blue, each 0-255, scaled to what the keyboard takes (0-50 here) |
| `SetChargingProfile(s)` | the active local desktop user | The firmware's charging profile: `high-capacity`, `balanced` or `stationary` (the battery's `charge_types`: Standard, Long_Life, Trickle) |
| `SetChargeCap(u)` | the active local desktop user | Stop charging at this level, 20-100 percent (the battery's `charge_control_end_threshold`) |
| `GetConfig() → s` | anyone | The configuration in force, as TOML |
| `SetConfig(s)` | the active local desktop user | Replaces the configuration (TOML). Invalid: refused with the reason, the previous one stays. Valid: saved and applied at once |

Callers that are not the user of an active, local, seat-attached logind session get `org.freedesktop.DBus.Error.AccessDenied`.
No password prompt. `dist/org.avell.Control1.conf` lets only root own the name and anyone call it.

## CLI

```
avellctl [--json] status | profile | profile set <name> | fan | fan set <curve|automatic|max|percent>
         | keyboard | keyboard brightness <level> | keyboard color <#rrggbb>
         | battery | battery profile <name> | battery cap <percent>
         | config | config set <key> <value>
```

Every command accepts `--json`. Errors come out as `{"error": "..."}` with exit code 1. When the daemon is not running,
the CLI says so.

## Profile bundles

Each Power profile applies one bundle. power-profiles-daemon stays the single source of truth for the active profile
(power menu, Fn key and `avellctl profile set` all go through it); the daemon follows it within a second.

`/etc/avelld/config.toml` (the built-in values apply when the file is missing; edit it with `avellctl config set`, or by
hand and restart `avelld`):

```toml
[charger]
enabled = true          # false: plugging and unplugging change nothing
on_ac = "performance"    # the profile set when the charger is plugged in
on_battery = "power-saver"   # and when it is unplugged

[game_detection]
enabled = true
load_percent = 60       # the graphics card counts as busy at this utilisation or above
hold_after_s = 15       # busy this long: hold the performance profile
release_after_s = 60    # below the threshold this long: release the hold

[fans]
mode = "curve"          # who runs the fans from start: "curve" (the daemon) or "automatic" (the firmware)
hysteresis_c = 4        # the fans slow down only once it is this much cooler than where they last sped up

[profiles.power-saver]
tgp_offset_w = 0        # TGP offset, 0 to 20 W (the firmware table's maximum)
dynamic_boost = false   # Dynamic Boost: nvidia-powerd.service running or not
cpu_long_term_w = 20    # CPU power limits through the kernel's power capping (intel-rapl), 5 to 120 W
cpu_short_term_w = 30   # not below the long-term limit
fan_curve = [[50, 20], [65, 35], [75, 50], [85, 75], [92, 100]]   # [°C, percent], see Fans

[profiles.balanced]
tgp_offset_w = 10
dynamic_boost = true
cpu_long_term_w = 45
cpu_short_term_w = 65
fan_curve = [[50, 30], [60, 40], [70, 60], [80, 80], [88, 100]]

[profiles.performance]
tgp_offset_w = 20
dynamic_boost = true
cpu_long_term_w = 80
cpu_short_term_w = 80
fan_curve = [[45, 35], [55, 50], [65, 65], [80, 85], [90, 100]]
```

A file may leave settings out; they keep the values above.

The bundle is applied again after resume. On a clean stop the daemon sets the TGP offset to 0 and the CPU limits back to
what they were when it started.

## Charger rules

When the power source changes (and at start), the daemon sets that source's profile through power-profiles-daemon. A manual
change of profile (power menu, Fn key, `avellctl profile set`) replaces `charger.on_ac` or `charger.on_battery`, whichever
source is in use, and is saved, so each source comes back with what was last chosen on it. On battery the TGP offset is 0
whatever the bundle says, with the rules on or off. Turn the rules off with `avellctl config set charger.enabled false`.

## Game detection

Every 2 s the daemon reads the graphics card's `runtime_status` from the kernel and asks the card for its load only when it
is already awake, so a sleeping card is never woken. Load at or above `load_percent` for `hold_after_s` makes the daemon ask
power-profiles-daemon to hold `performance` (`HoldProfile`, so the power menu shows the hold and its reason); once the load
has stayed below the threshold for `release_after_s` the hold is released and power-profiles-daemon returns to the owner's
own choice. A hold is not remembered as a manual choice. Choosing another profile during a hold cancels it, and no new hold
starts until that load has ended. Nothing is held on battery. `avellctl status` shows an active hold and why. Turn it off
with `avellctl config set game_detection.enabled false`.

## Fans

The daemon computes the fan speed (ADR-0001): the hotter of the CPU temperature, averaged over its last 3 readings (6 s,
because the CPU sensor jumps by 15-20 °C for a single reading), and the GPU temperature, along the active profile's
`fan_curve` (linear between points, flat before the first and after the last), with hysteresis. It writes the duty to both
fans every 2 s, changed or not, so the driver's 5 s watchdog returns the fans to the firmware if the daemon hangs; when the
daemon finds the fans on automatic (the watchdog fired, or the machine resumed) it takes them back. The EC moves the
fans towards a new duty at about 1% per second (ADR-0003), and the driver forces Max fan above its hard temperatures.

`avellctl fan set` hands the fans to `curve`, a fixed percentage, `max` (Max fan) or `automatic` (the firmware) until the
next call or a restart of the daemon; `fans.mode` in the configuration decides what a start begins with. Max fan and
automatic are set once and then left alone. Max fan turned on by someone else (the Fn key, `pwm1_enable = 0`) is left on
until it is released. A program that drives the fans through `pwm1_enable = 1` and `pwmN` itself is overwritten within
2 s under `curve` and `fixed`: use `automatic` for that. This includes the old Cooling widget's Max fan sequence; the bar
plugin's Max fan button and `avellctl fan set max` are the way to Max fan. A clean stop leaves the fans on automatic.

## Battery care

The charging profile and the charge cap go through the standard battery files. Until the driver exposes them for this board
(each is enabled only after it is verified on the device, see `docs/INVESTIGATION.md` section 14) the status lists them as
unavailable and setting them fails with a message saying so.

## Tests

```bash
cargo test
```

They drive the daemon through the control API over a private peer-to-peer D-Bus connection, against a fake system port:
a temporary directory laid out like this laptop's `/sys`, plus fake power-profile service, GPU probe, clock, sessions and service manager
(`tests/common/mod.rs`). No real hardware is touched.

Installed by `scripts/install.sh` (builds release, installs `/usr/local/bin/avelld`, `/usr/local/bin/avellctl`, the service and the D-Bus policy).
