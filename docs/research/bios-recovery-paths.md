# How a bad BIOS flash can be recovered on this board (#26)

Research ticket [#26](https://github.com/milenagriz5-dotcom/uniwill-avell/issues/26), map [#23](https://github.com/milenagriz5-dotcom/uniwill-avell/issues/23).
Everything here was read from the committed backup `evidence/bios/bios-N.1.05AVE04-20261115.bin`
(SHA-256 `baa08b4a…52412`) or from passive reads on the running laptop. Nothing was written to the flash, EFI variables,
the EC or any MSR. Each finding is marked **observed** (seen in this board's image or hardware), **inferred** (deduced
from observations plus a cited source), or **unknown**.

## Answer

- **AMI crisis recovery is built in, but only for the stock image, and only when the boot block survives.** The image
  carries AMI's recovery modules (`Recovery`, `FsRecovery`, `UsbPei`, `AhciRecovery`, `NvmeRecovery`) and the `ReFlash`
  recovery screen. The recovery file is named **`GMXTGFXN105.ROM`**, read from a FAT or ISO 9660 (CD) volume on USB,
  SATA or NVMe (observed). **No key combination is compiled in.** The platform recovery-hook list in `IsRecovery` is
  empty (observed). Recovery starts on its own when `RomLayoutPei` finds a protected firmware volume corrupted, or when the OS
  asks for a flash update through the `AmiFlashUpd` variable (observed in code). The BIOS region is signed (AMI Secure
  Flash, RSA-2048/SHA-256, observed), and recovery checks that signature (inferred), so **a modified image can never be
  restored or installed through recovery.**
- **What recovery rewrites:** the ReFlash screen offers *Reset NVRAM*, *Boot Block Update*, *Main Block Update* and *EC
  FIRMWARE Update*, and nothing for the ME region or the flash descriptor (observed strings). The recovery code itself
  lives in the PEI volumes at `0x1BC0000`–`0x1FFFFFF`. If any of those is damaged, recovery cannot run (inferred).
- **There is a backup copy of the boot block, but only of the top 256 KiB.** The volumes at `0x1FC0000` and
  `0x1F80000` are nearly identical: 17 bytes differ (observed). They fit Intel's Top Swap scheme. Pulling the
  `GPP_B14/SPKR` strap high at power-up makes the CPU boot the lower copy (Intel datasheet). The other PEI volumes have no
  copy (observed), so Top Swap only helps when the damage stays inside the top block. Top Swap is off now (observed). Where the strap
  is on this board is **unknown**.
- **ME recovery:** the ME region holds a recovery-capable *Bring Up* code partition in boot partition 1, and the
  operational *Main* partition in boot partition 2 (observed). If only *Main* is damaged, the ME can probably fall back to
  its recovery path (inferred, from ME 11 research). Boot partition 1 also carries the PMC and Type-C firmware. If it is
  damaged, the platform is not expected to start at all (inferred). Crisis recovery never touches the ME region.
- **No Uniwill-specific BIOS recovery mode was found.** The OEM modules cover AC power-loss behaviour (`OemACRecovery*`),
  keyboard lights, power modes and similar. The search was bounded to module names and strings.
- **The programmer is the only way back** for: a damaged boot block or PEI volume, a damaged flash descriptor, a damaged ME
  boot partition 1, any **modified image** that does not boot, and NVRAM contents that hang the machine before recovery
  or setup can run. As long as Linux still boots, `flashrom --programmer internal` can rewrite any single region from
  the backup, because the flash is unlocked (ADR-0002), so the programmer is only needed once the machine no longer
  reaches an OS.
- **A wiped model and serial number does not brick anything.** They live in a 4 KiB SMBIOS-data volume at `0x1270000`
  (observed), which can be rewritten from the backup, in Linux or with the programmer.

## Failure modes

| Failure | Does crisis recovery help? | Way back | Status |
|---|---|---|---|
| Main firmware volume (`0x15C2000`, DXE and setup) damaged, boot block intact | Yes, if the damage breaks the volume's structure: `RomLayoutPei` then enters recovery on its own. Needs a signed stock `GMXTGFXN105.ROM` | Recovery stick, or programmer | inferred from code |
| Main volume structurally valid but bad code inside (e.g. a modified image that hangs) | No: the header check passes, and a modified image fails the signature check anyway | Programmer (or flashrom, if Linux still boots) | inferred |
| PEI/boot-block volumes (`0x1BC0000`–`0x1FFFFFF`) damaged | No: the recovery code lives there | Programmer. Top Swap strap only if the damage is confined to the top 256 KiB | inferred |
| Flash descriptor (`0x0`–`0xFFF`) damaged | No | Programmer | inferred |
| ME region: Main (boot partition 2) damaged | No (recovery has no ME option) | ME may boot from its recovery path; then flashrom the ME region from Linux. Otherwise programmer | inferred |
| ME region: boot partition 1 (RBE, PMC, Bring Up) or whole region damaged | No | Programmer | inferred |
| NVRAM stores (`0x1200000`–`0x125FFFF`) corrupt | Partly: the recovery screen has *Reset NVRAM*, and the firmware keeps factory defaults (`StdDefaults`) in the boot block | Setup's load-defaults, recovery, flashrom of the NVRAM range, or programmer. Whether a CMOS/RTC clear resets NVRAM is **unknown** | inferred / unknown |
| A setting stored in NVRAM that hangs memory or CPU init (a boot-time setting change gone wrong) | No if it hangs before recovery starts | Programmer, or the NVRAM range only | inferred |
| Model and serial wiped (SMBIOS data volume `0x1270000`) | Not a boot failure | Rewrite that 4 KiB from the backup (flashrom with a layout, or programmer) | observed location |

## Evidence

### Tools and sources used

- `uefiextract` NE alpha 75 (`uefiextract img.bin all` on a copy of the backup in `/tmp`), GNU `objdump` 2.47
  (`-b binary -m i386`) on the TE/PE32 module bodies, `ifdtool` dump already in `evidence/bios/ifdtool-dump.txt`.
- REA `inspect_firmware_regions` (binwalk 3.1.0) over the backup: Evidence
  `ev_2ca9c272d223abe753b43011b42b1a49b6e4cb75a856c517afcffede098a9f88` (same image digest as #17's
  `ev_10ac21b9…`). REA `open_binary` (raw x86, load `0xffdb5b4c`) on the `IsRecovery` TE body (SHA-256
  `0457dcd6…0c5b3a`). REA's decompiler was unavailable (`provider_unavailable`: no `GHIDRA_INSTALL_DIR`), so code
  claims rest on `objdump` disassembly.
- Module bodies (SHA-256): `FsRecovery` `632c5aab…766d70`, `Recovery` `a8b49629…b7bdf3`, `ReFlash` `3bf2fd47…cd7138`,
  `IsRecovery` `0457dcd6…0c5b3a`, `IsSecRecoveryPEI` `a7efc14e…02b173`, `RomLayoutPei` `e1fda7d0…0658ef`.
- Intel, *500 Series Chipset Family PCH Datasheet Vol. 1*, doc 635218 rev 008,
  https://cdrdv2-public.intel.com/635218/635218-008.pdf (SHA-256 `fcc7ddd4…904729`), §3.2.1 *Boot Block Update
  Scheme* and Table 17 *Pin Straps*.
- coreboot `src/soc/intel/common/block/include/intelblocks/me_15.h` (commit `7e396164`) and `cse.h` (commit
  `3e1af74f`) for the CSME 15 status register layout.
- Positive Technologies, *Intel ME: The Way of the Static Analysis* (TROOPERS17),
  https://troopers.de/downloads/troopers17/TR17_ME11_Static.pdf (SHA-256 `c313ce29…aba66a3`), slide 22.
- Supermicro, *AMI BIOS Recovery* guide, https://www.supermicro.com/manuals/other/AMI_BIOS_Recovery.pdf. This is
  generic AMI behaviour on another vendor's boards. It is quoted only for contrast and is not a fact about this board.
- AMI, *AFU, a Secure Update Utility for Aptio V*, https://www.ami.com/resources/ami-firmware-utility-afu-a-secure-update-utility-for-aptio-v-uefi-bios-firmware/
  ("Aptio provides Secure Flash, which uses cryptographically signed data to ensure that only valid BIOS images … may be flashed").

### 1. Recovery modules and file name (observed)

Module placement from the UEFIExtract report (volume base, size):

| Volume | Holds | Copies |
|---|---|---|
| `0x1FC0000` (256 KiB) `289EFB98…` | PeiCore, RomLayoutPei, FvBb1Pei, StatusCodePei, NVRAM external defaults, reset vector (Volume Top File) | one more at `0x1F80000` |
| `0x1F80000` (256 KiB) `8487985B…` | same 10 files | — |
| `0x1EC0000` / `0x1E30000` | microcode (two different volumes, 0xC0000 and 0x90000) | — |
| `0x1D80000` (0xB0000) | pre-memory PEI: IsRecovery, IsSecRecoveryPEI, CapsulePei, NvramPei, OemACRecoveryPei, CrbPei, OemOcPei, FW key file | single |
| `0x1CEF000`, `0x1D7A000`, `0x1C80000` | SEC, FSP-T/M/S | single |
| `0x1BC0000` (0xC0000, compressed) | post-memory PEI: **UsbPei, AhciRecovery, NvmeRecovery, FsRecovery, Recovery**, PcdRecoveryPei, FwKeyHobPei, CryptoPPI | single |
| `0x15C2000` (0x5FE000) | FV_MAIN: DXE, setup, **ReFlash**, SecFlashUpdDXE, SecSMIFlash, SmbiosDmiEdit, FmpCapsuleUpdate | single |
| `0x1270000` (4 KiB) | SMBIOS data file `FD44820B…` (model, serials) | single |
| `0x1200000`, `0x1230000` (192 KiB each) | NVRAM stores | two |

The recovery file name is in `FsRecovery`, followed by the El Torito/ISO 9660 identifiers (offset `0x3ED0` in the PE32 body):

    003ed0 47 4d 58 54 47 46 58 4e 31 30 35 2e 52 4f 4d 00  >GMXTGFXN105.ROM.<
    003ee0 00 00 00 00 00 00 00 00 00 00 00 00 43 44 30 30  >............CD00<
    003f00 45 4c 20 54 4f 52 49 54 4f 20 53 50 45 43 49 46  >EL TORITO SPECIF<

`Recovery` carries the project tag `GMXTGFX`. The image's `$FID` file (`3FD1D3A2…`) reads `GMXTGFXN1`, version
`05.19 / 01.05`, built 2022-08-30, OEM `ALASKA A M I`. The name is project tag + BIOS version (`N105`), so a future
BIOS version would change it (inferred).

### 2. What puts the board into recovery (observed in disassembly, meaning inferred)

PEI service table offset `0x2C` is `SetBootMode`. `0x20` is `BOOT_IN_RECOVERY_MODE` and `0x12` is
`BOOT_ON_FLASH_UPDATE` (UEFI PI boot-mode values).

- **`IsRecovery`** (`0xffdb5c5b`): returns at once if the boot mode is already `0x20`. It then calls each entry of a
  recovery-hook table at `0xffdb5eb8`. **That table's first entry is 0**, so no platform hook (jumper, hotkey, EC button)
  is compiled in. Next it calls a flash-update hook table (`0xffdb5e8c`, one entry `0xffdb5d0e`). That hook reads the
  variable `AmiMediaCapsuleVariableName`. If it exists (`GetVariable` returns `EFI_BUFFER_TOO_SMALL`,
  `sub $0x80000005`), the boot mode becomes `0x12`.
- **`IsSecRecoveryPEI`** (`0xffd88af4`): outside S3 resume and outside recovery, it reads `AmiFlashUpd` (0x1D bytes) and
  `MonotonicCounter`. When the request byte is 1 and the counter matches, it sets boot mode `0x20` (`push $0x20; pop
  %edi … call *0x2c`). This is the OS-initiated recovery flash path (AFU / Secure Flash, inferred).
- **`RomLayoutPei`** (`0xfffca603`): calls `SetBootMode(0x20)`, installs a PPI, and reports status code `0x03031004`
  (`EFI_SOFTWARE_PEI_MODULE | EFI_SW_PEI_PC_RECOVERY_AUTO`, inferred from the PI status-code encoding). It is
  called from three places. Each runs after a ROM-layout area's volume check fails: a `_FVH` signature compare
  (`cmpl $0x4856465f`) or a validation routine returning `0x8000000A` (`EFI_VOLUME_CORRUPTED`), and only when an
  attribute bit `0x80` is clear. So **a structurally corrupted volume triggers recovery by itself** (inferred meaning of the
  observed code). Whether the check covers the contents (a hash) and not just the structure is unknown.
- A scan of all 81 TE-format PEI modules for `SetBootMode` with an immediate `0x20` found only `RomLayoutPei` and
  `IsRecovery`, plus `IsSecRecoveryPEI` through a register. The scan was bounded: it looked for the immediate-operand
  pattern only.

So the AMI "hold Ctrl+Home at power-on" trigger that other AMI boards document (Supermicro guide above) is **not
present in this image as an IsRecovery hook** (observed). Whether another module watches a key is **unknown**, but
no candidate was found.

### 3. Secure Flash: the BIOS region is signed (observed), and recovery checks it (inferred)

FV_MAIN's first file `414D94AD-998D-47D2-BFCD-4E882241DE32` (`0x15C2078`, 0x102C bytes) holds an AMI firmware-capsule
header: capsule GUID `4A3CA68B-7723-48FB-803D-578CC1FEC44D`, header size `0x1000`, a 14 MiB image (the BIOS region), a
ROM-layout table at offset `0xD30`, and a `WIN_CERTIFICATE_UEFI_GUID` (`0x0EF1`) of type
`A7717414-C616-4977-9420-844712A735BF` (`EFI_CERT_TYPE_RSA2048_SHA256_GUID`), filled with 4,084 non-`FF` bytes. A
firmware key file `5B85965C-455D-4CC6-9C4C-7F086967D2B0` at `0x1D801A8` holds a 32-byte value
`4aa78cdd…ae6d4310` (a key hash, inferred). The PEI phase has `FwKeyHobPei` and `CryptoPPI` next to the recovery modules.
ReFlash's strings include `Signature is invalid`, `Platform Key is invalid`, `ROM Layout mismatch detected`,
`Partial update is not allowed` and `Only full image can be updated`.

Consequence (inferred): only Uniwill's own signed code areas pass. A recovery file built from this backup's BIOS region
should pass, because NVRAM and SMBIOS data are outside the signed areas in AMI's design (inferred, not confirmed). A
modified image does not pass. **Which file layout recovery accepts (the 14 MiB BIOS region, the full 32 MiB flash, or a
capsule) is unknown.**

ReFlash recovery screen strings (UTF-16, observed): `Reset NVRAM`, `Boot Block Update`, `Main Block Update`,
`EC FIRMWARE Update`, `Updating EC Firmware`, `EC NOT Found!!`, `Backup boot block...`, `Write new boot block...`,
`Write new main block...`, `Write new nvram block...`, `Caution: A Reset is required to Clear the TPM for Flash Update`.
There is no ME or descriptor option. The **EC firmware option** matters for the map: a recovery image that carries EC
firmware could flash the EC, which is out of scope. This image's descriptor has no EC region (observed), and the 2 MiB "AMI ROM
hole" at `0x13B1000` holds only logo files (`Logo.bmp` and two other compressed freeform files, observed).

ROM-layout table (from the capsule header at `0x15C2DD4`, 24-byte entries: address, offset, size, type, attributes).
Attribute meanings are AMI-internal and **unknown**. Entries of note:

| Flash address | Size | Type | Attr | Matches |
|---|---|---|---|---|
| `0x1FC0000` | 0x40000 | FV | `0x660A` | top boot block |
| `0x1F80000` | 0x40000 | FV | `0x0200` | boot block copy |
| `0x1BC0000` | 0xC0000 | FV | `0x0608` | recovery PEI volume |
| `0x15C2000` | 0xA4 | FV | `0x020A` | FV_MAIN header |
| `0x1270000` | 0x1000 | FV | `0x4000` | SMBIOS data |
| `0x1260000` | 0x10000 | raw | 0 | gap |
| `0x1230000`, `0x1200000` | 0x30000 each | raw / FV | `0x6200` | NVRAM stores |
| `0x1BC0000` | 0x440000 | raw | 0 | the whole PEI/boot-block area |
| `0x1950000`–`0x1B10000` | five entries | FV | `0x0200` | same sizes as the PEI volumes, shifted down by `0x270000`, but that range is FV_MAIN free space here: purpose **unknown** |

### 4. Top Swap: a second boot block (observed copy, inferred mechanism)

    cmp -l <(dd … skip=0x1F80000 count=0x40000) <(dd … skip=0x1FC0000 count=0x40000) | wc -l   →  17

Intel's datasheet (635218 §3.2.1, Table 10): with a 256 KB boot-block size, Top Swap redirects `FFFC_0000h–FFFF_FFFFh`
to `FFF8_0000h–FFFB_FFFFh`, which are exactly these two volumes. "If a power failure occurs at any point after step 3, the
system will be able to boot from the copy of the boot block that is stored in the block below the top. … a pin strap
will allow forcing the top-swap bit to be set. This would be a last resort to allow the user to get the system to boot
(and avoid having to de-solder the system flash)." Table 17: `GPP_B14 / SPKR`, *Top Swap Override*, sampled at the rising
edge of `PCH_PWROK`, 20 kΩ internal pull-down, "1 ⇒ Enable Top Swap mode". ReFlash's `Backup boot block...` step
matches the datasheet's algorithm (copy the top block down, set Top Swap, rewrite the top block, inferred).

Current state (observed, passive PCI config reads):

    $ sudo setpci -s 00:1f.0 dc.b   →  84         # bit 4 (TS, Top Swap) = 0
    $ sudo setpci -s 00:1f.5 dc.l   →  10000888   # SPI BIOS_CONTROL, bit 4 (TSS) = 0

Top Swap is off, so the CPU boots the top copy. Limits: only the top 256 KiB has a copy. The PEI volumes between
`0x1BC0000` and `0x1F7FFFF` (memory init, recovery, USB) do not, so Top Swap rescues only damage confined to
`0x1FC0000`–`0x1FFFFFF` (inferred). Where `SPKR` can be reached on this mainboard (a test pad, the codec beep line) is
**unknown** and needs the board in hand. Bridging it to 3.3 V at power-up is a physical action for the owner.

### 5. ME region (observed layout, inferred behaviour)

UEFIExtract, ME region `0x1000`–`0x1000FFF`, IFWI 1.7:

| Partition | Offset | Contents |
|---|---|---|
| Boot partition 1 | `0x3000` (0x1EE000) | ROM Boot Extensions, **PMC**, **Bring Up**, OEM Key Manifest, TCSS FW IOM/PHY/TBT, PCH Config |
| Data partition | `0x1F1000` (0x88000) | ME file system (configuration, PTT/fTPM state, inferred) |
| Boot partition 2 | `0x279000` (0x1EE000) | **Main** |

For ME 11, PT's slides list a "Recovery" boot path and show the FTPR kernel, syslib and rbe running in "Boot: Normal,
Recovery", while NFTP modules run only in "Boot: Normal" (slide 22). Their CSME 12+ notes describe a "REC
(Recovery)" code partition directory. By analogy, the *Bring Up* code in boot partition 1 is the recovery copy and *Main* the
operational one (inferred for CSME 15). Boot partition 1 also holds the PMC firmware that powers the platform up. A broken
boot partition 1 therefore means no boot and a programmer (inferred).

Current ME state (observed, `/sys/class/mei/mei0/fw_status`, decoded with coreboot `me_15.h`/`cse.h`):

    HFSTS1 90000255 → working state 5 (Normal), manufacturing mode 1, FPT bad 0, operation state 1 (M0 with UMA),
                      init complete 1, error code 0, operation mode 0 (Normal)
    HFSTS3 00000020 → firmware SKU 2 (Consumer)
    fw_ver 15.0.30.1776

So the ME is healthy and not running from its recovery path today. Restoring the ME region from the backup also rolls back the
Data partition, including fTPM state, to 2026-11-15: anything sealed to the TPM after that date may stop unsealing (inferred).

### 6. NVRAM and SMBIOS data (observed)

- Two NVRAM stores, `0x1200000` and `0x1230000` (`FA4974FC…` volumes, 145 and 211 entries per #18). Copies of the factory
  defaults (`StdDefaults`) sit in both boot-block volumes (`NVRAM external defaults`, `0x1F8ED50` and `0x1FCED50`), and
  `NvramPei` reads `StdDefaults`. That the firmware falls back to these defaults on a corrupt store is AMI's design (inferred).
  Whether a CMOS/RTC clear (`CmosPei`) also resets NVRAM is **unknown**.
- The SMBIOS data file `FD44820B-F1AB-41C0-AE4E-0C55556EB9BD` sits in its own 4 KiB volume at `0x1270000`. The DMI
  model strings (`A52 MOB`) are there, and the live system and baseboard serial numbers (from `dmidecode -s`, compared
  in memory and not printed) both occur inside that 4 KiB (observed). `SmbiosDmiEdit` (SMM) keeps a `WriteOnceStatus`.
  Rewriting just this block from the backup restores the model and serial (inferred). One 4 KiB erase block,
  matching the VSCC 4 KB erase size in the descriptor (observed).
- Restoring whole NVRAM from the backup also rolls back Secure Boot `dbx` updates (fwupd reports dbx `20260402`) and
  boot entries (inferred). Restore only the damaged range.

### 7. Firmware update paths seen from Linux (observed)

`fwupdmgr get-devices`: *System Firmware* (UEFI capsule, ESRT class `14d67f09-a508-54cb-83c1-6ff4a0b026ab`), version
105, lowest supported 105, `updatable`. The ESRT (`/sys/firmware/efi/esrt/entries/entry0`) shows last attempt status 0
at version 105. `fwupdmgr get-releases` returns "No releases found". Four more ESRT device entries (`69585d92…`,
`d1144042…`, `4e88068b…`, `eed54281…`) are of **unknown** identity. The capsule path (`FmpCapsuleUpdate`, `CapsulePei`)
would accept only Uniwill-signed capsules and refuse versions below 105 (inferred from the lowest-supported field). It
does not help with recovery.

### 8. Uniwill-specific recovery (observed absence, bounded)

OEM modules present: `OemACRecoveryPei/Dxe` (reads `UniWillVariable`, which is AC power-loss restore, inferred from the
name), `OemOcPei/Dxe` (reads `CpuSetup` and does port I/O, purpose **unknown**), `OemManufactureModeDxe`, `OemHooks*`,
`OemBoardPei`, plus keyboard-light and power-mode drivers. None carries recovery strings. The hidden setup page `0x2712` is
titled "GCU - OverClock Recovery" (section 18). Whether that means an overclock fail-safe that reverts settings after
failed boots is **unknown**.

## Unknowns

1. Which file layout `GMXTGFXN105.ROM` must have (BIOS region, full flash, or capsule), and whether the BIOS region of
   this backup passes the signature check. Settling it statically needs a decompiler: REA's Ghidra provider is not
   configured (`GHIDRA_INSTALL_DIR`).
2. Whether `RomLayoutPei`'s check hashes volume contents or only checks structure, and the meaning of the ROM-layout
   attribute bits.
3. Where the `GPP_B14/SPKR` Top Swap strap can be reached on this mainboard.
4. Whether a CMOS/RTC reset clears NVRAM on this board, and whether `OemOcPei` is an overclock fail-safe.
5. Which USB ports work in the PEI phase (UsbPei, xHCI) for a recovery stick.
6. Identity of the four device ESRT entries.

## For the owner

- **Decision: rely on crisis recovery or not.** It works only for the stock image, and its file format is untested. A
  test would mean corrupting the main volume on purpose, so this research does not propose one. Suggested stance: treat
  the programmer (#19) as the only rollback, and keep a recovery stick with `GMXTGFXN105.ROM` (built from the backup's
  BIOS region) as a cheap extra that may or may not work.
- **Every modified image (map risk ceiling) has only the programmer as its rollback** once it fails to boot. Recovery
  rejects unsigned images, and the boot block it relies on would be the modified one. Have the programmer and clip
  verified on this chip before the first modified-image flash.
- **Restore by region, not the whole flash**, so that NVRAM (dbx, boot entries) and the ME data partition (fTPM) are
  not rolled back unless they are the damaged part.
- **Never pick "EC FIRMWARE Update"** on the ReFlash screen (out of scope: EC flashing).
- The committed backup image contains this laptop's serial numbers (in the SMBIOS volume). The repository is private
  now. If it ever goes public, strip or redact `evidence/bios/*.bin` first.
