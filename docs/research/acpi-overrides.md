# What can ACPI table overrides change without a flash? (#32)

Research for map #23 (BIOS and firmware deep dive). Read only: nothing on this laptop was written, and no table was
loaded. Each finding is marked **observed** (read on this machine, in its tables or its BIOS image, or in the kernel
source for this kernel's version), **inferred** (reasoned from those) or **unknown**.

## Answer

Overrides work on this kernel: `CONFIG_ACPI_TABLE_UPGRADE=y`, lockdown is `none`, and the boot chain (mkinitcpio
building a Limine UKI) already carries an early uncompressed cpio, which is where override tables go. But for the five
behaviours in the ticket, **an override is either useless or not needed**:

| Behaviour | What an override could do | Worth it? |
|---|---|---|
| **Power table** (PL1/PL2/PL4, thermal offset, TGP offset, Dynamic Boost buffers in `\_SB.INOU`) | Change bytes that nothing in the firmware reads. The buffers are only returned by the vendor method `SMRW`, which no AML calls. They are a lookup table for the vendor's Windows app, which then applies values through `T1WR`. The driver already applies them directly (TGP offset through `T1WR`, CPU limits through powercap). | **No**: no effect. It would also need a full DSDT replacement (high risk, below). |
| **Thermal trip points** (`ECTZ` critical 100 °C; `TZ00` critical 119 °C, active 71/55/50/45/40 °C) | Move or add trips. These only drive Linux's own reaction (shutdown at critical, the ACPI fan power resources). The CPU's own throttling (TCC, 95 °C) and the EC's fan control do not read them. | **No**: `thermal.crt=`, `thermal.act=`, `thermal.psv=` and `thermal.off=` do the same from the command line. |
| **Sleep states** | Edit `_S3` or the FADT's Low Power S0 flag. | **No**: S3 is already offered (`mem_sleep`: `[s2idle] deep`). `mem_sleep_default=deep` chooses it. |
| **`_OSC` and ASPM** | Grant the OS PCIe control, or clear the FADT "ASPM not supported" flag. | **No**: Linux already has full control (`_OSC` grants PCIeCapability and LTR, and the FADT flag is 0). ASPM is set at runtime through sysfs or `pcie_aspm.policy`. |
| **EC query handlers** (`_Qxx`, 53 in the DSDT) | Rewrite a handler. An existing method cannot be redefined from an added SSDT, so this means replacing the whole DSDT. One real bug was found: `_Q14`/`_Q15` (brightness keys) abort on a missing `\_SB.PC00.PEG1.PEGP.EDP1` (45 errors this boot). An **additive** SSDT that declares the missing device would silence it without replacing the DSDT. | **Only the additive SSDT, and only as a fix for the log noise** (owner decision). |

**The main risk** (observed): the firmware **patches the DSDT and SSDTs at every boot**. Comparing the DSDT stored in
the BIOS image with the one the kernel runs shows 17 differences: the addresses of every NVS operation region (GNVS
`0x736D8000`, the Uniwill `UWNV` region, and others), TPM presence, PCIe root port `_ADR` values, and the
`_PSV`/`_PSL`/`_TC1`/`_TC2`/`_TSP` passive-cooling methods, which are renamed away in `TZ00`. A replacement
DSDT freezes one boot's values. If a later boot places NVS elsewhere (after a setup change, a memory change or a BIOS
update), the override's AML reads and writes the wrong physical memory (inferred). Additive SSDTs do not freeze
anything, so they do not have this problem.

**What it costs to maintain**: a drop-in that adds the `acpi_override` hook, the `.aml` files in
`/etc/initcpio/acpi_override/`, and a UKI rebuild with `limine-mkinitcpio`, all installed by `scripts/install.sh`.
An additive SSDT only needs checking when the BIOS changes. A DSDT replacement would need regenerating from the
original (not the overridden) table after every BIOS or setup change. That is not worth it here.

## Evidence

### 1. The mechanism on this kernel and boot chain

