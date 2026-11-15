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

## 8. Fan mode reporting (#3)

Before, writing `pwmN_enable = 1` only recorded "manual"; the EC kept its own curve until `pwmN = 255` was
written, so the mode read `1` while the firmware was in control. Now (`7.2.15-avell2`, still in the upstreamable `patches/0003`):

- Entering manual mode turns Max fan on at once, as gpd-fan does.
- The mode read follows EC `0x0751` bit 6: bit clear reads `2` (automatic), whatever was written before.
- Writing `2` always clears the bit.

`driver/checks/fans.sh` on the device (observed): auto → manual reaches about 5800 RPM at once (`pwm1 = 255`), max → manual stays full,
every switch back to `2` returns to the firmware curve (`pwm1` about 76), and three cycles of the Cooling widget's own sequence all pass.

## 9. TGP offset: first live test (#4)

Test build `7.2.15-avell3` with `UNIWILL_FEATURE_NVIDIA_CTGP_CONTROL` enabled and the offset clamped to 20 W (not committed).

| Step | Result | Status |
|---|---|---|
| Baseline `nvidia-smi` | default 60 W, enforced 60 W, max 95 W | observed |
| `ctgp_offset = 20` (EC `0x0744`) | the write is accepted and reads back 20 | observed |
| `nvidia-smi` 3 s later | default 60 W, enforced 60 W: **unchanged** | observed |
| `ctgp_offset = 21` | rejected (`-EINVAL`) | observed |
| Restore | 0 | observed |

