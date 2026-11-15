# Investigation log (2026-11-15)

## 1. Starting point

The Omarchy cooling plugin (`ismoil.fans`) showed no **Max** button. Its README says Max appears only when
it finds a full-speed switch: a hwmon `pwmN_enable` file, ThinkPad `/proc/acpi/ibm/fan`, or MSI `cooler_boost`.

On this laptop **none of the 8 hwmon devices had a fan at all**: only the AC adapter, ACPI thermal zone,
battery, NVMe, the r8169 network card, coretemp, iwlwifi and UCSI.

## 2. Identifying the chassis

| Evidence | Value |
|---|---|
| DMI | sys_vendor `Avell High Performance`, product `A52 MOB`, board `Avell A52 MOB` |
| Firmware | the boot log loads the X.509 cert `UNIWILL Tech BIOS 2019 Root CA` |
| WMI GUIDs | `ABBC0F6D/6E/6F-8EA1-11D1-00A0-C90629100000`, the Uniwill management interface |
| `asus_wmi` | loaded but a false match (logs `ASUS Management GUID not found`) |

So this is a Uniwill laptop. Kernel 7.2 ships `uniwill-laptop`, but it only binds to 57 listed
DMI entries (TUXEDO, Schenker, Intel NUC laptops, Machenike, AiStone). Avell is not on the list.

## 3. Forced load: fans visible, no control

`modprobe uniwill-laptop force=1` added a `uniwill` hwmon device:
`fan1/fan2_input` (about 2000 RPM), `pwm1/pwm2` (76/255, about 30%), CPU/GPU temperature.

But:
- `pwm1/pwm2` were read-only (`0444`) and there was no `pwmN_enable`, so the plugin still showed no Max.
- `force=1` taints the kernel (`module_param_unsafe`) and turns on **every** feature except the
  charge limit: battery hooks, lightbar, RGB keyboard, cTGP. None of these are verified on this laptop.

## 4. Finding the full-speed switch

Sources read:
- linux v7.2.15 `drivers/platform/x86/uniwill/uniwill-acpi.c` plus Omarchy patches 0800/0803
- TUXEDO `tuxedo-drivers` (`uniwill_keyboard.h`, `tuxedo_io.c`), which controls fans on the same Uniwill ECs

How the driver talks to the EC: the ACPI methods `ECRR` (read) and `ECRW` (write) on device `INOU0000`,
wrapped in a regmap. Each register must be explicitly allowed as readable, writeable or volatile.

| EC register | Meaning | Source |
|---|---|---|
| `0x0741` bit 0 | `ENABLE_MANUAL_CTRL`. The driver sets it at probe | upstream |
| `0x0751` | `MANUAL_FAN_CTRL`: bits 0-2 level, 4 turbo, 5 high, **6 boost**, 7 user | upstream (defined, unused) |
| `0x075B/0x075C` | current PWM duty, 0..200 | upstream |
| `0x1804/0x1809` | writable PWM. Upstream calls them "unstable on some models" | upstream, TUXEDO |

TUXEDO's `set_full_fan_mode()` sets or clears **`0x0751` bit 6 (`0x40`)**. That is the EC's own
"full fan mode", the cleanest possible Max: the firmware still owns the fans, we only ask for full speed.

Registers read from regmap debugfs (`/sys/kernel/debug/regmap/INOU0000:00/registers`):
project ID `0x0740 = 0x10`, `0x0741 = 0x81`, `0x0782 = 0x9d` (FAN_V2_NEW + FAN_V3 + office table).

## 5. The patch (`patches/0003`)

1. Add `EC_ADDR_MANUAL_FAN_CTRL` to the readable, writeable and **volatile** register lists. It is volatile
   because the EC or a hotkey can change it, so regmap must not cache it (and `regcache_sync` on resume skips it).
2. Add the new feature bit `UNIWILL_FEATURE_FAN_BOOST` and expose `HWMON_PWM_ENABLE` on both channels.
3. Add `pwmN_enable` read/write following the hwmon ABI (0/1/2). The EC is the source of truth on
   read, so a boost toggled by a hotkey shows up correctly.
4. In manual mode, `pwmN = 255` turns boost on. Other values return `-EINVAL`, and writes outside manual mode return `-EBUSY`.
5. Boost is cleared at probe, at unbind (a devm action) and at shutdown, and restored on resume.
6. Add a DMI entry `Avell High Performance` / `Avell A52 MOB` with only the verified features.
7. Add `MODULE_VERSION("7.2.15-avell1")` so you can see which build is running.

The checks were against the kernel docs: hwmon `sysfs-interface`, `hwmon-kernel-api.rst`, and the ThinkPad,
`nct6775` and `gpd-fan` docs, which use the same 0/1/2 meaning. DKMS was checked against `man dkms`
(`DEST_MODULE_LOCATION` must start with `/kernel`).

## 6. Test results

The module was built with `W=1` and produced no warnings. It was loaded without `force`: no "unsupported device" warning and no battery hook.

| Step | fan1 | fan2 | pwm1 | CPU |
|---|---|---|---|---|
| auto | 2002 | 1978 | 76 | 64 °C |
| `pwm1_enable=0`, +3 s | 5514 | 5337 | 255 | 51 °C |
| +18 s | 5500 | 5324 | 255 | 47 °C |
| `pwm1_enable=2`, +3 s | 2000 | 2041 | 76 | 48 °C |
| +6 s | 2002 | 0 | 76 | 48 °C |

Fan 2 at 0 RPM is the EC stopping it while cool; its duty (`pwm2`) also reads 0.

The plugin helper's exact sequence (`enable=1`, `pwm=255`; then `pwm=76` (rejected and ignored), `enable=2`) works.
The plugin's own `discover.py` reports `maxFan = hwmon`.
