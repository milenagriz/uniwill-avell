---
status: accepted
---

# The daemon computes fan speed, guarded by a kernel watchdog

Fan speed outside Max fan is computed by our own daemon (`avelld`) from CPU and GPU temperature and written to the fans directly, instead of loading a curve into the firmware's fan table and letting the firmware run it. We chose this because the project optimises for gaming performance and wants curves the firmware table cannot express, alongside profile coupling and GPU-activity rules that already need a daemon. Because a crashed or hung daemon would leave the fans at a stale speed, the kernel driver hands the fans back to the firmware's automatic curve when the daemon stops updating, and forces Max fan above a hard temperature.

## Considered options

- **Daemon picks a curve, firmware runs it.** Nothing to crash, but limited to the firmware table's shape. Rejected as too restrictive.
- **Direct writes with no watchdog.** Simplest, with the CPU's own thermal throttle as the only protection. Rejected as unsafe.

## Consequences

- The direct fan writes and the watchdog are things the upstream maintainer calls unstable, so they live in a local-only kernel patch on top of the upstreamable ones and must be rebased by us on every kernel update.
