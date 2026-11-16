# What may change in the ME, and what is irreversible? (#28)

Research for map #23 (BIOS and firmware deep dive). Read only: nothing on the laptop was written. Every finding is
marked **observed** (read on this machine or in its own flash image), **inferred** (deduced, or taken from another
board or a secondary source) or **unknown**.

## Answer

- **HAP bit: probably works, not verified on this board; reversible.** On Tiger Lake-H the HAP bit is PCH strap 37,
  bit 16 (byte `0x196` of the descriptor), according to Dasharo's coreboot. Upstream coreboot says the location is not known
  for Tiger Lake. On this board the bit is clear. Setting it is a 1-bit descriptor change that the programmer undoes by
  writing back the 4 KiB descriptor. Cost: no PTT (this machine's TPM *is* PTT), no HDCP/PXP through the MEI, and,
  per one Dasharo report on Tiger Lake, no S0ix and no package C10 (higher idle power). Thunderbolt costs nothing:
  it is already disabled and absent.
- **The BIOS's own "ME State = Disabled" (HECI soft disable) is the gentler variant**: a setup option, no flash image
  change, undone from the same menu. It costs about the same as HAP.
- **me_cleaner-style trimming does not work** on this image: me_cleaner cannot parse the IFWI 1.7 layout (observed:
  it stops with "firmware is corrupted or missing"). Its `-S` path would also set the wrong strap bit on Tiger Lake. No
  maintained tool trims CSME 15. Not worth pursuing.
- **Irreversible (never do; out of scope, named so nobody does them by accident):** (1) anything that closes
  manufacturing mode: End of Manufacturing (EOM), `FPT -closemnf`, or flashing an ME region whose MFIT configuration
  commits EOM on its first boot. EOM burns the field-programmable fuses (FPFs): OEM key hash, Boot Guard policy, ARB
  SVN. It also locks the descriptor. (2) The two **Anti-Rollback SVN** options in this BIOS's setup (*Automatic
  HW-Enforced Anti-Rollback SVN*, *Set HW-Enforced Anti-Rollback for Current SVN*), which burn the ARB SVN into fuses. After that, an
  older ME, including the backup's, refuses to boot. (3) Running vendor or sibling ME/BIOS update packages or Intel
  CSME System Tools at all, since they can do (1) or (2).
- **A newer CSME 15.0 build is not worth carrying now.** 15.0.30 misses fixes up to 15.0.56 (Intel SA-00783,
  SA-00999, SA-01315, SA-01427, ...). Most of those are AMT bugs, and AMT does not run on this Consumer SKU. The few
  CSME-proper ones are local-privileged, adjacent or physical. That adds little on a machine whose flash ADR-0002 leaves
  writable to root. Building a newer region needs Intel's non-public CSME System Tools and carries the EOM risk above.
- **What the programmer undoes by restoring the ME region (and descriptor):** HAP, any trim, any CSME version
  change, ME setup state in the ME's file system. It does *not* undo fuse burns, or the loss of data sealed in PTT. A
  restore also rolls PTT's stored state back to the backup date (2026-11-15).

## Evidence

### 1. What the ME on this machine is

