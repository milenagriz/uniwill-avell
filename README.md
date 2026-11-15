# uniwill-avell

A patched Linux `uniwill-laptop` driver for the **Avell A52 MOB** (i5-11400H, RTX 3050 Mobile), which is built on a Uniwill chassis.

With the kernel's own driver, Linux sees **no fans** on this laptop, so fan tools and the
[Omarchy cooling plugin](https://github.com/ismoil77/omarchy-cooling) have nothing to show or control.
This driver adds:

- **A match for this laptop's hardware ID (DMI).** It loads automatically, so `force=1` and the kernel taint it causes are no longer needed.
- **Only the features verified on this machine:** Fn lock, Super key toggle, touchpad toggle,
  CPU/GPU temperature, and both fans (RPM and duty).
- **Max fan mode.** A standard hwmon `pwmN_enable` file, backed by the EC's own "full fan mode" bit.
  This is what makes the cooling plugin show its **Max** button.

| `pwmN_enable` | Meaning |
|---|---|
| `0` | Full speed (EC fan boost on) |
| `1` | Manual. Only `pwmN = 255` (full speed) is accepted; the EC offers no other manual speed here |
| `2` | Automatic (the EC's own fan curve). This is the default after the driver loads |

Both fans share one EC bit, so changing `pwm1_enable` also changes `pwm2_enable`.

## Install

```bash
scripts/install.sh      # copies the sources to /usr/src, installs with DKMS, loads it at boot, swaps the running module
scripts/test-fans.sh    # about 10 s at full speed, then back to automatic
```

Requires `dkms` and the headers for the running kernel (`linux-omarchy-headers`).

## Remove

```bash
scripts/uninstall.sh
```

## Layout

| Path | What |
|---|---|
| `src/` | The driver sources DKMS builds (the result of applying `patches/` on top of linux v7.2.15) |
| `patches/0001`, `0002` | The `uniwill` parts of Omarchy kernel patches 0800 and 0803, so the module matches the running `linux-omarchy` kernel |
| `patches/0003` | **Our change:** the Avell DMI entry and fan boost |
| `dkms.conf` | DKMS package `uniwill-avell/7.2.15.1` |
| `docs/INVESTIGATION.md` | How the fan control was found and tested |
| `docs/IMPROVEMENTS.md` | Next steps and known limits |

## After a kernel update

`src/` is fixed at v7.2.15. DKMS rebuilds it automatically. If the build fails on a newer kernel,
the fans just disappear again; the kernel's own driver does not bind to this board, so nothing else breaks.
To fix it, rebase `patches/0003` onto the new kernel's `drivers/platform/x86/uniwill/uniwill-acpi.c`
(see `docs/IMPROVEMENTS.md`). Check with `dkms status`.

## License

GPL-2.0-or-later, the same as the upstream driver (`uniwill-acpi.c` by Armin Wolf).
