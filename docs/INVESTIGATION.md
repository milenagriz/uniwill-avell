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

## 7. Firmware deep dive

Each finding is marked **observed** (read on this machine), **inferred** (deduced from observations or from
sibling boards, not confirmed here) or **unknown** (not settled yet; the ticket that settles it is named).
The raw material is in `evidence/` (see `evidence/README.md`).

### 7.1 Board identity

| Finding | Status |
|---|---|
| BIOS version `N.1.05AVE04`, DMI board `Avell A52 MOB` | observed |
| Mainboard is Tongfang/Uniwill **GM5TGFX** (barebone GM5TGEO) | inferred: matched from firmware strings and the sibling list below |
| Siblings on the same board and BIOS base N.1.05: Mechrevo Z3 Air, Monster Abra A7 V11.4/V11.5, CyberPowerPC Tracer V Edge I15E, Wootware WootBook Pro III, Avell A70 MOB (RTX 3050) | inferred: from vendor and owner listings |
| No BIOS or EC build newer than the installed one exists for this board, from Avell or a sibling vendor | inferred: research found none, which does not prove none exists |

### 7.2 The firmware's profile table for this machine

The firmware chooses a power table from the GPU, panel and adapter. The tables are 8- and 12-byte buffers in
`evidence/acpi/DSDT.dsl` (around line 70040: `ACPF`, `A8PF`, `ACSF`, `A8SF`, ...), laid out as PL1, PL2, PL4,
thermal offset, TGP offset, Dynamic Boost, all in watts except the thermal offset. The `A8xx` entries hold
`50 50 A5 05 14 0F`; their `ACxx` twins are identical except for TGP offset and Dynamic Boost, which are 0.
The entry selected for this machine reads:

| Field | Value | Status |
|---|---|---|
| CPU PL1 / PL2 / PL4 | 80 W / 80 W / 165 W | observed in the table; which buffer the firmware selects at runtime is inferred from its GPU, panel and adapter conditions |
| Thermal offset | 5 | observed |
| **TGP offset** | +20 W | observed |
| **Dynamic Boost** | +15 W | observed |
| Graphics driver power limit: default / maximum | 60 W / 95 W (`nvidia-smi -q -d POWER`) | observed |
| 60 W + 20 W TGP offset + 15 W Dynamic Boost = the 95 W maximum | inferred |
| The EC's TGP offset register (`0x0744`) holds 0 | observed |
| Whether the EC forwards a TGP offset write to the NVIDIA platform device | **unknown**, settled by the first live test (#4) |

### 7.3 EC facts

EC RAM is memory-mapped at physical `0xFE410000` (4 KiB window). Values are from `evidence/ec/ec_a.bin`, and
are identical in all four dumps. Nothing in this table has been written on this board except the Max fan bit.

| Purpose | Location | Observed value | Write status |
|---|---|---|---|
| Operating mode: Power profile bits and Max fan bit (bit 6) | `0x0751` | `0x10` (performance) | Max fan bit verified (section 6); Power profile bits untested (#5) |
| TGP offset / Dynamic Boost control and offsets | `0x0743`–`0x0746` | `07 00 ff 19` | untested (#4) |
| Direct fan duty, fan 1 / fan 2 | `0x1804`, `0x1809` | not read: outside the mapped window | untested (#6) |
| Feature flags: universal fan control, charging profile | `0x078E` | `0x6C` (both flags set) | read-only |
| Charging profile | `0x07A6` bits 4–5 | 0 | untested (#12) |
| Charge cap | `0x07B9` | 0 | untested (#12) |
| Keyboard status / colour R, G, B | `0x078C` / `0x0769`–`0x076B` | `0x08` / `00 32 32` | untested (#13) |
| CPU power limits | `0x0783`–`0x0785` | 0 | not used: the spec sets CPU limits through powercap instead |
| Project ID, manual control, fan table flags | `0x0740`, `0x0741`, `0x0782` | `0x10`, `0x81`, `0x9d` | read-only |

What the bit layout of `0x0751` means for the three Power profiles is **inferred** from TUXEDO's driver and is confirmed in #5.
EC state resets on a full power-off, which is the rollback for any EC write (inferred from the Uniwill EC design; not needed so far).

### 7.4 Thermals

| Finding | Status |
|---|---|
| With the graphics card asleep, an all-core load (`evidence/scripts/loadtest.py`) brings the CPU to 95 °C within 3 s | observed |
| The CPU settles at about 49 W with both fans at full speed | observed |
| More than 17,000 package throttle events in one boot | observed |
| The cooler is saturated, so any power raise will be limited by temperature until the repaste | inferred |
| EC fan duty under that load: `0x075B/0x075C` = `c8 c8` (200 of 200, full) | observed (`ec_load.bin`) |

### 7.5 CPU power limits and undervolting

| Finding | Status |
|---|---|
| `MSR_PKG_POWER_LIMIT` holds 120 W for both PL1 and PL2 and is **not** locked | observed |
| The overclocking lock is set, which is what blocks undervolting | observed |
| The setup variable that holds the lock is not reachable from the running system | observed |

### 7.6 Battery

| Finding | Status |
|---|---|
| 2900 mAh full charge of 4100 mAh design (71%) | observed |
| The battery is always charged to 100%; no charge limit is exposed | observed |

### 7.7 Flash protection

| Finding | Status |
|---|---|
| Flash lock off, BIOS region writable | observed |
| Management engine in manufacturing mode | observed |
| Boot Guard not enforcing | observed |
| Kept that way on purpose: ADR-0002 | decision |

### 7.8 Warnings from other owners of this board family

| Warning | Status |
|---|---|
| A vendor BIOS update on a sibling wiped the model and serial number; one owner then reported the laptop no longer booting with Dynamic Boost enabled | inferred: owner reports, not reproduced |
| On some older boards from this manufacturer, boot-time setting changes lasted only one boot | inferred: owner reports |
| A sibling board of the same era used a leadless flash package that a clip cannot grip | inferred: the chip on this board is **unknown** until checked by eye (#19) |
