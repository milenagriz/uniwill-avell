# Which firmware settings hold idle power back? (#31)

Research for [#31](https://github.com/milenagriz5-dotcom/uniwill-avell/issues/31) on the map
[#23](https://github.com/milenagriz5-dotcom/uniwill-avell/issues/23). Done on 2026-11-15, read only: the
committed image `evidence/bios/bios-N.1.05AVE04-20261115.bin` (SHA-256 `baa08b4a…52412`), the committed ACPI
tables, and passive reads on the running laptop (kernel 7.2.15-1-omarchy, on AC, one boot of 7 h 20 min).
Nothing was written to the flash, EFI variables, the EC or MSRs.

Each finding is marked **observed** (read here), **inferred** (deduced) or **unknown**, as in INVESTIGATION.

## Answer

1. **The CPU C-state settings in `CpuSetup` do not hold idle back.** Every one is already at its most permissive
   value (C-states on, C1E on, demotion and un-demotion on, Package C State Limit = Auto), and the CPU's own
   register confirms the result: package C-state limit 8 = C10, "unlimited". The cores do reach C10. (observed)
2. **The firmware setting that does hold idle back is in the graphics VBT, not in Setup.** The Intel VBT stored in
   the BIOS image switches **Panel Self Refresh (PSR) off, and DRRS off, for the VBT's panel type**
   (panel type 2, the only type with its bit clear: PSR bitmask `0xfffb`, so every bit is set except bit 2). i915 follows the VBT, so PSR is off even though the
   panel supports it. Without PSR the display engine never reaches DC6 (DC5→DC6 count 0 in this boot), and the
   package spent 40% of the boot in PC2/PC3 and 0.1% in PC10. (VBT and counters observed; that PSR is what
   blocks the deep package states is inferred, and other blockers remain, see 4.) Route: **OS side first**
   (`i915.enable_psr=1` for one boot), and a **modified image** (one bit of the VBT) only if that boot is stable.
   **How much it saves is unknown** until the owner gives a measurement window on battery.
3. **Smaller firmware items**, all with unknown cost:
   - **SA GV disabled** (`SaSetup[0x17d]` = 0): the memory never drops to a lower frequency and voltage point. Route: boot-time setting change.
   - **VMD enabled** (`SaSetup[0x101]` = 1), with nothing behind it. The `vmd` driver binds to it and keeps it in D0. Route: boot-time setting change back to the factory 0.
   - **The CPU root port to the graphics card advertises no ASPM** (LnkCap). This matters only while the card is awake.
4. **S0ix versus S3: the firmware offers both, and the default s2idle never reaches S0ix.** The FADT sets
   Low Power S0 Idle, `_S3` is present and Setup has S3 enabled, so Linux defaults to s2idle. Yet
   SLP_S0 residency is **0** after at least 5 s2idle suspends this boot. (observed) Route: OS side.
   Either `mem_sleep_default=deep`, once the owner has tested S3 resume, or a hunt for the S0ix blocker during an
   owner-run suspend. No firmware change is needed for either.
5. **ASPM on the SSD link is not a firmware problem.** The firmware enables L1 and L1.1/L1.2 on root port 9
   (`PchSetup[0x119]` = Auto, `[0x281]` = L1.1 & L1.2), and the kernel enabled both ends at boot. The "ASPM off on
   the SSD's side" in INVESTIGATION 18 has another cause: the **NVMe controller dropped off the bus at 17:38**
   (`CSTS=0xffffffff`, after a PCIe RxErr) and was reset. After that reset the SSD's link control register has
   ASPM *and* Common Clock cleared, while the root port and the kernel's bookkeeping still say enabled.
   (event observed; that the reset caused the mismatch is inferred) This is a reliability question for a new ticket, not an idle setting.
6. **The biggest idle drains found are on the OS side, not in firmware.** The graphics card was awake (D0, P8)
   for 34% of the boot and the whole time it was watched, at **6.9 W** by its own sensor. Linux's r8169 driver turns
   off ASPM L1 on the Ethernet link (cable unplugged). EPP is `performance`, because the
   performance Power profile is applied on AC. (observed)

## Settings, one by one

| Setting | Location | Current value | Effect on idle | Route | Status |
|---|---|---|---|---|---|
| C states / Enhanced C-states (C1E) | `CpuSetup[0x14]` / `[0x15]` | 1 / 1 (Enabled) | none: already on | n/a | observed |
| C-State Auto Demotion / Un-demotion | `CpuSetup[0x3e]` / `[0x3f]` | 1 / 1 | none | n/a | observed |
| Package C-State Demotion / Un-demotion | `CpuSetup[0x40]` / `[0x41]` | 1 / 1 | none | n/a | observed |
| Package C State Limit | `CpuSetup[0x4b]` (visible) and `[0x244]` (hidden numeric) | 255 = Auto ("deepest available") / 200 | none: `MSR_PKG_CST_CONFIG_CONTROL` = `0x70008008`, limit 8 (C10, unlimited), locked | n/a | observed |
| C6DRAM | `CpuSetup[0xe5]` | 1 | none | n/a | observed |
| Core C-states offered to the OS | ACPI `_CST` | C1 (MWAIT 0x00), C7s (0x31), C10 (0x60) | none: cpu0 spent 78% of the boot in C10 | n/a | observed |
| **PSR for the eDP panel** | VBT, block 44 (LFP power), `psr` bitmask at VBT offset `0xf36`, bit = panel type | `0xfffb`: off for panel type 2 | display cannot reach DC6, which blocks deep package states while the screen is on | OS side: `i915.enable_psr=1`; modified image: set bit 2 (`0xffff`) and fix the VBT checksum | VBT observed; effect inferred; size unknown |
| DRRS for the eDP panel | VBT block 44 `drrs` bitmask at `0xf38` | `0xfffb`: off for panel type 2 | no refresh-rate drop at idle | OS side has no override in i915 (modified image only) | observed; size unknown |
| SA GV | `SaSetup[0x17d]` (Chipset > SA > Memory Configuration, visible) | 0 = Disabled (options: Disabled, fixed point 1-4, 5 = Enabled) | memory stays at the 3200 point | boot-time setting change (`SaSetup` is a non-authenticated boot-service variable, as `CpuSetup` in INVESTIGATION 12) | observed; size unknown |
| Enable VMD controller | `SaSetup[0x101]` (form `VMD setup menu`, reachable only from the unlinked OEM page) | form and factory default 0; store 2 = 1, and VMD `00:0e.0` is live with `vmd` bound, `power/control` = on | a D0 device with nothing behind it | boot-time setting change to 0 | observed; size unknown, probably small (inferred) |
| PEG (CPU) root port ASPM | `SaSetup[0x52]`-`[0x55]` = 4 (Auto); `00:01.0` LnkCap: "ASPM not supported" | no L1 on the dGPU link | costs only while the card is awake; when it is in D3cold the link is off | unknown which knob clears LnkCap; not worth chasing before the dGPU wake issue | observed |
| PCH root port 9 (SSD) ASPM / L1 substates | `PchSetup[0x119]` = 4 (Auto) / `[0x281]` = 2 (L1.1 & L1.2) | root port: L1 + L1.1/L1.2 on | none from firmware; see the NVMe reset | n/a | observed |
| PCH root port 5 (Ethernet) ASPM / L1 substates | `PchSetup[0x115]` = 4 / `[0x27d]` = 2 | L1 off on both ends, set by the r8169 driver, not the BIOS | the link stays in L0 | OS side (r8169 policy), see below | observed |
| Native ASPM | `Setup[0x2f]` | 2 = Auto; FADT "PCIe ASPM Not Supported" = 0; `_OSC` grants PCIeCapability and LTR to the OS | the OS owns ASPM | n/a | observed |
| Low Power S0 Idle Capability | `Setup[0x39]` | 1 → FADT Low Power S0 Idle = 1 | makes Linux default to s2idle | OS side: `mem_sleep_default=deep`; or boot-time setting change to 0 | observed |
| ACPI Sleep State | `Setup[0x998]` | 2 = S3; DSDT `_S3` present; kernel "supports S0 S3 S4 S5" | S3 is available | n/a | observed |
| LPM S0i2.0 / S0i3.0 (S0i2.1, S0i3.1-3.4 off) | `PchSetup[0x13]`, `[0x16]` (others `[0x14]`-`[0x1a]`) | 1 / 1 | the PMC substates are allowed | n/a | observed |
| PEP constraints | `Setup[0x3a]`-`[0x994]` | standard Intel set; dGPU (`PEP PCIe PEG0/DG`) = 0, VMD = 1 | read by Windows, only logged by Linux | n/a | observed (Linux behaviour inferred) |
| PCIe clock gating / power gating | `SaSetup[0x241]` / `[0x242]` | 1 / 1 | none | n/a | observed |
| DMI ASPM | `PchSetup[0x469]` = 2 (L1), `SaSetup[0x38e]` = 2 (L1) | on | none | n/a | observed |

The full list (291 matching questions, with options and conditions) comes from the script in "Method" below.

## Evidence

### Firmware: Setup forms and NVAR values (observed)

Read with `bios/analyze.py`'s parser (UEFIExtract + ifrextractor over a private copy of the image, NVAR stores parsed
by `bios/nvar.py`). The script also prints each question's options. **The repo parser drops every question whose
Help text runs over several lines**, which includes all the per-root-port `ASPM` questions, so this run joins those lines
first (see Method). Excerpts:

```
Package C State Limit | CpuSetup[0x4b] | ifr 255 fac 255 cur 255/255 | linked | Advanced > Power & Performance > CPU - Power Management Control
    0=C0/C1; 1=C2; 2=C3; 3=C6; 4=C7; 5=C7S; 6=C8; 7=C9; 8=C10; 254=Cpu Default; 255=Auto
Package C State Limit | CpuSetup[0x244] | ifr Other fac 200 cur 200/200 | SuppressIf(True)
Low Power S0 Idle Capability | Setup[0x39] | ifr 1 fac 1 cur 1/1 | linked | Advanced > RC ACPI Settings
ACPI Sleep State | Setup[0x998] | ifr 2 fac 2 cur 2/2 | linked | Advanced > ACPI Settings | 0=Suspend Disabled; 2=S3 (Suspend to RAM)
SA GV | SaSetup[0x17d] | ifr 0 fac 0 cur 0/0 | linked | Chipset > SA > Memory Configuration
    0=Disabled; 1=Fixed to 1st Point; 2=Fixed to 2nd Point; 3=Fixed to 3rd Point; 4=Fixed to 4th Point; 5=Enabled
Enable VMD controller | SaSetup[0x101] | ifr 0 fac 0 cur 0/1 | conditional | GCU - OverClock Recovery > VMD Setup Menu
ASPM | PchSetup[0x119] | ifr 4 fac 4 cur 4/4 | PCH-IO > PCI Express Configuration > PCI Express Root Port 9 | 0=Disabled; 1=L0s; 2=L1; 3=L0sL1; 4=Auto
L1 Substates | PchSetup[0x281] | ifr 2 fac 2 cur 2/2 | ... Root Port 9 | 0=Disabled; 1=L1.1; 2=L1.1 & L1.2
```

("ifr" = form default, "fac" = `StdDefaults`, "cur" = NVAR store 1 / store 2.)

REA: the Setup driver PE (`899407D7-…`, body SHA-256 `3aeb08c4d3526bd27bcfe4e4d664aaac2bfd26db205fcf15a842809485ad0d91`) was
opened in REA (Hopper). Overview Evidence `ev_449dd8f6d765baef09b406f34ec6046dfdaa4f5026c1d147fe75186876890cb5`. A literal search for
"Package C State Limit" (`ev_b92e70fe3969be69b4a6eb34213bdc77410c9a869ac553f7e43ce40c5b9cb6e7`) and the other prompts returns no
match: Hopper's string inventory does not cover the UTF-16 HII package. That is a gap in REA's coverage, not evidence that the
strings are missing. The form text therefore comes from ifrextractor. REA's byte search and raw-binary mode need Ghidra, which is not
configured on this machine.

### Firmware: the VBT (observed)

- The running VBT (`/sys/kernel/debug/dri/0000:00:02.0/i915_vbt`, 8704 bytes) has SHA-256
  `e179c8b4c2c7959d9319e91e680b54d03d58201fa2dfcf96b151e04e109cf59e`. It is **byte-identical** to the raw section of FFS file
  `56752DA9-DE6B-4895-8819-1945B6B76C22` in the BIOS region of the committed image (UEFIExtract `all` on a private copy).
  REA `inspect_firmware_regions` over that section: Evidence `ev_508926480ce877ff7f9d5ca0a6f1f6d6f1af4f3ef99ed0f99daff22aa7d50811`
  (same SHA-256; binwalk finds only Intel's copyright text, so the decoding below is by the kernel's struct layout).
- Decoded with the layout in the kernel's `intel_vbt_defs.h`: signature `$VBT TIGERLAKE`, BDB version **240**, blocks
  1,2,9,10,12,20,27,40,42,43,44,46,51,52,56,57,58,252-254. LFP options (block 40): `panel_type` = **2**, `panel_type2` = 3.
  LFP power (block 44, 58 bytes): `dpst` = `0xffff`, **`psr` = `0xfffb`**, **`drrs` = `0xfffb`** (bytes at `0xf34`: `ff ff fb ff fb ff`).
  Driver features (block 12) has `psr_enabled` = 1, but i915 ignores that bit from BDB 228 on.
- How i915 uses it ([intel_bios.c](https://github.com/torvalds/linux/blob/aebb7274476403040d582c61478df8f4a7bf9cd3/drivers/gpu/drm/i915/display/intel_bios.c),
  `parse_power_conservation_features`): for BDB ≥ 228, `panel->vbt.psr.enable = panel_bool(power->psr, panel_type)`, which is
  bit `panel_type` of the bitmask. [intel_psr.c](https://github.com/torvalds/linux/blob/aebb7274476403040d582c61478df8f4a7bf9cd3/drivers/gpu/drm/i915/display/intel_psr.c)
  `psr_global_enabled`: on eDP with the default parameter, PSR follows `vbt.psr.enable`.
- On the machine: `i915.enable_psr` = -1 (default: follow the VBT), `i915_edp_psr_status`: "Sink support: PSR = yes [0x01] …
  PSR mode: disabled". `i915_dmc_info`: DMC 2.12 loaded, "DC3 -> DC5 count: 1, DC5 -> DC6 count: 0".
- That the selected panel type is 2 is **inferred**: it is the VBT's value, and it is the only one whose PSR bit is clear, which
  matches "PSR mode: disabled". The OpRegion panel type takes precedence in i915 and was not read.
- Why PSR helps, from i915's own documentation (intel_psr.c, "DOC: Panel Self Refresh"): PSR "allows the display to go to lower
  standby states when system is idle but display is on as it eliminates display refresh request to DDR memory completely".
- **Unknown**: why Avell (or the reference VBT) disabled PSR and DRRS for this panel. It may be a real panel or timing-controller
  problem (flicker, freezes) or just a default. The legacy VGA option ROM in the image carries an older `$VBT ICELAKE` (BDB 223)
  that the OS never uses.

### Firmware: ACPI (observed, `evidence/acpi/`)

- FADT (`iasl -d FACP.dat`): Boot Flags `0002`, **PCIe ASPM Not Supported = 0**, **Low Power S0 Idle = 1**, Hardware Reduced = 0.
- DSDT: `Name (_S3, Package (0x04) { 0x05, … })` at line 5885, unconditional; `SS3` = One. `PEPD` (`INT33A1`) with the LPS0 `_DSM`
  (`c4eb40a0-6cd2-11e2-bcfd-0800200c9a66`), present when `S0ID` = 1 (line 48678 on).
- LPIT: two native C-state entries (MWAIT 0x60 = C10, counter MSR `0x632` and the PMC SLP_S0 counter at `0xFE00193C`), and a
  third entry flagged disabled.
- Boot log: "ACPI: PM: (supports S0 S3 S4 S5)", "Low-power S0 idle used by default for system suspend",
  "`_OSC: OS now controls [PCIeHotplug SHPCHotplug PME AER PCIeCapability LTR DPC]`".
- Kernel docs ([sleep-states.rst](https://github.com/torvalds/linux/blob/aebb7274476403040d582c61478df8f4a7bf9cd3/Documentation/admin-guide/pm/sleep-states.rst)):
  the default variant can be "s2idle" "depending on the information in the ACPI tables … even if suspend-to-RAM is supported", and
  `mem_sleep_default` overrides it.

### On the machine (observed)

The machine was **not idle** during these reads: other work was running (load average 1.5; turbostat 19-57% busy, 22-44 W
package). So the readings below show **which states are reachable**, not idle watts.

```
$ sudo cat /sys/kernel/debug/pmc_core/package_cstate_show      # µs since boot (count * 1000 / tsc_khz, pmc/core.c)
Package C2 : 3982644142      # 3983 s
Package C3 : 6729616131      # 6730 s
Package C6 : 0
Package C7 : 3239
Package C8 : 0
Package C9 : 0
Package C10 : 29692453       # 29.7 s, of 26,433 s uptime
$ sudo cat /sys/kernel/debug/pmc_core/substate_residencies /sys/kernel/debug/pmc_core/slp_s0_residency_usec
S0i2.0 0 / S0i3.0 0 / 0
$ cat /sys/devices/system/cpu/cpuidle/low_power_idle_cpu_residency_us /sys/devices/system/cpu/cpuidle/low_power_idle_system_residency_us
29692453 / 0
$ journalctl -b | grep "PM: suspend entry"        # 16:58, 19:05, 19:13, 20:16, 20:35 … all "(s2idle)"
$ sudo turbostat --debug …
cpu0: MSR_IA32_POWER_CTL: 0x0024005b (C1E auto-promotion: ENabled)
cpu0: MSR_PKG_CST_CONFIG_CONTROL: 0x70008008 (UNdemote-C1, locked, pkg-cstate-limit=8 (unlimited))
cpu0: MSR_HWP_REQUEST: 0x00002d08 (min 8 max 45 des 0 epp 0x0 window 0x0 pkg 0x0)
cpu0: EPB: 0 (performance)
$ for s in /sys/devices/system/cpu/cpu0/cpuidle/state*; do …            # driver intel_idle, governor menu
state1 C1_ACPI "ACPI FFH MWAIT 0x0"  state2 C2_ACPI "ACPI FFH MWAIT 0x31"  state3 C3_ACPI "ACPI FFH MWAIT 0x60" (time 20,533 s)
```

`rdmsr -a 0xe2` gives `0x70008008` on 10 CPUs and `0x74008008` on CPUs 1 and 7 (C1 auto-demotion bit 26 also set). The limit and
lock bits are the same on all 12. intel_idle has no Tiger Lake entry in its model table at
[aebb727](https://github.com/torvalds/linux/blob/aebb7274476403040d582c61478df8f4a7bf9cd3/drivers/idle/intel_idle.c) (no `TIGERLAKE` match),
so it runs from the ACPI `_CST`. That is why the states are named `C*_ACPI`.

**dGPU** (`01:00.0`, nvidia 615.78.08, `DynamicPowerManagement=3`, "Runtime D3 status: Enabled (fine-grained)"):
`runtime_status` = active throughout 30 s of sampling. `runtime_active_time` 9.05e6 ms against `runtime_suspended_time` 17.4e6 ms.
`nvidia-smi`: `P8, 6.89 W, 210 MHz, 0 %, 2 MiB`, no processes. Holders of `/dev/nvidia0`: Hyprland, nvidia-powerd, and for a while
zed-editor. avelld reads the load with `nvidia-smi` every 2 s while the card is awake (`daemon/src/engine.rs` `GPU_SAMPLE`, guarded by
`runtime_status` = active). Its reaped children's CPU time grew by 0.17 s in 10 s, so it is spawning. Whether this polling
keeps the card from going back to sleep is **unknown**: it needs a test with the daemon stopped (owner window, because stopping avelld
hands the fans back to the EC). NVIDIA's RTD3 documentation:
[dynamicpowermanagement.html](https://download.nvidia.com/XFree86/Linux-x86_64/580.95.05/README/dynamicpowermanagement.html) (580 README;
the installed driver is 615).

**PCIe links** (`sudo lspci -vvv`, kernel `link/` files):

| Link | Root port LnkCtl | Endpoint LnkCtl | Endpoint L1SubCtl1 | Kernel `link/` |
|---|---|---|---|---|
| `00:01.0` → dGPU | LnkCap "ASPM not supported", Disabled | Disabled | all off | `clkpm=1` only |
| `00:1c.0` → RTL8125B (`enp2s0`, link down) | Disabled | Disabled | all off | `l1_aspm=0`, all substates 0 |
| `00:1d.0` → NVMe | **L1 Enabled, CommClk+** | **Disabled, CommClk-** | all off | `l1_aspm=1 l1_1=1 l1_2=1` |

- Ethernet: r8169 calls `pci_disable_link_state(pdev, PCIE_LINK_STATE_L1)` at probe unless the chip's vendor flag marks ASPM safe
  ([r8169_main.c](https://github.com/torvalds/linux/blob/aebb7274476403040d582c61478df8f4a7bf9cd3/drivers/net/ethernet/realtek/r8169_main.c),
  `rtl_aspm_is_safe`). The boot log has no "System vendor flags ASPM as safe", so this is the driver's choice, not the BIOS's.
  `power/control` = on for `02:00.0` and `03:00.0` (runtime PM not allowed). (observed)
- NVMe: `17:37:54 pcieport 0000:00:1d.0: … RxErr`, then `nvme nvme0: controller is down; will reset: CSTS=0xffffffff,
  PCI_STATUS=0x10` / "Does your device have a faulty power saving mode enabled?" / `nvme 0000:03:00.0: enabling device`. The
  hint text is printed whenever CSTS reads all ones
  ([pci.c](https://github.com/torvalds/linux/blob/aebb7274476403040d582c61478df8f4a7bf9cd3/drivers/nvme/host/pci.c)), so it does not identify
  the cause. APST is on (PS3 0.1 W after 100 ms idle, PS4 5 mW), `default_ps_max_latency_us` = 100000. Only this one boot is in the
  journal, so it is **unknown** whether the drop recurs. INVESTIGATION 18 was written (22:08) after the drop.

**PMC** (`/sys/kernel/debug/pmc_core/`, live, under load): LTR aggregated snoop 653 µs (CNVi 646 µs, xHCI 500 µs, no LTR_IGNORE).
Live status: xHCI not power-gated, PCIe and USB PLLs on, CPU_C10_REQ 0. That is expected with the system busy, so it says nothing about idle.

## What the owner needs to do (cannot be done from here)

1. **A measurement window on battery**, with this agent work stopped, the screen on at a fixed brightness, and Wi-Fi on. Per step:
   `sudo turbostat --quiet --interval 30 --num_iterations 4 --show Busy%,Pkg%pc2,Pkg%pc3,Pkg%pc6,Pkg%pc8,Pk%pc10,SYS%LPI,PkgWatt,CorWatt,GFXWatt`,
   battery `current_now` × `voltage_now` every 10 s (`BAT0` has no `power_now`), the `i915_dmc_info` DC counters before and after, and
   the dGPU `runtime_status`. Then one change at a time, each for one boot: `i915.enable_psr=1`, then avelld stopped, then
   (if wanted) the boot-time setting changes. That gives the "by how much" that this ticket could not.
2. **One S3 test** (`echo deep > /sys/power/mem_sleep` then suspend, and check that resume, Wi-Fi, keyboard backlight and the dGPU come back)
   before choosing `mem_sleep_default=deep`. Or, to keep s2idle: one suspend with `/sys/power/pm_debug_messages` = 1, to see which
   LPS0 constraint or PMC requirement blocks S0ix.

## Decisions this surfaces

- **PSR**: try `i915.enable_psr=1` for one boot (OS side, reversible by removing the parameter). If it is stable and saves power,
  choose between keeping the parameter and a modified image that sets VBT `psr` bit 2 (`0xfffb` → `0xffff` at VBT offset `0xf36`,
  plus the VBT checksum). The modified image belongs in the Modified-image spec, together with DRRS (`0xf38`), which has no i915 override.
- **Suspend**: S3 (`mem_sleep_default=deep`) or fixing s2idle.
- **Boot-time setting changes**: whether SA GV (`SaSetup[0x17d]` → 5) and VMD off (`SaSetup[0x101]` → 0) join the boot-time
  setting change of #20/#21. Each would be measured first. Both are human-only (ADR-0002).

## Method

- Setup questions: a scratch script that imports `bios/analyze.py` and `bios/nvar.py` from the repo. It joins IFR lines whose `Help:`
  string spans several lines, then lists every question whose prompt matches a power regex (C-states, ASPM, L1 substates, S0ix, S3,
  VMD, SA GV, PEP, LTR, gating), with its offset, form default, `StdDefaults` value, both NVAR stores, menu path, options and conditions.
  Run as `python3 -I dump_idle.py <repo>/bios <image> '<regex>'` over the committed image.
- VBT: `sudo cat /sys/kernel/debug/dri/0000:00:02.0/i915_vbt`. Image copy: `uefiextract image.bin all` on a private copy, then
  `grep -rlaF BIOS_DATA_BLOCK`. Both decoded with a few lines of JavaScript that follow `struct vbt_header`, `struct bdb_header`, block 40
  `bdb_lfp_options` and block 44 `bdb_lfp_power` (`psr` at block offset 24: 1 byte of features + 5 × 4 bytes of ALS entries + 1 byte
  + `dpst` u16).
- On the machine, all read only: `turbostat` (2026.04.21), `rdmsr` (after `modprobe msr`; no `wrmsr`), `lspci -vvv`, sysfs and
  debugfs reads (`pmc_core`, `dri`, `cpuidle`, `power/`), `nvme get-feature`/`id-ctrl`, `nvidia-smi --query-gpu` (the card was already awake),
  `journalctl -k -b`.
- Kernel sources: torvalds/linux at `aebb7274476403040d582c61478df8f4a7bf9cd3` (2026-11-15). Nothing was downloaded from a
  vendor or a sibling board.
