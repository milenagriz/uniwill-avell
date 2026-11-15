# uniwill-avell

A control stack for the **Avell A52 MOB** (Tongfang/Uniwill GM5TGFX board, i5-11400H, RTX 3050 Laptop) under Linux.
Avell has never shipped a Linux driver, firmware update or Linux software for it. See the spec, issue #1.

What works today: a patched `uniwill-laptop` driver that binds to this laptop, shows both fans and both
temperatures, and adds **Max fan**, so the [Omarchy cooling plugin](https://github.com/ismoil77/omarchy-cooling)
gets its **Max** button. It also sets the **TGP offset** (0 to 20 W, `/sys/bus/platform/devices/INOU0000:00/ctgp_offset`),
and the install script turns on **Dynamic Boost** (`nvidia-powerd`). The **Power profile** (quiet, balanced, performance)
appears through the kernel's platform-profile interface, so the Omarchy power menu drives the firmware, and the button
next to the power key cycles it. Planned: daemon-driven fan curves, battery care, the keyboard backlight and BIOS tools.

## Layout

| Path | What |
|---|---|
| `driver/` | The kernel driver, packaged with DKMS |
| `driver/src/` | The driver sources DKMS builds (linux v7.2.15 plus `driver/patches/`) |
| `driver/patches/0001`, `0002` | The `uniwill` parts of Omarchy kernel patches 0800 and 0803, so the module matches the running `linux-omarchy` kernel |
| `driver/patches/0003` | Upstreamable: the Avell DMI entry and Max fan |
| `driver/patches/0004` | Upstreamable: TGP offset for this board (clamp to 20 W, forwarded to the graphics driver through the firmware's `T1WR`) |
| `driver/patches/0005` | Upstreamable: Power profile through platform_profile, cycled by the performance button |
| `driver/patches/0006` | **Local-only:** direct fan speed through the universal fan tables, with the kernel watchdog (ADR-0001, ADR-0003) |
| `driver/dkms.conf` | DKMS package `uniwill-avell/7.2.15.1` |
| `driver/checks/` | On-device check scripts. Each one writes to the EC and ends by restoring automatic behaviour |
| `daemon/` | `avelld` and `avellctl` (Rust) |
| `bar-plugin/` | The Omarchy bar widget |
| `app/` | The windowed control centre |
| `bios/` | BIOS backup and analysis tools, and the written procedures for BIOS changes |
| `evidence/` | ACPI tables, EC dumps and the scripts that took them |
| `scripts/` | `install.sh` and `uninstall.sh` for the whole stack |
| `docs/INVESTIGATION.md` | What was found on this machine and how it was tested |
| `docs/IMPROVEMENTS.md` | Known limits and ideas outside the spec |
| `docs/adr/` | Architecture decisions |
| `GLOSSARY.md` | The terms used everywhere: Max fan, Power profile, TGP offset, Dynamic Boost |

## The fan interface

The driver adds a standard hwmon `pwmN_enable` file (shared by both fans):

| `pwmN_enable` | Meaning |
|---|---|
| `0` | Max fan |
| `1` | Manual. Starts at Max fan; then each fan takes any `pwmN` from 0 to 255. Duties too low to spin a fan are rounded. If nothing writes a duty for `fan_watchdog_sec` (default 5 s), the driver returns to `2`. Above `fan_hard_temp_cpu`/`fan_hard_temp_gpu` it forces Max fan |
| `2` | Automatic (the firmware's own fan curve). This is the default after the driver loads |

The EC moves the fans towards a manual duty slowly (about 1% per second); full speed always goes through Max fan, so it is instant. The mode read always follows
the EC: if the firmware or a hotkey turns Max fan off, the mode reads `2`, never `1`.

To get clangd working in an editor, generate `driver/src/compile_commands.json` with
`make -C /usr/lib/modules/$(uname -r)/build M=$PWD modules compile_commands.json` in `driver/src` (`driver/src/.clangd` strips the GCC-only flags).

## Install

```bash
scripts/install.sh          # copies driver/ to /usr/src, installs with DKMS, loads it at boot, swaps the running module
driver/checks/fans.sh       # the fan mode contract and the Cooling widget's Max button, ends on automatic
driver/checks/direct-fan.sh # manual duties, Max fan override and the watchdog (about 1.5 min)
```

Requires `dkms` and the headers for the running kernel (`linux-omarchy-headers`).

## Remove

```bash
scripts/uninstall.sh
```

## After a kernel update

`driver/src/` is fixed at v7.2.15. DKMS rebuilds it automatically. If the build fails on a newer kernel,
the fans just disappear again; the kernel's own driver does not bind to this board, so nothing else breaks.
To fix it, rebase `driver/patches/0003` onto the new kernel's `drivers/platform/x86/uniwill/uniwill-acpi.c`
(see `docs/IMPROVEMENTS.md`). Check with `dkms status`.

## License

GPL-2.0-or-later, the same as the upstream driver (`uniwill-acpi.c` by Armin Wolf).