| Finding | Status | Source |
|---|---|---|
| Kernel `7.2.15-1-omarchy`: `CONFIG_ACPI_TABLE_UPGRADE=y`, `CONFIG_ACPI_CONFIGFS=m`, `CONFIG_EFI_CUSTOM_SSDT_OVERLAYS=y`, `CONFIG_ACPI_CUSTOM_DSDT` not set, `CONFIG_LOCK_DOWN_KERNEL_FORCE_NONE=y`, `CONFIG_LOCK_DOWN_IN_EFI_SECURE_BOOT` not set | observed | `zcat /proc/config.gz \| grep -E 'ACPI_TABLE_UPGRADE\|ACPI_CONFIGFS\|...'` |
| `/sys/kernel/security/lockdown` = `[none] integrity confidentiality`; Secure Boot disabled (`bootctl status`) | observed | commands named |
| No override is active now: `/proc/sys/kernel/tainted` = 12352 (bits 6, 12, 13: U, O, E), not bit 8 (A, "ACPI table overridden") | observed | `cat /proc/sys/kernel/tainted`; [tainted-kernels.rst](https://github.com/torvalds/linux/blob/v7.2/Documentation/admin-guide/tainted-kernels.rst) |
| The initrd scan reads `kernel/firmware/acpi/*` from the initrd. It accepts the signatures in `table_sigs[]`, which include DSDT, FADT (`FACP`), SSDT, DMAR, HPET, MCFG, NHLT and UEFI, but not RSDP or FACS. It rejects a table whose length or checksum is wrong, and it ignores all overrides when the kernel is locked down | observed | [drivers/acpi/tables.c](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/tables.c) lines 398-492 (`table_sigs`, `acpi_table_upgrade`) |
| A file **replaces** a firmware table only if signature, OEM ID and OEM table ID match **and** its OEM revision is higher. Otherwise it is **installed as an extra table** | observed | same file, `acpi_table_initrd_override` (lines 546-602) and `acpi_table_initrd_scan` (604-650); [initrd_table_override.rst](https://github.com/torvalds/linux/blob/v7.2/Documentation/admin-guide/acpi/initrd_table_override.rst) |
| In 7.2 only the built-in `CONFIG_ACPI_CUSTOM_DSDT` path calls `acpi_table_taint()`. The initrd (physical) path logs "Table Upgrade: override" but adds no taint | observed in source, not tried | tables.c lines 376-381, 681-699 |
| Dynamically loaded SSDTs (the `PmRef` CPU power tables `Cpu0Ist`, `ApIst`, ... loaded at runtime here) go through the same override hook: the AML `Load` path calls `acpi_tb_install_and_load_table(..., override = TRUE, ...)`, and `acpi_tables_addr` is not `__initdata` | observed in source | [exconfig.c](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/acpica/exconfig.c) line 458, [tbinstal.c](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/acpica/tbinstal.c) `acpi_tb_override_table`, tables.c line 384 |
| Other routes: **configfs** (`/sys/kernel/config/acpi/table/<name>/aml`, SSDT only) loads at runtime, and `rmdir` unloads it (`acpi_unload_table`). **`efivar_ssdt=`** loads an SSDT from an EFI variable | observed in source | [acpi_configfs.c](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/acpi_configfs.c) lines 26-60, 221-229; [ssdt-overlays.rst](https://github.com/torvalds/linux/blob/v7.2/Documentation/admin-guide/acpi/ssdt-overlays.rst); [efi.c](https://github.com/torvalds/linux/blob/v7.2/drivers/firmware/efi/efi.c) lines 253-349 |
| `efivar_ssdt` needs an EFI variable written, which the hard rules of this map forbid. It is an owner decision and not needed. configfs is the safe way to **test** an additive SSDT (load, check, `rmdir`) with no reboot and no initrd change | inferred | |
| Boot chain: Limine 12.9.4 boots a UKI (`/boot/EFI/Linux/omarchy_linux-omarchy.efi`). Its `.initrd` section starts with an uncompressed `070701` cpio holding `kernel/x86/microcode/GenuineIntel.bin`, which is the same early-cpio slot the override uses | observed | `objcopy -O binary --only-section=.initrd` on a copy, then `cpio -t` (copy deleted afterwards) |
| mkinitcpio ships the `acpi_override` hook: it adds `/usr/lib/initcpio/acpi_override/*.aml` and `/etc/initcpio/acpi_override/*.aml` with `add_file_early` to `/kernel/firmware/acpi/` | observed | `/usr/lib/initcpio/install/acpi_override` |
| Omarchy's `/etc/mkinitcpio.conf.d/00-omarchy-hooks.conf` **resets** `HOOKS`, so an override drop-in must sort after it and append (`HOOKS+=(acpi_override)`). `/usr/local/bin/mkinitcpio` warns that only `limine-mkinitcpio`/`limine-update` refresh the Limine entry, whose path carries a hash (`hash_mismatch_panic: no`) | observed | the files named, `/boot/limine.conf` |
| Rollback for a bad override: pick the Limine snapshot entry (`//Snapshots`, which keeps an older UKI copy), or remove the `.aml`, rebuild, and reboot | inferred | `/boot/limine.conf` |
| Measured boot: the UKI is measured, so an override changes the PCR values. Nothing here is sealed to them: no `tpm2` in `/etc/crypttab*`, and the root disk is opened by passphrase through `cryptdevice=` | observed (configuration); that nothing else is sealed is inferred | `bootctl status`, `grep tpm2 /etc/crypttab*` |

### 2. The firmware rewrites its tables at boot

The BIOS region (`0x1200000`-`0x1FFFFFF`) of `evidence/bios/bios-N.1.05AVE04-20261115.bin` was extracted with
REA `extract_firmware` (Unblob 26.6.4) into `/tmp`. **Evidence `ev_dad0890c6c446811deeaf53ee1512359cce94058db4575c0d87411b18b12c41e`**.
In the decompressed DXE volume (`selected.bin_extract/3944752-7054044.lzma_extract/lzma.uncompressed`, SHA-256
`0932977000eb86d47153173987242238359bb5f2cb1ffb46ac657e6cda399420`), a DSDT with the running table's length (`0x5462D`)
sits at offset `0xF366BC`. Its SHA-256 is `de4c0fe8...7620f9`, while the running table, `evidence/acpi/DSDT.dat`
(identical to `/sys/firmware/acpi/tables/DSDT` this boot), is `bfa49b31...f97daa`. 79 bytes differ. Disassembled with
`iasl -d` and diffed (observed):

| Object | In the image | Running |
|---|---|---|
| `OperationRegion (GNVS, ...)` | `0xFFFF0000, 0xAA55` (placeholder) | `0x736D8000, 0x0AB6` |
| `UWNV` (Uniwill NVS region under `INOU`'s scope), `UWSV`, `OTPM` | `0xFFFF0000, 0xAA55AA55`, `0xFF`, `0xFF` | `0x73713018, 0x00070000`, `0x02`, `0x02` |
| `SANB/SANL`, `CNVB/CNVL`, `OGAD/OGSZ`, `ADAR/ADSZ`, `SNVB/SNVL`, `PNVB/PNVL` (NVS bases and sizes) | `0xFFFF0000/0xAA55` | real addresses (`0x737E4C98`, `0x7356B000`, ...) |
| TPM: `FTPM`, `PPIM`, `PPIL`, `TPMF`, `PPIV`, `DTP1`, `TTPF`, `TTDP` | defaults | this board's TPM values |
| Two PCIe root ports' `_ADR` | `0x001C0000` / `0x001C0004` | swapped |
| `IOST`, `TOPM`, `SS1`, `TMRQ`, touchpad `SADR`/`HADR` | defaults | patched |

SSDTs in the same volume against the running ones: `SSDT1` (CpuSsdt) 61 bytes, `SSDT4` (TglH_Rvp) 7,
`SSDT5` (Ther_Rvp) 12, `SSDT7` (UsbCTabl) 7, `SSDT10` (Opt2Tabl) 5 (`TPNV` region `0xFFFF0000` → `0x736E0000`),
and the rest 0 (observed, `cmp -l` on each carve). In `SSDT5`, the firmware **renames `TZ00`'s `_PSL`, `_PSV`, `_TC1`,
`_TC2`, `_TSP` to `XPSL`, `XPSV`, `XTC1`, `XTC2`, `XTSP`**. So passive ACPI cooling is switched off at boot, which is why
Linux registers `TZ00` with no passive trip (observed). Which DXE driver does it (`OemACPIDriverDxe`,
`AcpiPlatformFeatures`, ...) is unknown.

Consequences (inferred): a DSDT replacement has to start from the **running** table, not the image's, and it fixes
those addresses for good. Whether they stay the same from boot to boot is **unknown**: the journal holds one boot, and
the evidence tables come from that same boot. The check is read-only: after the next reboot, compare
`sudo sha256sum /sys/firmware/acpi/tables/DSDT` with `evidence/acpi/DSDT.dat`. Even if they match, a BIOS setup change
(the boot-time setting changes this map plans), a memory change or a TPM toggle can move them. Once an override is
active, `/sys/firmware/acpi/tables/DSDT` shows the override, so the original has to come from somewhere else for a
regeneration.

### 3. The power table

| Finding | Status | Source |
|---|---|---|
| `\_SB.INOU` (`INOU0000`, the device the driver binds to) holds 12-byte buffers `ACPB`...`A8SH` laid out as PL1, PL2, PL4, thermal offset, TGP offset, Dynamic Boost, then `57 3C 2D` and `FF FF FF` | observed | DSDT.dsl lines 69999-70096 |
| The only code that touches them is `INOU.SMRW(Arg0)`. `Arg0[0] = 0xBB` returns a DWORD at offset `Arg0[1]`, and `0xAA` **overwrites the whole buffer** with `Arg0[1..12]`. The buffer is chosen by EC `GFID` (`0x7D3` bits 4-7), `PDIN` (`0x74C` bits 0-3) and, for some groups, `ADPT` (`0x49F` bits 3-5) | observed | DSDT.dsl lines 70446-70700; field map lines 70781-70834 |
| No AML calls `SMRW` or `T1WR` (`rg` over all `.dsl`), and the driver calls neither `SMRW` nor any table read | observed | `rg -n 'SMRW\|T1WR' evidence/acpi/*.dsl driver/src/*.c` |
| So the buffers are inert data for an OS-side reader (the vendor's Windows app), which applies them itself through `T1WR` (`0x81`/`0x82`/`0x84` → EC `APL1/2/4` at `0x783`-`0x785`, `0x1171` → TGP offset, `0x1172`/`0x1173` → Dynamic Boost) | inferred | DSDT.dsl lines 70317-70428 |
| With the EC dumps' values (`GFID` = 9, `PDIN` = 3, `ADPT` = 3; same in all four dumps), the lookup picks **`A8SG`** (line 70072), not `A8PF`. Its bytes match those in INVESTIGATION 7.2 (80/80/165 W, 5, +20 W, +15 W). This refines 7.2 without changing any number | observed | `od -An -tx1 -j 0x7D3 -N1 evidence/ec/ec_a.bin` → `90`; `0x74C` → `33`; `0x49F` → `1a` |
| Overriding the buffers would change nothing the firmware does, and `SMRW 0xAA` can rewrite them at runtime anyway (lost at power-off). Each value they describe already has a direct route in the stack | inferred | |
| What bytes 6-8 (`0x57` = 87, `0x3C` = 60, `0x2D` = 45) mean | unknown | 60 matches the graphics card's 60 W default limit (INVESTIGATION 7.2), but that is only a guess |

### 4. Thermal trip points

| Finding | Status | Source |
|---|---|---|
| `thermal_zone0` = `ECTZ` (DSDT): `_CRT` fixed at `0x0E94` (100 °C), temperature from EC `XHPP`. `thermal_zone1` = `TZ00` (SSDT5): critical 119 °C (GNVS `CRTT`), active 71/55/50/45/40 °C (`ACTT`/`ACT1`) on `FAN0`-`FAN4`, no passive trip | observed | `/sys/class/thermal/thermal_zone*/trip_point_*`; DSDT.dsl 71696-71720; SSDT5.dsl 396-510 |
| `TZ00`'s active trips switch the power resources `FN00`-`FN04`, whose `_ON`/`_OFF` call `FNCL`. `FNCL` writes EC `FPMS` (`0x7CF`) = `0xC8`/`0x96`/0, but only while EC `S0IF` = 1. `FPMS` reads 0 in all four dumps. Three of the five ACPI fan cooling devices show `cur_state` 1 now (which device is which `FANn` was not mapped) | observed | SSDT5.dsl 82-391; `/sys/class/thermal/cooling_device*`; `evidence/ec` |
| These trips are only Linux's policy: shutdown at critical, and the AML above. The CPU throttles itself at TjMax − 5 = 95 °C (TCC, INVESTIGATION 12), and the EC runs the fans itself | inferred | |
| `thermal.crt=<°C>` sets **every** critical trip to that value (also above the firmware's, despite the "lower" in its parameter description), `-1` disables them. `thermal.act=`, `thermal.psv=` and `thermal.off=1` do the same for the other trips | observed in source | [thermal.c](https://github.com/torvalds/linux/blob/v7.2/drivers/acpi/thermal.c) lines 66-84, 347-375, 428-443; [kernel-parameters.txt](https://github.com/torvalds/linux/blob/v7.2/Documentation/admin-guide/kernel-parameters.txt) `thermal.*` |
| A kernel cooling device `TCC Offset` (`cooling_device18`, `cur_state` 5 of 63) exposes the CPU's throttle offset at runtime. Writing it is an MSR write, which is outside this research and an owner decision | observed (it exists); its effect is not tried | `/sys/class/thermal/cooling_device18/type` |

### 5. Sleep states

| Finding | Status | Source |
|---|---|---|
| `_S0`, `_S3`, `_S4`, `_S5` exist; the kernel reports `ACPI: PM: (supports S0 S3 S4 S5)`; FADT "Low Power S0 Idle" = 1; `/sys/power/mem_sleep` = `[s2idle] deep` | observed | DSDT.dsl 5878-5905; `journalctl -k -b`; `iasl -d FACP.dat` |
| So S3 needs no override: `mem_sleep_default=deep` or `/sys/power/mem_sleep` selects it. Whether S3 resumes cleanly on this board (EC, NVIDIA, NVMe) is **unknown** and needs an owner-run test, since a failed resume may need a forced power-off | observed (option); unknown (behaviour) | [kernel-parameters.txt](https://github.com/torvalds/linux/blob/v7.2/Documentation/admin-guide/kernel-parameters.txt) `mem_sleep_default=` |
| `ACPI: button: [Firmware Bug]: Unexpected lid state reported by firmware` comes from `LID1` defaulting `LIDS` to 1. `button.lid_init_state=` handles it without an override if it ever matters | observed (message); inferred (remedy) | `journalctl -k -b`; DSDT.dsl 71725-71745 |

### 6. `_OSC` and ASPM

| Finding | Status | Source |
|---|---|---|
| `\_SB` platform `_OSC`: support mask = control mask = `006e7eff` (everything asked for is granted) | observed | `journalctl -k -b` |
| PCI host `_OSC`: "OS supports [ExtendedConfig ASPM ClockPM Segments MSI EDR HPX-Type3]", "OS now controls [PCIeHotplug SHPCHotplug PME AER PCIeCapability LTR DPC]" | observed | same |
| FADT "PCIe ASPM Not Supported" = 0; `pcie_aspm` policy `[default]` | observed | `iasl -d FACP.dat`; `/sys/module/pcie_aspm/parameters/policy` |
| So the kernel already owns ASPM, and per-link control exists (`/sys/bus/pci/devices/*/link/*`, `pcie_aspm.policy=`). An override adds nothing | inferred | |
| Not an ACPI matter, but found on the way: at 9065 s this boot the NVMe controller dropped off the bus (`controller is down; will reset: CSTS=0xffffffff`), 30 s after a correctable `RxErr` on root port `00:1d.0`. It came back. Since then `lspci` shows ASPM and L1 substates **off on the SSD's side** of the link while its `link/*` files read 1, which may explain INVESTIGATION 18's mismatch | observed | `journalctl -k -b -o short-monotonic`, `lspci -vvv -s 03:00.0` |

### 7. EC query handlers

| Finding | Status | Source |
|---|---|---|
| 53 `_Qxx` methods under `\_SB.PC00.LPCB.EC0` in the DSDT, none in SSDTs | observed | `rg -o 'Method \(_Q..' DSDT.dsl` |
| `_Q14`/`_Q15` (brightness up/down) send `Notify (GFX0.DD1F, 0x86/0x87)`, then `Notify (^^^PEG1.PEGP.EDP1, ...)`. `\_SB.PC00.PEG1.PEGP` is the RTX 3050 (`01:00.0`'s `firmware_node/path`), but `EDP1` exists only as an `External`, so the method aborts with `AE_NOT_FOUND`: 45 times this boot | observed | DSDT.dsl lines 97, 71216-71228; `journalctl -k -b \| grep -cF PEG1.PEGP.EDP1` |
| The iGPU notify runs before the abort, so the brightness keys still work, and the error is log noise | inferred | |
| Fix without a DSDT replacement: an SSDT that declares `Device (\_SB.PC00.PEG1.PEGP.EDP1)` with no `_BCL`, so the second `Notify` finds a target. It can be tested through configfs and removed with `rmdir`. What `acpi_video` does with the new child (an extra output, no backlight device expected) is **unknown** until tried | inferred | |
| Changing what any `_Qxx` does requires replacing the DSDT. An added SSDT cannot redefine a name that already exists: the load reports `AE_ALREADY_EXISTS` for that object | inferred (ACPICA namespace rules; not tried) | |
| The handlers the stack cares about (`_Q83` refreshes the graphics card's TGP offset from `CTWA`) are already reproduced by the driver's `T1WR` call (INVESTIGATION 9.1) | observed | DSDT.dsl `_Q83`; `driver/src/uniwill-acpi.c` `T1WR` |

### 8. Maintenance beside the driver (if the owner adopts the additive SSDT)

All inferred from the files in section 1:

- Source `driver/acpi/edp1-stub.asl` (or similar), compiled by `iasl` at install time with a unique OEM table ID, so it
  is always *installed*, never matched as an *override*.
- `scripts/install.sh` copies the `.aml` to `/etc/initcpio/acpi_override/`, adds
  `/etc/mkinitcpio.conf.d/90-avell-acpi-override.conf` with `HOOKS+=(acpi_override)`, and runs `limine-mkinitcpio`.
  Uninstall reverses all three.
- Re-check after any BIOS change that the `External` paths still exist (`\_SB.PC00.PEG1.PEGP`). Nothing else
  depends on per-boot values.
- A DSDT replacement would add regeneration after every BIOS or setup change, keeping a copy of the original
  table outside the override, and re-verifying the driver's `INOU` methods (`ECRR`, `ECRW`, `T1WR`) in it. Not recommended.

## Unknowns

- Whether the boot-patched NVS addresses (GNVS `0x736D8000`, `UWNV` `0x73713018`, ...) are the same on every boot.
  Read-only check after the next reboot: `sudo sha256sum /sys/firmware/acpi/tables/DSDT` against `evidence/acpi/DSDT.dat`.
- Which DXE driver renames `TZ00`'s passive methods, and why.
- What power-table bytes 6-8 (`57 3C 2D`) mean.
- Whether S3 (`deep`) resumes cleanly on this board (an owner-run test).
- What `acpi_video` does with an `EDP1` stub (a configfs test).

## Commands and tools

- On the device (read only): `zcat /proc/config.gz`, `cat /sys/kernel/security/lockdown /proc/sys/kernel/tainted /sys/power/mem_sleep`,
  `bootctl status`, `sudo cat /boot/limine.conf`, `journalctl -k -b`, `/sys/class/thermal/*`, `lspci -vvv`,
  `sha256sum` of `/sys/firmware/acpi/tables/*` against `evidence/acpi/` (DSDT, FACP, SSDT1, SSDT5, SSDT10 identical).
- Image: REA `extract_firmware` (Evidence above), `grep -boaP 'DSDT\x2d\x46\x05\x00'`, `dd`, `cmp -l`, `iasl -d` (20251212).
- Kernel source: `torvalds/linux` tag `v7.2` (`drivers/acpi/tables.c`, `acpi_configfs.c`, `thermal.c`, `acpica/tbinstal.c`,
  `acpica/exconfig.c`, `drivers/firmware/efi/efi.c`, the admin-guide documents). `tables.c` at `v7.2` has the same size
  (21731 bytes) as at stable `v7.2.15`.