| Finding | Status | Source |
|---|---|---|
| CSME 15.0.30.1776, all three version slots | observed | `/sys/class/mei/mei0/fw_ver` → `0:15.0.30.1776` ×3 |
| FTPR, NFTP and RBEP manifests: `$MN2`, 15.0.30.1776, dated 2021-04-28, header flags `0x1`, manifest SVN field 1 | observed | `$MN2` headers at image offsets `0x5C2CC`, `0x27A164`, `0x405C` (`od -t x1`, version at +0x24, SVN at +0x2C) |
| The SVN field of 1 is the ARB SVN | inferred | field layout as read by MEAnalyzer/me_cleaner; not decoded by an Intel tool |
| Other IFWI parts: PMC 150.2.10.1017, IOM 24.25.0.0, NPHY 15.106.135.5015, TBT 14.0.0.2500, PCHC 15.0.0.1021, OEM Key Manifest (OEMP, OEM-signed) | observed | same `$MN2` header parse; partition list in `evidence/bios/uefiextract-report.txt` lines 1-290 |
| Firmware SKU **Consumer** (HFSTS3 bits 6:4 = 2) | observed | `fw_status` HFSTS3 `0x00000020`; values from coreboot `me_15.h` and fwupd `FuMeiFirmwareSku` (Consumer = 2) |
| No AMT: the NFTP module list holds dal, pavp, ish_srv, mctp, icc, tls_iso, vdm, sigma, hotham, iunitcse, adspa, and no AMT modules | observed (list) / inferred (no AMT) | `uefiextract-report.txt` lines 257-270 |
| PTT (the fTPM) is a module inside FTPR, the partition that also runs at bring-up | observed | `uefiextract-report.txt` line 102 (`ptt`) |
| The TPM is PTT: TPM2 manufacturer `INTC`, ACPI `MSFT0101`, TPM2 table start method 7 (CRB) | observed | read-only `TPM2_GetCapability(TPM_PT_MANUFACTURER)` on `/dev/tpmrm0` → `INTC`; `/sys/firmware/acpi/tables/TPM2` byte 48 = 7 |
| The root LUKS volume has only a passphrase keyslot, no systemd-tpm2 token | observed | `cryptsetup luksDump /dev/nvme0n1p2` (keyslot 0 only, no tokens) |
| i915 uses the MEI for HDCP and PXP | observed | kernel log: `mei_hdcp ... bound 0000:00:02.0`, `mei_pxp ... bound` |
| The ME region is 16 MiB, but its content ends at `0x467000` (about 4.4 MiB); the rest is `FFh` padding | observed | `uefiextract-report.txt` line 290 |
| REA identity of the extracted ME region: SHA-256 `377582153495180a301864573ce648b906d3aa6219a36dc59e809863404a6736`. Binwalk recognises no CSME structure in it, only seven spurious small signatures | observed | REA `inspect_firmware_regions` Evidence `ev_7c19f98acfb5351970789535923b99916aabef77b1eb388b90c3003040c2b543` (region cut with `dd bs=4096 skip=1 count=4096`) |

### 2. Manufacturing mode, fuses and Boot Guard today

