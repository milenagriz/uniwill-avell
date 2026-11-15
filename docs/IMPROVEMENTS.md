# Improvements and known limits

## 1. Go to full speed as soon as manual mode is entered (recommended next change)

**Now:** writing `pwmN_enable = 1` only records "manual". The fans stay in whatever state they were in,
usually automatic, until `pwmN = 255` is written. So during the gap between the two writes,
`pwm1_enable` reads `1` while the EC is still running its own curve.

**Proposal:** do what `gpd-fan` does. Its kernel documentation says: *"In consideration of the safety
of the device, when setting to manual mode, the pwm speed will be set to the maximum value (255) by
default."* Entering manual mode would turn boost on straight away:

```c
case UNIWILL_PWM_MODE_MANUAL:
	data->pwm_mode = val;
	return uniwill_set_fan_boost(data, true);	/* manual starts at 255 */
```

and in `hwmon_pwm_input`, the only accepted value in manual mode stays `255`, which is now a no-op.

**Why:** the state you read back is always the real state (`1` always means full speed). Any tool that sets
manual mode then crashes before writing `pwm` leaves the fans *louder*, never *hotter*.
The cooling plugin is unaffected: it writes `1` then `255`, and the result is the same.

## 2. Rebase when the kernel moves past 7.2.15

`src/` is frozen. On a kernel update that changes this driver, either:
- Take the new kernel's `uniwill-acpi.c` (from the stable tree plus the matching Omarchy `08xx` patches),
  apply `patches/0003`, fix any rejected hunks, bump `PACKAGE_VERSION` and `MODULE_VERSION`, then
  run `scripts/install.sh`; or
- Turn this into a small rebase script that downloads the right sources for `uname -r` automatically.

## 3. Submit upstream

The two pieces are independent and could go to platform-driver-x86 (maintainer: Armin Wolf):
- The Avell A52 MOB DMI entry (small, easy to accept).
- `pwmN_enable` / fan boost through `0x0751` bit 6. Upstream may want it limited to devices where it is
  tested, so it would need reports from other Uniwill models. Once it is upstream, this DKMS package is no longer needed.

## 4. Features left off (need testing first)

`force=1` exposed these, but they were not verified on this laptop:

| Feature | How to test safely |
|---|---|
| Keyboard backlight (`uniwill:multicolor:kbd_backlight`) | check whether the keyboard is RGB or single colour, then try brightness levels |
| NVIDIA cTGP / Dynamic Boost (`ctgp_offset`, EC `0x0743 = 0x07`) | compare GPU power limits in `nvidia-smi -q -d POWER` before and after |
| Battery charge limit / modes | check `0x07B9` behaviour; wrong values could stop charging. Do this last |
| USB-C power priority, AC auto boot, USB power share | read-only check first, then toggle and observe |

Add each one to `avell_a52_mob_descriptor` only after it is confirmed.

## 5. Fan boost hotkey

Uniwill firmware usually has a fan boost hotkey and reports `FAN_BOOST_STATUS` (`0x0768` bit 2). Reading
`pwm_enable` already follows the EC bit, but an input or uevent notification would let the plugin update
without polling.

## 6. Intermediate speeds

`0x1804/0x1809` accept direct duty writes (TUXEDO uses them, with a ramp-up workaround and a
50-minute restart timer). Upstream calls them unstable. They could add a real manual curve
(`pwm` from 0 to 255), but only behind a module parameter and after long testing here.
