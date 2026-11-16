## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues for milenagriz5-dotcom/uniwill-avell, via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Uses the default five triage labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.

## Working rules

### Hardware safety

- Before the first write to an EC register class not yet written on this board, ask the owner. Name the registers and the rollback (write the original value back; a full power-off resets the EC). Each register's write status is recorded in `docs/INVESTIGATION.md`.
- On-device check scripts (`driver/checks/`) restore the original state in an `EXIT` trap and end with the fans on automatic.
- Enable a feature in `avell_a52_mob_descriptor` only after it is verified on the device.
- BIOS tools only read: every flash write is a human-only procedure (ADR-0002).

### Driver patch series

- `driver/src` is the v7.2.15 driver with `driver/patches/` applied in order: upstreamable patches first, `*-local-*` patches last.
- After editing `driver/src`, regenerate the patch you changed (`diff -u`, `a/drivers/platform/x86/uniwill/uniwill-acpi.c` headers, trailing newline) and prove the series: `patch -p5` over the base reproduces `driver/src` byte for byte, and every step builds with `W=1` and no warnings. Get the base by reverse-applying the patches, newest first.
- Bump `MODULE_VERSION` on every driver change, and inspect the built module with REA `inspect_kernel_module` (decompress the DKMS `.ko.zst` first).
- Remove build artifacts by hand: `make clean` also deletes `compile_commands.json`, which clangd needs.
- Gotchas: a read of `/sys/kernel/debug/regmap/INOU0000:00/registers` takes about a second (every register goes through ACPI), so keep it out of timed loops. Reloading the driver turns the keyboard backlight off.

### Daemon

- Tests drive the daemon only through the control API (`daemon/tests/common`: peer-to-peer D-Bus, fake system port), one red → green slice at a time.
- `scripts/install.sh` installs the driver, the daemon, its service and the D-Bus policy together, and restarts `avelld`.