**Why it had no effect** (observed in `evidence/acpi/DSDT.dsl`, this machine's own tables):

- The ACPI field map of EC RAM (`OperationRegion ECMG` at `0xFE410000`) names `0x0743` bits 0–1 `GNEN`/`ECDC` and
  `0x0744`–`0x0747` `CTVA`, `DBCT`, `MXDB`, `MIDB`.
- The NVIDIA platform device `\_SB.NPCF` takes its TGP offset from `NPCF.UOCT`, which the firmware sets to
  `EC0.CTWA × 8`. `CTWA` is a **different byte, EC `0x0788`** (holds 0).
- `UOCT` is only refreshed, followed by `Notify (NPCF, 0xC0)`, by EC query `_Q83` or by the vendor method
  `\_SB.INOU.T1WR` (`Arg0 = 0x1171`: writes `CTWA = Arg1`; `Arg0 = 0x71`: re-reads `CTWA`).
  `INOU` is the ACPI device this driver already binds to (`INOU0000`). A plain EC RAM write triggers neither.
- `T1WR` also covers Dynamic Boost (`0x1172`: `NPCF.DBAC = Arg1`; `0x1173`: Dynamic Boost min/max into
  `EC0.DBD1/DBD2` and `NPCF.AMAT/AMIT`; `0x73`: from `EC0.DBEN`), `0x2273` (`NPCF.ATPP`), `0x1176` (`EC0.CGCT`, notifies the GPU),
  and `0x81` (CPU `APL1`).

So whether EC `0x0744` matters to the EC itself is **unknown**. The graphics driver only learns a TGP offset through
`CTWA` plus a notify. That alternative route needs a decision before any write to `0x0788` or a call to `T1WR`.

### 9.1 The T1WR route (decision: write both `0x0744` and `T1WR`)

Build `7.2.15-avell3` (upstreamable `patches/0004`): `ctgp_offset` (clamped to the firmware table's 20 W) writes EC
`0x0744` and calls `\_SB.INOU.T1WR(0x1171, W, 0)`. The offset is 0 at load and is set back to 0 at unload
and shutdown. Resume re-applies it (`0x0744` through the regmap cache, `T1WR` explicitly). Module checked with REA
`inspect_kernel_module` (Evidence `ev_585631cab96002465ebb6fea4d866e6ad8dac9bb137fb4fc07b3ab2e119f724e`: version 7.2.15-avell3, signature present).

| Condition | Default | Enforced | Max | Status |
|---|---|---|---|---|
| No `nvidia-powerd`, TGP offset 0 or 20 (`0x0744` only) | 60 W | 60 W | 95 W | observed |
| `nvidia-powerd` running, TGP offset 0 | 60 W | 85 W | 95 W | observed |
| `nvidia-powerd` running, TGP offset 20 (`0x0744` + `T1WR`) | 60 W | **95 W** | 95 W | observed |
| Back to 0 | 60 W | 85 W | 95 W | observed |
| After a real suspend (s2idle, `rtcwake`) with offset 20 | 60 W | 95 W | 95 W | observed; `ctgp_offset` still reads 20 |
| 21 W | rejected | | | observed |

- The graphics driver's **default** limit stays at 60 W. The offset shows up in the **enforced** limit, as the ticket allowed for.
- **Dynamic Boost** (`nvidia-powerd`, enabled by `scripts/install.sh`): starting the daemon raises the enforced limit by 25 W,
  which matches EC `0x0746` (`MXDB` = `0x19` = 25) rather than the table's 15 W (observed). `nvidia-smi` has no field that
  says "Dynamic Boost enabled". `nvidia-powerd` logs that the firmware disabled the DC (battery) controller, which is expected:
  Dynamic Boost runs on AC only (inferred).
- 60 + 20 + 25 would exceed the 95 W ceiling, so the enforced limit tops out at 95 W (inferred).
- Load run (observed, 2026-11-15, on AC, performance profile, `nvidia-powerd` running): `prime-run glmark2 --off-screen --run-forever -b terrain`, sampled every 2 s for 36 s:

  | TGP offset | Enforced limit under load | GPU draw | GPU temp | Graphics clock | Utilisation |
  |---|---|---|---|---|---|
  | 0 | 85 W (two samples dipped to 77 and 81 W) | 65–68 W | 59 → 68 °C | 1965–1972 MHz | 88–90% |
  | 20 | 95 W | 65–69 W | 63 → 70 °C | 1950–1972 MHz | 74–90% |

  Dynamic Boost adds +25 W under load, and the TGP offset adds +20 W on top (observed). The terrain scene is **not power-limited**:
  it draws about 67 W at a pinned clock in both runs, so it cannot show a performance gain from the higher ceiling. A power-bound
  game is needed for that (unknown until measured). Even at offset 0 the draw is above the old 60 W budget, which only Dynamic Boost allows.

### 9.2 Later re-check (after the #5 work, `7.2.15-avell4`)

With `nvidia-powerd` running (also after restarting it) and on AC: TGP offset 0 gives enforced 60 W, and 20 gives enforced **80 W**
(+20 exactly, as the ticket predicted). The extra +25 W seen at 19:04 (enforced 85 W at offset 0) did not come back,
and the Power profile does not change it at idle (observed). Dynamic Boost adjusts the budget with load, so the enforced
limit at idle may not show it (inferred). Settled by the load run below: Dynamic Boost applies under load.

## 10. Power profile (#5)

Build `7.2.15-avell4`, upstreamable `patches/0005`. Platform-profile handler "uniwill" with three choices, mapped to EC `0x0751`
bits 4/5/7 (TUXEDO's `uw_set_performance_profile_v1` mapping). Bit 6 (Max fan) is never touched; every read-modify-write of `0x0751` takes the same lock.

| Profile | `0x0751` bits | PPD name |
|---|---|---|
| quiet | `0xA0` | power-saver |
| balanced | `0x00` | balanced |
| performance | `0x10` (the firmware's boot value) | performance |

Observed on the device:
- `driver/checks/power-profile.sh` passes 3 times in a row. Max fan survives a profile change (`0xE0` = quiet + Max fan), and the profile survives Max fan off.
- PPD 0.30 (restarted once after the driver first appeared) reports `PlatformDriver: platform_profile`. Setting power-saver,
  balanced and performance in PPD writes `a0`, `00` and `10`, and a kernel-side change shows up in PPD.
- The button next to the power key reports WMI event `0xB0` (`KEY_F14`, code 184) and does **not** change `0x0751` itself.
  The driver now cycles the profile in the kernel (`platform_profile_cycle()`): quiet → balanced → performance, and PPD follows.
- The profile survives an s2idle suspend. The boot value is restored on unload and shutdown.
- One early failed run, with `0x0751` changing right after a write, was not reproduced with nobody touching the button: probably
  late button presses from the key test. A balanced → performance change between 19:16 and 19:33 is unexplained (not reproduced).

Each profile under a 20 s all-core load (`openssl speed`, after 20 s of settling, `--measure`):

| Profile | CPU package | CPU temp | Fan 1 / fan 2 (RPM) |
|---|---|---|---|
| quiet | 11.3 W | 52 °C | 2007 / 0 |
| balanced | 54.8 W | 91 °C | 4834 / 4647 |
| performance | 58.0 W | 90 °C | 5500 / 5259 |

`MSR_PKG_POWER_LIMIT` (`0x610`) stays `0x4283c000a183c0` (PL1 = PL2 = 120 W) in all three profiles, and the MMIO RAPL limits are 0.
So the firmware caps quiet at about 11 W some other way, from the EC side; the mechanism is **unknown**. Module checked with REA
`inspect_kernel_module` (Evidence `ev_a0937c09e466aac5a8ef5082fcc97ef7bb332c779a50a9cb5e45de7de92d8d45`: version 7.2.15-avell4, signature present).

## 11. Direct fan speed and the kernel watchdog (#6)

Build `7.2.15-avell5-local`, **local-only** `patches/0006` on top of the upstreamable 0003-0005. The design is in ADR-0003:
universal fan tables, plus Max fan whenever full speed is needed at once. Module checked with REA `inspect_kernel_module`
(Evidence `ev_7d9d77995c5f103c10025393c7c208478000bf16ad2846c9d34c0514d1fee547`: version 7.2.15-avell5-local, signature present).

Interface (hwmon, both fans): `pwmN_enable` 0 = Max fan, 1 = manual, 2 = automatic. In manual mode `pwmN` takes 0-255 per fan.
Module parameters (0644): `fan_watchdog_sec` (default 5, clamped to 1-60), `fan_hard_temp_cpu` (default 95 °C), `fan_hard_temp_gpu`
(default 87 °C); hysteresis 5 °C. A 1 s work item enforces both while in manual mode. Probe, unload, shutdown and suspend all turn the tables off.

Observed on the device (`driver/checks/direct-fan.sh`, `--sweep`):

| Behaviour | Result |
|---|---|
| Live state before the first write | `0x07C5` = `0x07C6` = 0, tables `0x0F00-0x0F5F` all 0 |
| Fans follow the table | yes, but **slowly**: about +3/255 every 2 s up (81 → 117 in 20 s) and down, so a full swing takes about 2 minutes |
| Enter manual (Max fan start) | 5445 / 5310 RPM within 3 s, mode reads 1 |
| Fans independent | fan 1 at 255 rises while fan 2 at 0 falls |
| Max fan over a low table duty (hard temp set to 30 °C) | back to about 5400 RPM within 6 s, mode still 1, kernel log "forcing Max fan" |
| Max fan released at duty 0 | fans stop straight away (no slow ramp from Max fan down) |
| Watchdog (no write for 5 s) | kernel log "no fan duty written for 5 s, returning to automatic", mode 2 |
| rmmod in manual | `0x07C5` = `0x07C6` = 0, Max fan off |
| s2idle suspend in manual | mode 2 after resume, tables off |
| Cooling widget sequence, 3 cycles | Max fan instantly (about 5450 RPM), back to automatic |

Minimum spinning duty, from standstill, 40 s per step (both fans):

| pwm (of 255) | 8 | 16 | 24 | 32 | 40 | 48 | 64 |
|---|---|---|---|---|---|---|---|
| fan 1 / fan 2 RPM | 0 / 0 | 390 / 406 | 615 / 628 | 838 / 845 | 1055 / 1058 | 1302 / 1292 | 1703 / 1674 |

So the minimum is set to 13 of the EC's 200 (16/255). Duties below half of that round to off (written as 1, which stops the fan at
once; 0 would make the EC spin it at 30% for minutes, per TUXEDO). Anything else below the minimum rounds up to it. Checked through
the table registers: 8 → 13, 3 → off, 5 → off, 12 → 13, 255 → 200.

Hard temperatures (chosen, not measured limits): CPU 95 °C, the temperature the CPU reaches within seconds under all-core load (section 7.4),
so a sane curve is at full there anyway; GPU 87 °C, the usual ceiling for this NVIDIA Laptop GPU class (inferred).
The firmware also runs the table's dummy zones at full speed above 115 °C (CPU) and 120 °C (GPU).

## 12. BIOS image analysis (#18)

`bios/analyze.py` (read-only; image file only) → `evidence/bios/analysis.md`, checked by `bios/test-analyze.sh`. AMI Aptio V,
Setup driver `899407D7-99FE-43D8-9A21-79EC328CAC21` (242 forms), two NVAR stores. Everything is from this machine's image (observed),
except the two marked inferred:

| Target | Where | Factory / current | Route |
|---|---|---|---|
| Overclocking lock | `CpuSetup` `0xDF` (1 byte), next to CFG Lock `0x43` | 1 / 1 (set) | boot-time setting change (inferred: `CpuSetup` is boot-service, non-runtime, non-authenticated) |
| Power limit overrides | `CpuSetup` PL1 override `0x1B`, PL1 `0x17` (mW, 4 bytes), PL1 window `0x1C`; PL2 override `0x1D`, PL2 `0x1E` | PL1 override off, PL2 override on with value 0 | boot-time setting change (inferred) |
| cTDP table | `CpuSetup` `0x5B`/`0x5F` (PL1/PL2 = 120000 mW), window `0x63` = 96 | 120 W / 120 W, matching `MSR 0x610` | same |
| Thermal throttle point | `CpuSetup` Tcc Activation Offset `0x7F` | 5 / 5: throttling at TjMax − 5 = 95 °C, which is where the CPU sits under load (7.4) | boot-time setting change (inferred) |
| Hidden menus | not in the forms: the root form links Main, Advanced, Chipset, Security, Boot and Save & Exit without conditions | n/a | modified image, AMITSE page data (inferred) |

Also observed: Avell ships **OverClocking Feature** (`0x135`) enabled (form default 0, factory default 1) while keeping the Overclocking Lock set.
The firmware fills `SetupCpuFeatures` at boot (it is not stored), and it decides whether several power-limit questions are shown.
