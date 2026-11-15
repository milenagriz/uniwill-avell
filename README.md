# uniwill-avell

A control stack for the **Avell A52 MOB** (Tongfang/Uniwill GM5TGFX board, i5-11400H, RTX 3050 Laptop) under Linux.
Avell has never shipped a Linux driver, firmware update or Linux software for it. See the spec, issue #1.

What works today: a patched `uniwill-laptop` driver that binds to this laptop, shows both fans and both
temperatures, and adds **Max fan**, so the [Omarchy cooling plugin](https://github.com/ismoil77/omarchy-cooling)
gets its **Max** button. Planned: **Power profile** switching, a **TGP offset** raise with **Dynamic Boost**,
daemon-driven fan curves, battery care, the keyboard backlight and BIOS tools.

## Layout

| Path | What |
|---|---|
| `driver/` | The kernel driver, packaged with DKMS |
| `driver/src/` | The driver sources DKMS builds (linux v7.2.15 plus `driver/patches/`) |
| `driver/patches/0001`, `0002` | The `uniwill` parts of Omarchy kernel patches 0800 and 0803, so the module matches the running `linux-omarchy` kernel |
| `driver/patches/0003` | **Our change:** the Avell DMI entry and Max fan |
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

The driver adds a standard hwmon `pwmN_enable` file, backed by the EC's own Max fan bit:

| `pwmN_enable` | Meaning |
|---|---|
| `0` | Max fan |
| `1` | Manual. Starts at full speed straight away; only `pwmN = 255` is accepted for now |
| `2` | Automatic (the firmware's own fan curve). This is the default after the driver loads |

Both fans share one EC bit, so changing `pwm1_enable` also changes `pwm2_enable`. The mode read always follows
the EC: if the firmware or a hotkey turns Max fan off, the mode reads `2`, never `1`.

To get clangd working in an editor, generate `driver/src/compile_commands.json` with
`make -C /usr/lib/modules/$(uname -r)/build M=$PWD modules compile_commands.json` in `driver/src` (`driver/src/.clangd` strips the GCC-only flags).

## Install

```bash
scripts/install.sh          # copies driver/ to /usr/src, installs with DKMS, loads it at boot, swaps the running module
driver/checks/fans.sh       # about 10 s of Max fan, then back to automatic
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
