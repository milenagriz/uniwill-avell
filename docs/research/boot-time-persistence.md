# Do boot-time setting changes stick on this image? (#24)

Research for [#24](https://github.com/milenagriz5-dotcom/uniwill-avell/issues/24), part of the map
[#23](https://github.com/milenagriz5-dotcom/uniwill-avell/issues/23). Everything here was read from the committed
backup `evidence/bios/bios-N.1.05AVE04-20261115.bin` (SHA-256 `baa08b4a…52412`) and, read-only, from the running
system. Nothing was written to the flash, EFI variables, EC or MSRs.

## Answer

**Mostly yes. The image has no generic reset of `CpuSetup`: no setup checksum, no defaults restore at every boot, and no
variable lock on `CpuSetup`. A value written with a boot-time tool is stored in the SPI NVRAM and kept across reboots and
power-offs. The exception is three groups of fields that firmware code rewrites from another source. The one that explains
"lasts one boot" is the undervolt.**

- **Reverted on every boot: Core Voltage Offset, Offset Prefix and Core Voltage** (`CpuSetup` `0x13B`, `0x13D`, `0x13E`).
  On each boot the OEM driver `OemOcDxe` copies these three fields from the OEM variable `UniWillVariable` into
  `CpuSetup` (observed in the code). A boot-time undervolt therefore lasts at most one boot: the boot that reads it,
  before `OemOcDxe` writes the `UniWillVariable` values back. This is the same symptom as the "lasts one boot" reports
  (INVESTIGATION 7.8).
- **Kept: Overclocking Lock (`0xDF`), CFG Lock (`0x43`), the power-limit overrides and values (`0x17`–`0x1E`),
  Package Power Limit MSR Lock (`0x30`) and Tcc Activation Offset (`0x7F`).** No module rewrites these on a normal
  boot. The NVRAM history agrees: `CpuSetup` was not rewritten once in at least 8 recorded boots and 2 setup-screen saves.
- **Reverted only on events:**
  - An overclocking watchdog failure (a hang or unexpected reset during a covered boot) makes `DxeOverClock` restore
    the overclocking fields, the lock and the power limits included, from saved copies.
  - EC RAM `0x0741` bit 7 set at boot makes `OemOcPei` and `OemOcDxe` turn the OverClocking Feature off.
  - Load Defaults in the setup screen, and any NVRAM reset, bring back the factory defaults. What triggers an NVRAM
    reset is still unknown.
- **Routes that persist across a reboot and a full power-off:**
  - A GRUB `setup_var`-style command or a UEFI-shell tool works for the "kept" fields (inferred: same write path as the
    setup screen).
  - New defaults in a modified image only matter after defaults are loaded or the NVRAM is reset, so they are a safety
    net, not a route on their own.
  - For the undervolt, the value has to change in `UniWillVariable`, or `OemOcDxe` has to be patched in a modified
    image. Simpler still: unlock once (Overclocking Lock, which persists) and apply the offset at runtime through MSR
    `0x150`, which skips the BIOS voltage fields altogether.

## Findings

Status as in INVESTIGATION: **observed** (read from this image or this machine), **inferred** (deduced, not tried on the
device), **unknown**.

### 1. No generic reset, checksum or lock on `CpuSetup`

| Finding | Status |
|---|---|
| `CpuSetup` (`B08F97FF-…-5E9E9B0ADB32`, 0x249 bytes, NVAR attributes `0x82`: non-volatile, boot-service only) has **no update** in either NVAR store: one copy, values equal to the factory defaults except for fields computed at first boot (turbo ratios, cTDP 120 W, flex ratio 27) | observed (`bios/nvar.py` over the image; see "Commands") |
| Store 2 (`0x1230090`) is the active store, a compacted copy of store 1 (`0x1200090`) plus later writes: `MonotonicCounter` runs `0x4FB`→`0x4FE` in store 1 and `0x4FE`→`0x503` in store 2, and store 2 holds the newer boot entry (`LimineLastBootedEntry` = Omarchy, CachyOS in store 1) | observed |
| So **at least 8 boots** (3 in store 1, 5 in store 2) went by without any `CpuSetup` write, and so did two setup-screen saves (store 2 entries `0x124FF86`–`0x1251685`: Wake on LAN on, `SaSetup`, `FixedBoot`, `UniWillVariable`; and `0x1252FC2`–`0x1253D27`) | observed |
| Several modules write `CpuSetup` back on every boot (`OemOcDxe`, `DxeOverClock`, `Setup`). Since no new NVAR entry appears, a write with unchanged data is dropped by the AMI NVRAM driver. A changed field therefore survives unless code recomputes it | inferred (from the NVAR history and the code below) |
| `CpuSetup` is not locked: `EDKII_VARIABLE_LOCK_PROTOCOL` (`CD3D0A05-…`) is used by 24 modules. The ones traced lock other variables: `Setup` driver → `TcgNvmeVar`, `SaveMemoryConfig` → `MemoryConfig`, `NvramDxe` → `HwErrRecSupport`, `Bds` → capsule/firmware-update variables. The remaining users were not traced to a name | observed (bounded search); "not locked" is inferred |
| `EnhancePeiVariable` wraps the PEI `GetVariable`. It returns the built-in default only when the variable is **missing** (`EFI_NOT_FOUND`), so it validates nothing and rewrites nothing | observed (disassembly, entry `0x380`, `0x3E4`) |
| No module was found that checksums `CpuSetup` or restores `StdDefaults` on every boot | inferred (bounded: every module that names `CpuSetup` or its GUID was listed; the 28 DXE/SMM candidates were scanned for `SetVariable` calls) |

### 2. The voltage fields are rewritten from `UniWillVariable` on every boot (`OemOcDxe`)

`OemOcDxe` (FFS `37D40C9C-0622-4478-9801-4678B99684A0`, PE32 SHA-256 `6f379888…d0a8`).

| Finding | Status |
|---|---|
| Entry `0x4BC` reads EC RAM `0x0741` (EC commands `0xA3`/`0xA2` set the address, `0xA4` reads it, over ports `0x66`/`0x62`). If bit 7 is set it calls `0x59C`, otherwise `0x6C8` | observed (REA `ev_f8c61617868e61f47c14c91374691a1a314d534cf98bf3b6871530cbc2514210`) |
| `0x6C8` (normal path, no condition other than bit 7 clear) reads `CpuSetup`, `UniWillVariable` and `Setup`. When `UniWillVariable[0x5D] == 1` it copies: `CpuSetup[0x13D]` (Offset Prefix) from the mode `UW[0x66]`; `CpuSetup[0x13B]` (Core Voltage Offset, mV) from `UW[0x64]` (mode 1, or mode 2 when `UW[0x64] != 0`) or `UW[0x3A]`; `CpuSetup[0x13E]` (Core Voltage, mV) from `UW[0x34]`. It also sets `UW[0x36] = 2000` and `UW[0x3C] = 100`, then writes `Setup`, `CpuSetup` and `UniWillVariable` back | observed (disassembly `0x860`–`0x968`; REA `ev_f6c64636d225acdd40bf2ee63de7464193f11d6c2d49875e3d05ee48f2d3c249`) |
| Field names from the forms: `0x13B` "Core Voltage Offset" (0–1000), `0x13D` "Offset Prefix" (0 = +, 1 = −), `0x13E` "Core Voltage" (0–2000 mV, extra turbo voltage) | observed (ifrextractor over the Setup driver) |
| Current values: `UW[0x5D] = 1`, `UW[0x66] = 1`, `UW[0x64] = 0`, `UW[0x34] = 0`. `CpuSetup`: offset 0, prefix 1, core voltage 0, exactly what the copy produces | observed (image and `/sys/firmware/efi/efivars/UniWillVariable-9f33f85c-…`, identical) |
| So a boot-time write of an undervolt into `CpuSetup[0x13B/0x13D]` is overwritten in DXE of the next boot. PEI of that boot already read the new value, so it applies for **at most one boot** | inferred (PEI reads `CpuSetup` before DXE runs; the PEI module that turns `0x13B` into the OC-mailbox policy was not located) |
| `UniWillVariable` (`9F33F85C-13CA-4FD1-9C4A-96217722C593`, 180 bytes, attributes `0x07`: non-volatile, boot service **and runtime**) is visible and readable from Linux. It is the OEM's own store for these values (written by `OemServiceSmm`, `OemUniWillVariableDxe`, `OemPowerModeDxe`, `Setup`) | observed (attributes); "the OEM's store for the Windows control centre" is inferred |
| `0x59C` (EC `0x0741` bit 7 set): writes `CpuSetup[0x135]` (OverClocking Feature) = 0 and `UW[0x5C]` = 1, then clears bit 7 in the EC. `OemOcPei` (SHA-256 `45dfa49f…9cdf`) checks the same bit in PEI and, when `CpuSetup[0x135] == 1`, clears overclocking fields of that boot's policy | observed (REA `ev_0335df3aac0bafe95daab4ee2d0b8a2c4ebc1f5e14bc72a497618b348e8a2e1b`; `OemOcPei` by `objdump -m i386` over the TE image) |
| EC `0x0741` reads `0x81` (bit 7 set) at runtime now and in all four committed dumps, yet `CpuSetup[0x135]` stayed 1 with no NVAR write in 8 boots. So bit 7 was clear when DXE ran on each of those boots | observed (`/sys/kernel/debug/regmap/INOU0000:00/registers`, `evidence/ec/*.bin`) / inferred |
| What sets bit 7 (and what clears it before the next boot) | **unknown**: EC firmware |
| The setup-screen action "GCU - OverClock Recovery" ("Are you sure to reset GCU OverClock defaults?") sets `UW[0x5C] = 1`. It is in the store-2 history (`0x12506AA`, `UW[0x5C]` 0 → 1) | observed (`Setup` driver `0x1B16`–`0x1B9D`, strings at `0x26720`/`0x26790`); who consumes `UW[0x5C]` is unknown |

### 3. Overclocking recovery and the `OcSetup` mirror (`DxeOverClock`)

`DxeOverClock` (FFS `B9B13798-D409-4D58-86E5-B1836CBEF387`, SHA-256 `7c1ca7bf…43cd`). It does nothing when
`CpuSetup[0x135]` (OverClocking Feature) is 0. On this machine it is 1, so it runs on every boot.

| Finding | Status |
|---|---|
| `OcSetup` (`3524AF07-4548-7374-C9CB-A392F225125D`, 117 bytes, attributes `0x07`, **runtime**) mirrors about 60 overclocking fields of `CpuSetup`/`SaSetup`/`Setup`. Function `0x1358(1)` copies them into `OcSetup`; `0x1358(0)` copies them back. The `CpuSetup` fields covered include `0x135`, `0xDF` (Overclocking Lock → `OcSetup[0x59]`), `0x1B`, `0x17` (PL1 → `[0x48]`), `0x1C`, `0x1D`, `0x1E` (PL2 → `[0x51]`), `0x30`, `0x13B`/`0x13D`/`0x13E` and the turbo ratios `0xD6`–`0xDD` | observed (REA `ev_4f4ca388998e233169a3114300a39c1da34707be1c932db9a2b77b86ca4babe4`). The decode matches the live `OcSetup`: lock 1, PL1 0, cTDP 120000 mW, ratios 45 45 44 43 42 41 41 41 |
| At the end of every boot `DxeOverClock` refreshes `OcSetup` from the setup variables (`0x775`–`0x7A1`), so `OcSetup` follows `CpuSetup` and never overwrites it on a normal boot | observed (REA `ev_f5e693d1a662372069bc51ad545d66745ac788ec9035580eed58a46b11d9ceaa`) |
| If `OcSetup[0] == 1` at boot, `DxeOverClock` copies `OcSetup` **into** `CpuSetup`, `SaSetup` and `Setup`, clears the flag, arms the watchdog with a known reset and resets the platform (`0x58C`–`0x6B0`). `OcSetup[0]` is 0 now | observed |
| Watchdog check (`0x70E`): `WDT_PROTOCOL.CheckStatus` returns FAILURE (1) after a watchdog expiry or an unexpected reset during a covered boot ([edk2-platforms `Wdt.h`](https://github.com/tianocore/edk2-platforms/blob/4db2845d14425599238be86b8a73d288f7c0b4a6/Silicon/Intel/AlderlakeSiliconPkg/Include/Protocol/Wdt.h); FAILURE = 1, OK = 0 in [`PchRegsPmc.h`](https://github.com/tianocore/edk2-platforms/blob/4db2845d14425599238be86b8a73d288f7c0b4a6/Silicon/Intel/SimicsIch10Pkg/Include/Register/PchRegsPmc.h)). On FAILURE, or when `Setup[0x753] == 1`, it calls `0xF8C` | observed (code); the FAILURE/OK values come from Intel reference code for other chipsets, so they are inferred for this one |
| `0xF8C` restores the overclocking fields from copies kept elsewhere in `CpuSetup`: Overclocking Lock `0xDF` ← `0xDE` (0 now), PL1 `0x17` ← `0x124`, PL2 `0x1E` ← `0x128`, PL1 override `0x1B` ← 0, ratios, voltages `0x13B` ← `0x150`…, plus memory fields in `SaSetup`. It then writes `SaSetup` and `CpuSetup` | observed (REA `ev_0a9d7d60487dcb83cf1ab581d1a7fb11889436003359b4d17093f662dd05a6f8`) |
| When `Setup[0x754] == 0` (factory default 0, live 1), the one-time initialisation `0xABC` (run from the protocol notify `0x7BC`) sets `Setup[0x753..0x754] = 00 01`, reads MSR `0xCE` and the power limits, writes the saved copies, and sets **`CpuSetup[0xDF]` (Overclocking Lock) = 0** and `[0xDE]` = 0 | observed (REA `ev_82ef59f9ebab43a2b44ceda1ff2585aebdf1b9ff952d527da81e14666827584a`; disassembly `0xACE`–`0xCEE`) |
| Live `CpuSetup[0xDF]` = 1 although `Setup[0x754]` = 1. So something set the lock again after that first initialisation, most likely the factory setup step. No module was found that writes `0xDF` on a normal boot: only `DxeOverClock` writes it, and only in `0xABC`, `0xF8C` and `0x1358(0)` | observed (store `0xDF` instructions searched in every candidate DXE module) / inferred (cause) |
| Hence, untested: **Load Defaults in the setup screen** (`Setup[0x754]` → 0, `CpuSetup[0xDF]` → 1) followed by two boots would leave the Overclocking Lock **off**. This relies on the setup browser loading whole variables from `StdDefaults` | inferred, **not verified**; a test for the owner (see "Open questions") |
| `OcSetup` is runtime-writable by attributes. An OS-side write with `OcSetup[0] = 1` would make the next boot copy its fields into `CpuSetup`. Whether the SMM variable service accepts such a write from the OS is not known (`OverClockSmiHandler` also names `OcSetup`) | observed (attributes) / **unknown** (write acceptance; not tried, since writing is out of bounds) |

### 4. Defaults, NVRAM resets and power-off

| Finding | Status |
|---|---|
| The NVRAM is two NVAR stores in the BIOS region of the SPI flash (`0x1200090`–`0x1230000` and `0x1230090`–`0x1260000`). Garbage collection copies current values into the other store, so a written value survives it (store 2 carried `CpuSetup`, `OcSetup` and the others over) | observed |
| A full power-off does not touch the flash, so stored values persist unless firmware resets the NVRAM | inferred |
| `StdDefaults` (inside the NVAR store) holds factory defaults for 20 variables (`Setup`, `CpuSetup`, `SaSetup`, `PchSetup`, …). `NvramDxe` also knows `MfgDefaults` | observed |
| What makes the AMI NVRAM driver reset to defaults (a CMOS or RTC power loss check, a jumper, manufacturing mode). No such check was found in `NvramPei`, `CmosPei` or the PMC `GEN_PMCON` addresses, and the hook is probably in another module (`SbPei`) | **unknown** |
| Whether this board has an RTC coin cell, which decides whether "full power-off" can mean RTC power loss | **unknown**: look under the cover (#19) |

### 5. What each route reaches

| Route | Reaches `CpuSetup`? | Survives reboot / power-off? | Status |
|---|---|---|---|
| GRUB `setup_var`, `setup_var.efi`, RU.efi (boot services, before `ExitBootServices`) | yes: `CpuSetup` is boot-service, non-authenticated, not locked | yes for the "kept" fields; **no** for `0x13B`/`0x13D`/`0x13E` (one boot); lost on Load Defaults, a watchdog recovery, an EC `0x0741` bit 7 event, or an NVRAM reset | inferred (code and NVAR history); not tried on the device |
| From Linux directly | no: `CpuSetup`, `Setup` and `SaSetup` are absent from efivarfs (boot-service only) | n/a | observed |
| `UniWillVariable` (runtime) | the voltage fields, through `OemOcDxe`, on the next boot | yes for the voltage fields (it is their source) | inferred; writing it is an EFI-variable write: owner's decision |
| `OcSetup` with `[0] = 1` (runtime) | about 60 overclocking fields including the lock, on the next boot, followed by a reset | yes, except that the voltage fields are then overwritten again from `UniWillVariable` | inferred; write acceptance unknown; owner's decision |
| Modified image with new `StdDefaults` (and matching form defaults) | only after Load Defaults or an NVRAM reset | yes, and it is the only route that also survives a defaults reload | inferred |
| Modified image with a changed current `CpuSetup` entry in the NVAR store | yes, the same as `setup_var` but written by the programmer | same as `setup_var` | inferred |
| Modified image patching `OemOcDxe` (`0x860`–`0x8D7`) | removes the per-boot copy of the voltage fields | yes | inferred; a code change, more risk |
| Runtime MSR `0x150` undervolt from the daemon, after the Overclocking Lock is off | bypasses the BIOS voltage fields | has to be reapplied every boot by the daemon, which is the design of #22 anyway | inferred |

## Open questions

- **Unknown:** what sets EC `0x0741` bit 7 at runtime and what clears it before DXE. This waits on the EC firmware (map
  fog "EC firmware analysis").
- **Unknown:** what triggers an AMI NVRAM reset to defaults (RTC power loss or other), and whether the board has an RTC
  cell.
- **Unknown:** which PEI module turns `CpuSetup[0x13B]` into the overclocking-mailbox voltage. If that happens after
  `OemOcDxe`, the boot-time undervolt never applies at all, not even for one boot.
- **Unknown:** whether OS writes to `OcSetup` and `UniWillVariable` are accepted, and what `UW[0x5C]` means to its
  reader.
- **Not verified:** "Load Defaults then two boots" leaves the Overclocking Lock off (section 3).

## Decisions for the owner

1. **Undervolt route.** Unlock once at boot time (Overclocking Lock `0xDF` = 0, which persists) and apply the offset
   at runtime through MSR `0x150` per Power profile (#22)? Or keep a BIOS-level offset, which means writing
   `UniWillVariable` (runtime EFI variable) or patching `OemOcDxe` in a modified image?
2. **Unlock route** for the Overclocking Lock: a boot-time tool (GRUB `setup_var`/UEFI shell, writing `CpuSetup[0xDF]`),
   the untested Load Defaults path (it also resets every other BIOS setting, including Wake on LAN and the boot order),
   or the `OcSetup` apply flag from Linux? Each one is a write to an EFI variable not yet written on this board.
3. **A persistence test on the device**, done by the owner. Change one "kept" field, reboot twice, power off fully (and,
   separately, remove all power if there is no RTC cell), and read the result back each time without writing: MSR
   `0x194` bit 20 for the lock, or a re-dump of the NVAR store. Rollback: write the original value back, or Load
   Defaults.

## Commands and tools

- Image unpacked with `uefiextract image.bin all` (UEFITool NE) into a scratch copy. Modules located by GUID and name:
  `CpuSetup` GUID bytes `ff978fb0e8e69341a9975e9e9b0adb32` and UTF-16 `CpuSetup`, `UniWillVariable`, `OcSetup`.
- NVAR stores parsed with `bios/nvar.py` (`find_stores`, `parse_store`, `current_values`, and the update chains for the
  timeline). Forms decoded with `bios/analyze.py`'s `ifr_text` (ifrextractor).
- DXE modules: REA (Hopper provider) `decompile_all` and `analyze_function` (Evidence ids above;
  `decompile_all` OemOcDxe `ev_75f893a6b74e2478a0d1fd91472167fc973d289ad4611ba3d6fd5d44aaa0dc9b`, DxeOverClock
  `ev_d31cb0e87c86d2dfbbf32346f380e0c0261131f0ba6b5c5255d646c80ec7acab`), with instruction-level checks by
  `objdump -d -M intel`. The Hopper pseudocode drops stores into local buffers, so every field mapping above was read
  from the disassembly.
- PEI modules (TE, IA32): `objdump -D -b binary -m i386 --adjust-vma=<StrippedSize-0x28>`.
- Runtime, read-only: `od -tx1 /sys/firmware/efi/efivars/{UniWillVariable-9f33f85c-13ca-4fd1-9c4a-96217722c593,OcSetup-3524af07-4548-7374-c9cb-a392f225125d}`
  (both identical to the image, attributes `07000000`); `ls /sys/firmware/efi/efivars` has no `CpuSetup`/`Setup`/`SaSetup`;
  `sudo cat /sys/kernel/debug/regmap/INOU0000:00/registers` → `741: 81`.
- Live values from the active store: `CpuSetup` `0xDF`=1, `0xDE`=0, `0x135`=1, `0x17`=0, `0x124`=120000, `0x13B`=0,
  `0x13D`=1, `0x13E`=0; `Setup` `0x753`=0, `0x754`=1 (default 0); `OcSetup[0]`=0; `UniWillVariable` `0x5C`=1, `0x5D`=1,
  `0x64`=0, `0x66`=1.