`/sys/class/mei/mei0/fw_status` = `90000255 8B210506 00000020 00004000 00000000 00400000` (HFSTS1-6), decoded with coreboot
[`me_15.h`](https://github.com/coreboot/coreboot/blob/f13a6c7a61a761559d719f2b2e2a0ae1f43ddeb9/src/soc/intel/common/block/include/intelblocks/me_15.h)
and fwupd [`fu-intel-me.rs`](https://github.com/fwupd/fwupd/blob/bb7de036c0a2549ad09f913e77f76a4d84afc88f/libfwupdplugin/fu-intel-me.rs):

| Field | Value | Meaning | Status |
|---|---|---|---|
| HFSTS1 working state | 5 | Normal | observed |
| HFSTS1 bit 4 (`mfg_mode` / `spi_protection_mode`) | 1 | manufacturing mode, SPI unprotected | observed |
| HFSTS1 operation mode (bits 19:16) | 0 | Normal; HAP would read 2 ("Debug" in fwupd, "ME ALT Disabled" in this BIOS's setup strings), soft disable 3 | observed (value) / inferred (meaning of 2 for HAP) |
| HFSTS6 bit 30 `fpf_soc_lock` | 0 | FPFs not locked | observed |
| HFSTS6 bit 28 `boot_guard_disable` | 0 | Boot Guard not disabled by FPF; key manifest ID 1 | observed |
| fwupd HSI: `Mei.ManufacturingMode` not-locked; `IntelBootguard.Enabled` enabled but `.Otp`, `.Verified`, `.Acm`, `.Policy` not-valid; `Mei.Version` not-valid (15.0.30 < 15.0.40); `Spi.Descriptor` BIOS and ME read+write | | | observed: `fwupdmgr security --json --force` (fwupd 2.1.8) |

Coreboot notes that from Tiger Lake on, this bit alone does not prove EOM has not happened in a staged flow
([`cse.c` lines 338-354](https://github.com/coreboot/coreboot/blob/f13a6c7a61a761559d719f2b2e2a0ae1f43ddeb9/src/soc/intel/common/block/cse/cse.c#L338-L354)).
Together with `fpf_soc_lock` = 0, OTP not valid and Boot Guard not verifying, **EOM has not happened on this board**
(inferred, strong). This matches INVESTIGATION 7.7.

### 3. What EOM is and why it is irreversible (primary sources)

- Intel, [Key Usage in Integrated Firmware Images](https://www.intel.com/content/www/us/en/developer/articles/technical/software-security-guidance/resources/key-usage-in-integrated-firmware-images.html):
  "The FPFs are persistently stored in the Platform Controller Hub (PCH), and once provisioned, a hardware lock is
  closed." "The fuses cannot be changed for the life of the platform. This is referred to as the End of Manufacturing
  (EOM)." "The FPFs ... can only be programmed via the manageability engine in the PCH." Boot Guard policies
  "during development ... may reside in CSE internal non-volatile (NV) variables" and "are committed into FPFs at the
  End of Manufacturing (EOM) process". The OEM KM key hash "is programmed into the FPF at the End of Manufacturing".
  "If the OEM KM has not been configured at EOM, an FPF will be permanently set to indicate that the OEM KM is not present."
- Intel, [CSME Security Technical White Paper](https://www.intel.com/content/dam/www/public/us/en/security-advisory/documents/intel-csme-security-white-paper.pdf)
  rev. 1.5, Oct 2022 (SHA-256 `687f91d606ae8b0632c1e69249333403847260e248920a5b96cf9b54558c2280`), section 6.1: the EoM command
  "Write[s] and lock[s] manufacturers' settings into FPFs", "Close[s] some CSME APIs used for debugging and
  manufacturing" and "Lock[s] the SPI flash descriptor". Page 24: hardware-enforced anti-rollback makes "the new
  firmware ... burn specific, FPFs to record the FW ARB (anti-rollback) SVN into hardware"; CSME firmware with a lower
  SVN is then refused. The mechanism exists since CSME 12.0.
- Coreboot [`cse.c` lines 338-343](https://github.com/coreboot/coreboot/blob/f13a6c7a61a761559d719f2b2e2a0ae1f43ddeb9/src/soc/intel/common/block/cse/cse.c#L338-L343):
  on Tiger Lake EOM "is triggered ... either through first boot with required MFIT configuration or FPT /CLOSEMANUF".
  So **flashing an ME region is enough to trigger EOM** if that region was built with EOM set.
- Positive Technologies' [mmdetect](https://github.com/ptresearch/mmdetect): "When Manufacturing Mode is enabled, Intel
  ME allows execution of the command which makes the ME region writable". (Their 2018 blog post on manufacturing mode,
  CVE-2018-4251, was unreachable at the time of writing.)

What EOM would burn on this board is **unknown** in detail. The policies wait in the ME's own configuration (`FITC` and
`MFS` partitions at `0x1F2000` and `0x1FF000`, observed), and no open tool decodes them. Inferred from Intel's
documents: the OEM KM key hash (this image has an OEM-signed OEMP), the Boot Guard profile and key hash, the ARB SVN
and the descriptor lock. If the Boot Guard key hash is the vendor's, every modified BIOS image would stop booting, and
the programmer could not undo it.

**Inferred risk not named in ADR-0002:** while manufacturing mode is open and the FPFs are unlocked, root software with
Intel's manufacturing tools could commit FPFs of its own choosing. ADR-0002 accepts persistence in the flash, which
the programmer undoes. Burnt fuses would be permanent.

### 4. Setup options in this BIOS that touch the ME

Parsed from this image's Setup forms with `bios/analyze.py`'s parser (script outside the repo, run with `python3 -I`):

| Setting | Variable | Current | Path in the form tree | Effect | Reversible? |
|---|---|---|---|---|---|
| ME State (Disabled / **Enabled**) | `MeSetupStorage[0x2]` | 1 | Advanced > PCH-FW Configuration (linked without conditions) | Help: "When Disabled ME will be put into ME Temporarily Disabled Mode" (HECI soft disable) | yes, same option (observed option; effect inferred from Intel/coreboot `MKHI_SET_ME_DISABLE`) |
| **Automatic HW-Enforced Anti-Rollback SVN** | `MeSetup[0x2a]` | 0 (Disabled) | ... > Anti-Rollback SVN Configuration | Help: "once ME FW was successfully run on a platform, FW with lower ARB-SVN will be blocked from execution" | **no: burns ARB FPF** |
| **Set HW-Enforced Anti-Rollback for Current SVN** | `MeSetupStorage[0xc]` | 0 | same (shown while the automatic one is Disabled) | Help: "FW with lower ARB-SVN will be blocked from execution" | **no** |
| TPM Device Selection (dTPM / **PTT**) | `AmiWrapperSetup[0x3]` | not stored, default PTT | ... > PTT Configuration | Help: "PTT/dTPM will be disabled and all data saved on it will be lost" | setting yes, data no |
| Me FW Image Re-Flash | `MeSetup[0x3]` | 0 | ... > Firmware Update Configuration | opens the ME region to host writes; the descriptor already grants them (`FLMSTR1` = `0xffffffff`) | yes |
| FW Update | `MeSetupStorage[0x1]` | 1 | same | ME firmware updates through the HECI | yes |
| ME Unconfig on RTC Clear | `MeSetup[0x11]` | 1 | PCH-FW Configuration | a CMOS clear resets ME settings | yes |
| End Of Post Message (Send in DXE) | `MeSetup[0x13]` | 2 | ME Debug Configuration (unlinked form) | leave alone: without EOP the ME stays in pre-boot mode | yes |
| Integrated Thunderbolt Support | `Setup[0x6db]` | 0 (disabled) | Advanced > Platform Settings > TCSS > Thunderbolt | | |
| Low Power S0 Idle Capability; PEP CSME | `Setup[0x39]`, `Setup[0x4b]` | 1, 1 | Advanced > RC ACPI Settings | S0ix is declared, and the OS power policy counts the CSME | |

Status: the forms, variables and help texts are **observed** in the image. Whether Avell's setup actually shows *PCH-FW
Configuration* is **unknown**: the root form links it without conditions, but AMI can hide forms by other means. The owner
can check by eye, and it is the hidden-setup-menus ticket's question. The gray-out gate on most of these options
(`MeSetupStorage[0x3]`) is probably Intel's "after End of Post" flag (inferred).

The BIOS module `OemManufactureModeDxe` (`5F2719E2-4235-44EB-B908-CF74CE26B343`, 3,360 bytes, SHA-256
`4bbd657e5ddfe7629a303d7c5a8201e9298219689e23842e90f90ecf0e47b32e`) names only the variables `SystemAccess` and
`SecureBootSetup` (UTF-16 strings). It looks like an OEM factory mode for setup and Secure Boot, not the CSME's EOM
(inferred). REA could not decompile it: no deep-analysis provider is configured (Ghidra `GHIDRA_INSTALL_DIR` unset).

### 5. HAP bit

| Finding | Status | Source |
|---|---|---|
| me_cleaner (corna, master `41eba87`, v1.2) cannot parse this ME region: `python3 -I me_cleaner.py -c <copy>` → "Full image detected / The ME/TXE region is valid but the firmware is corrupted or missing", exit 1. It looks for `$FPT` at the region start, but IFWI 1.7 begins with an IFWI header and BPDT; the FPT is at `0x1F1000` | observed (run on a copy; the copy's SHA-256 was unchanged afterwards) | [me_cleaner.py lines 600-611](https://github.com/corna/me_cleaner/blob/41eba87c85a1f0d013790f12b649c2da5b35dcfa/me_cleaner.py#L600-L611) |
| me_cleaner's `-S` sets PCHSTRP0 bit 16 for every ME ≥ 11 | observed (source) | [me_cleaner.py lines 878-882](https://github.com/corna/me_cleaner/blob/41eba87c85a1f0d013790f12b649c2da5b35dcfa/me_cleaner.py#L878-L882) |
| Upstream coreboot: that location is Skylake/Kaby Lake only, and using it elsewhere "is completely invalid and leads to setting a different bit". Its `ifdtool` now knows HAP only for SKL/KBL and ADL, and refuses on Tiger Lake ("HAP bit location not known for platform") | observed (source) | coreboot commit [`a7c655550c`](https://github.com/coreboot/coreboot/commit/a7c655550c) (2026-11-08); [`ifdtool.c` lines 887-920](https://github.com/coreboot/coreboot/blob/f13a6c7a61a761559d719f2b2e2a0ae1f43ddeb9/util/ifdtool/ifdtool.c#L887-L920) |
| Dasharo's coreboot: HAP for `SOC_INTEL_TIGERLAKE_PCH_H` is descriptor byte `0x196` bit 0, that is PCHSTRP37 bit 16; Dasharo's `ifdtool` uses strap `0x94/4` = 37 for "RKL-H/TGL-H" | observed (source) | [Dasharo `me_15.h`](https://github.com/Dasharo/coreboot/blob/9e398ce002bdbecf58b5c816924ce0a51b1f8a55/src/soc/intel/common/block/include/intelblocks/me_15.h), [Dasharo `ifdtool.c` lines 852-879](https://github.com/Dasharo/coreboot/blob/9e398ce002bdbecf58b5c816924ce0a51b1f8a55/util/ifdtool/ifdtool.c#L852-L879) |
| This board: PCH strap section at `0x100`, PSL 101 words; PCHSTRP37 (`0x194`) = `0x800000b0`, so byte `0x196` = `0x00`: **HAP clear**; PCHSTRP31 (`0x17c`) = 0 | observed | `od -A x -t x4 -j 0x100 -N 0x1a0` on the image; `evidence/bios/ifdtool-dump.txt` |
| Dasharo's `ifdtool` sanity check expects bits 8:1 of PCHSTRP37 to be `0x18`; here they are `0x58`, so that tool would refuse this descriptor ("Unusual Tiger Lake flash descriptor") | observed (arithmetic on the observed value) | same |
| A System76 Tiger Lake-H descriptor (oryp8, `HAP: false`, ME 15.0.49.2573 per its README) has the same PCHSTRP37 `0x800000b0` | observed on the sibling file, not a fact about this board | `fd.rom` from `https://github.com/system76/firmware-open/raw/master/models/oryp8/fd.rom`, 4,096 bytes, SHA-256 `5a45ede5fa56ac39a6ff86e76f8c04b38da4940104298344a7f9a68a8464ac68`, not committed; REA Evidence `ev_e1a06fae9e51ff00c52770a87654ca4ad9062317e9be15cf83f016ad6aef09ff` |
| Whether setting PCHSTRP37 bit 16 puts *this* ME into HAP mode | **unknown**: no Tiger Lake-H board in Dasharo's own list confirms the offset; only a flash test settles it (HFSTS1 operation mode would read 2) | |

**Costs of HAP or soft disable on this machine**

| Area | Cost | Status |
|---|---|---|
| PTT | no fTPM: the TPM disappears (PTT runs in the CSME, and HAP stops the CSME after bring-up). systemd's TPM services lose their device; LUKS does not need it | inferred (mechanism); observed (TPM is PTT, LUKS has no TPM token) |
| S0ix / modern standby | Dasharo on Tiger Lake: "disabling ME breaks suspend mode" and "prevents the CPU package from going into C-state C10, causing high power usage" ([dasharo-issues#111](https://github.com/Dasharo/dasharo-issues/issues/111), NV4x, 2022). Coreboot refuses S0ix when the ME is disabled ([`cse.c` lines 319-335](https://github.com/coreboot/coreboot/blob/f13a6c7a61a761559d719f2b2e2a0ae1f43ddeb9/src/soc/intel/common/block/cse/cse.c#L319-L335)) | inferred (sibling platforms) |
| S0ix today | the machine already does not reach S0ix: two s2idle suspends this boot, yet `slp_s0_residency_usec` = 0 and S0i2.0/S0i3.0 residency 0. S3 is offered (`/sys/power/mem_sleep` = `[s2idle] deep`) | observed |
| Package C10 / idle power | no idle baseline yet. The only turbostat sample (5 s) was taken under load: no package C-states, 31.6 W. The owner should measure idle before and after any ME change | unknown |
| Thunderbolt / Type-C | none: integrated Thunderbolt is disabled in setup and no TCSS, Thunderbolt or USB4 PCI function exists (`lspci -nn`) | observed |
| Graphics content protection | HDCP and PXP via `mei_hdcp`/`mei_pxp` stop working | inferred |
| Thermals / fans | no direct link: fans and thermal limits are the EC's and the CPU's (INVESTIGATION 7.3, 7.4). Only indirectly, through idle power | inferred |
| Boot | the BIOS knows the states (setup strings "ME ALT Disabled", "ME Temporarily Disabled"); any extra boot delay from HECI timeouts is unknown | observed (strings) / unknown (delay) |
| fwupd | loses the ME version and HSI attributes | inferred |

### 6. Trimming (me_cleaner style)

Not possible with available tooling (section 5: me_cleaner cannot parse IFWI 1.7). On CSME 12+ every partition is
signed on its own, and on Tiger Lake the IFWI also carries the PMC, Type-C and PCH configuration firmware
(`PMCP`, `IOMP`, `NPHY`, `TBTP`, `PCHC`, observed). A trim would have to keep those and the bring-up modules (`rbe`,
`kernel`, `syslib`, `bup`, per [me_cleaner.py line 31](https://github.com/corna/me_cleaner/blob/41eba87c85a1f0d013790f12b649c2da5b35dcfa/me_cleaner.py#L31)).
Its effect on this BIOS is unknown. It would be reversible by restoring the region. It gives nothing HAP does not, for
much more risk.

### 7. A newer CSME 15.0 build

| Advisory | 15.0 fixed in | CSME-proper issues (not AMT) | Status |
|---|---|---|---|
| [SA-00613](https://www.intel.com/content/www/us/en/security-center/advisory/intel-sa-00613.html) (2022-05) | 15.0.40 (11th Gen line) | CVE-2022-0004, Boot Guard/TXT debug-mode override, physical | observed (page) |
| [SA-00610](https://www.intel.com/content/www/us/en/security-center/advisory/intel-sa-00610.html) (2022-11) | 15.0.42 | AMT only (plus SPS) | observed (page) |
| [SA-00783](https://www.intel.com/content/www/us/en/security-center/advisory/intel-sa-00783.html) (2023-08) | 15.0.45 | CVE-2022-38102, local privileged DoS | observed (page) |
| [SA-00999](https://www.intel.com/content/www/us/en/security-center/advisory/intel-sa-00999.html) (2024-08) | 15.0.49 | CVE-2023-40067 (physical EoP), CVE-2023-48361, CVE-2024-21844 (adjacent DoS), CVE-2023-34424 | observed (page) |
| [SA-01152](https://www.intel.com/content/www/us/en/security-center/advisory/intel-sa-01152.html) (2025-02) | 15.0.50 | AMT and the Windows driver installer | observed (page) |
| [SA-01315](https://www.intel.com/content/www/us/en/security-center/advisory/intel-sa-01315.html) (2026-02) | 15.0.55 | CVE-2025-27708, local privileged info disclosure | observed (page) |
| [SA-01427](https://www.intel.com/content/www/us/en/security-center/advisory/intel-sa-01427.html) (2026-08) | 15.0.56 | AMT only | observed (page) |

Advisory list from Intel's [CSME Version Detection Tool release notes](https://downloadmirror.intel.com/28632/Intel_CSME_Version_Detection_Tool_Release_Note_15_0_4_22.txt)
(15.0.4, July 2026). fwupd treats 15.0 builds below 15.0.40 as vulnerable
([`fu-intel-me-device.c` lines 141-196](https://github.com/fwupd/fwupd/blob/bb7de036c0a2549ad09f913e77f76a4d84afc88f/libfwupdplugin/fu-intel-me-device.c#L141-L196)).

Inferred weighing:
- The AMT issues do not apply to a Consumer SKU.
- The rest are local-privileged, adjacent or physical. On this machine root can already rewrite the flash (ADR-0002), and anyone with physical access can clip the chip.
- So the security gain is small.
- Carrying a newer build means either Intel's FWUpdate tool, which updates in place, needs Windows or an EFI shell, and is not public, or a rebuild with Intel's MFIT, which is not public, has to reproduce the OEM configuration, and is exactly the step that can commit EOM.
- Whole ME regions from other boards (System76 ships 15.0.49.2573 for oryp8) carry those boards' configuration: never flash them.
- No Avell or sibling CSME update exists (INVESTIGATION 7.1).

Not worth it unless a CSME-proper, remotely reachable bug appears.

### 8. What a programmer restore undoes

| Change | Undone by writing back... | Status |
|---|---|---|
| HAP bit | the descriptor (`0x0000-0x0FFF`) | inferred (soft strap read at power-on) |
| Trim, CSME version change, ME region from any source | the ME region (`0x1000-0x1000FFF`) | inferred; holds only while no FPF was committed |
| ME settings in setup (ME State, PTT/dTPM, FIPS, UPID) | the setup option, or the BIOS region's NVRAM | inferred |
| MFS contents (PTT storage, ME state) | the ME region, but **back to 2026-11-15**: anything sealed to PTT after that date stops working | inferred |
| ARB SVN if the ARB setup options are not used | the ME region; the `PSVN` partition (`0x1FA000`) travels with it | inferred |
| EOM, `-closemnf`, ARB HW enforcement, any FPF write | **nothing** | from Intel's documents (section 3) |

## Open unknowns

- Whether PCHSTRP37 bit 16 is HAP on *this* board: only a flash test, with the programmer ready, settles it.
- What the `FITC`/`MFS` configuration would commit at EOM (Boot Guard profile, key hashes). No open decoder. A third-party
  tool (MEAnalyzer) shows part of it, and running it is the owner's call.
- Whether Avell's setup shows *PCH-FW Configuration* and the ARB submenu (owner, by eye).
- Idle power and package C10 with the ME as it is, as the baseline for any ME change (owner, idle and on battery).
- Why S0ix residency is 0 after s2idle suspends (likely not the ME; the idle-power ticket).
- Whether the boot slows down with the ME disabled.

## Proposed for the owner

- Write the irreversible list (section 8, last row, and the setup ARB options) into ADR-0002 or the hardware-safety rules,
  so no session or tool ever runs FPT/MFIT, vendor flash packages or the ARB options.
- No ME change is recommended now. If one is wanted, try the setup's *ME State = Disabled* first: reversible and with no
  image change. Measure idle power and suspend before and after.

## Commands

All read-only:

```
cat /sys/class/mei/mei0/{fw_ver,fw_status}
fwupdmgr get-devices --json ; fwupdmgr security --json --force
sudo od -A d -t x1 /sys/firmware/acpi/tables/TPM2
python3 -I tpm_mfr.py            # TPM2_GetCapability(TPM_PROPERTIES, TPM_PT_MANUFACTURER) on /dev/tpmrm0 -> INTC
sudo cryptsetup luksDump /dev/nvme0n1p2
cat /sys/power/mem_sleep ; sudo cat /sys/kernel/debug/pmc_core/{slp_s0_residency_usec,substate_residencies}
od -A x -t x4 -j 0x100 -N 0x1a0 evidence/bios/bios-N.1.05AVE04-20261115.bin     # PCH straps
od -A n -t x1 -j <manifest offset> -N 0x40 <image>                             # $MN2 headers
python3 -I me_forms.py <image> '<regex>'   # imports bios/analyze.py + nvar.py; lists Setup questions and NVAR values
python3 -I me_cleaner.py -c <copy of image>
```

Downloaded and not committed (each into its own `mktemp -d` directory): System76 oryp8 `fd.rom` (above), coreboot,
Dasharo, me_cleaner and fwupd sources at the commits linked, Intel's white paper (SHA-256 above).
