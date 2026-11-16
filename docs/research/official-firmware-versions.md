# Which newer official firmware exists for this laptop's parts? (#30)

Research for the wayfinder map #23. Read only: nothing was written to the flash, EFI variables, the EC or any device.
Each finding is marked **observed** (read on this machine or its own flash image), **inferred** (deduced, not confirmed
here) or **unknown**. Dates are those of the sources, read on 2026-11-15/16.
## Answer

| Component | Installed | Newest official | Through what | What the newer one fixes |
|---|---|---|---|---|
| CPU microcode (i5-11400H, `06-8d-01`, platform ID `0xc2`) | **in the image: `0x34`** (2021-05-23); **running: `0x58`**, loaded early by Linux | `0x58` (2025-07-24), still current in Intel's latest release `microcode-20261110` | Already applied: Arch `intel-ucode` through the initramfs `microcode` hook. Into the image only by a modified image | 17 Intel releases between `0x2c` and `0x58`, with security fixes from INTEL-SA-00442 to INTEL-SA-01396 (table below) |
| Intel CSME (consumer SKU, HM570) | `15.0.30.1776` (2021-04-28, SVN 1) | `15.0.56` per Intel's advisories (2026-08-11, revised 2026-09-09); build `15.0.56.2834` seen in an OEM package (2026-10-27) | **Neither fwupd nor any vendor tool for this board.** Only a modified image (ME region), or Intel's update tool with an image from outside Avell's channels | Ten Intel advisories with a fixed version above 15.0.30 (table below). fwupd's HSI flags the running version as vulnerable |
| GOP (graphics boot driver) | build `1064` (major/minor not stored as text) | unknown: Intel gives GOP builds only to firmware vendors | Only a modified image | unknown |
| VBT | `$VBT TIGERLAKE`, BDB version 240 | unknown | Only a modified image | unknown |
| RST / VMD | UEFI VMD driver `18.1.1.5201`; legacy SATA option ROM `17.3.0.4067` | unknown for the UEFI driver (Intel ships it to firmware vendors only) | Only a modified image | Linux does not use either: VMD has no device under it and the SSD is attached directly (INVESTIGATION 18) |
| NVMe SSD (NX-1TB 2280, Realtek RTS5772DL) | `VF100C6B` | **unknown**: no vendor publishes firmware for this drive | Would be fwupd's NVMe plugin or `nvme fw-download`, if an image existed; none on LVFS | unknown |
| Webcam (Chicony `04f2:b711`) | `97.45` | **unknown**: no release on LVFS for it | fwupd's DFU plugin would carry one; none exists | unknown |
| BIOS (sibling boards) | `N.1.05AVE04` (2022-08-30) | **none found newer**, from Avell or any sibling vendor's official pages | None: no vendor of this board is on LVFS, and Avell gives BIOS updates only at service centres | n/a |
| EC (sibling boards) | `1.13` (from DMI) | **none found** on any official page | None official | n/a |

In short: **the only newer official firmware that matters is the ME**, and it reaches this board only through a modified
image. The microcode is already current at runtime. Nothing here is offered through fwupd/LVFS: `fwupdmgr get-updates`
reports no updates for any device, and none of the board's vendors (Uniwill, Tongfang, Avell, Monster, Mechrevo,
CyberPowerPC) is an LVFS vendor.

One finding outside the question but relevant to the map: **all four device-update (FMP) drivers in the image embed
the same self-signed AMI test certificate (`CN=test1`)**. If that is the capsule trust anchor, this firmware's update
path for microcode, ME, Boot Guard ACM and retimer is guarded by a test key. See "Capsule update path" below.

## Evidence

### Starting facts reused (not redone)

`evidence/bios/` (backup `bios-N.1.05AVE04-20261115.bin`, SHA-256 `baa08b4a...d52412`), INVESTIGATION 7.1 (sibling list;
"no newer BIOS or EC found", inferred), 7.7 (ME manufacturing mode, Boot Guard not enforcing), 12 and 18 (VMD enabled
with nothing under it; SSD on the chipset root port).

All image work below was done on a copy of the committed backup in `/tmp/fwres-own-image.*`, extracted with
`uefiextract image.bin all` (UEFIExtract NE). Extracted leaf files and their SHA-256:

| Leaf | SHA-256 |
|---|---|
| GOP driver PE (file `A0327FE0`, section `380B6B4F`) | `369d366e937b60784a38b9b46a701ca9c6f5441086670b1ec359e6c558fea605` |
| RST VMD driver PE (file `8D3168FD`) | `8462b6c3d7a7a23c8628635f377921c8edf7bc9c8287be9558909c1c8772ec42` |
| VBT raw section (file `56752DA9`) | `e179c8b4c2c7959d9319e91e680b54d03d58201fa2dfcf96b151e04e109cf59e` |
| RST SATA option ROM (`A0327FE0` / `D46346CA`) | `6b5954641f3bdc4c7e4c5f9b9c6bf45e611e89aec40ccf502c453d1c8afe6b13` |
| ME FmpDxe PE (file `0EAB05C1`) | `ee88adc34a7f0cd3d00e35337f06a81cc62c01c347641cfd72cdbde85280a6fc` |

REA Evidence (this session; bundle saved to `/tmp/fwres-rea/official-firmware-versions.rea.json`, not committed):

| Evidence id | What |
|---|---|
| `ev_612e17e70cf62a35173e27d50daded28183298b8fb8989c4b206823d4783d29d` | `inspect_firmware_regions` (binwalk 3.1.0) over the whole image |
| `ev_0dffcc191e6674d3c427f4c93a602d1377d98f4ce93541b7448f236e9b58a776` | GOP PE: UTF-16 `Intel(R) GOP Driver` at `0x251b0` (Hopper) |
| `ev_c1a3819ba7850bc8c14f1c9d58dfd983c29039f94180f6b87b51437defe8628a` | RST PE: UTF-16 `Intel(R) RST 18.1.1.5201 RST VMD Driver` at `0x271b0` |
| `ev_bf6c46c99ebfd2f99921b1672ee2bfc3b0e3036c9723cf2ef62faff529a86d1d` | ME FmpDxe PE: UTF-16 `TigerLake CSME System Firmware Device` at `0xdbb10` |

Limits of the REA pass: Hopper's string inventory skips strings under five characters and strings inside DER data, so
the GOP build `1064` and the certificate below were read with the local scripts named in each section, not REA
(`ev_66d5618e...` and `ev_5ff177fd...` are those two empty searches).

### 1. CPU microcode

- **observed**: the image carries one microcode update for CPUID `0x806D1`, revision `0x34`, dated 2021-05-23,
  platform flags `0xc2`, 100,352 bytes, checksum valid, in two copies (`0x1E31000` in volume `4409A1B5`,
  `0x1EC1000` in volume `AFDD39F1`, both inside FFS file `17088572-377F-44EF-8F4E-B09FFF46A070`). The FIT has one
  microcode entry, at `0xFFEC1000` (the second copy), plus a startup ACM entry at `0xFFF40000`, and **no key
  manifest or boot policy manifest entries**. Read with a header and FIT scan of the image (`ucode.py`, `fit.py`).
- **observed**: the ESRT's microcode FMP entry (`69585d92-...`, the `UcAcmFmp` driver) reports version 52 = `0x34`.
- **observed**: `journalctl -k -b`: `microcode: Current revision: 0x00000058`, `microcode: Updated early from: 0x00000034`.
  The update comes from `intel-ucode 20260925-1` (`/usr/lib/firmware/intel-ucode/06-8d-01`, through mkinitcpio's
  `microcode` hook). Its SHA-256 `20b65b22457c835ed7d71b30c0ddcf9665cdbbe1ffe307e7ce69fa7fb9066336` equals Intel's file
  at tag `microcode-20261110` (header: revision `0x58`, date 2025-07-24, total size 105,472 bytes).
- **observed** (primary source: [Intel release notes](https://github.com/intel/Intel-Linux-Processor-Microcode-Data-Files/blob/main/releasenote.md)):
  TGL-H `06-8d-01/c2` history. The security advisories are those the release lists for all its processors; which of them
  concern TGL-H is in each advisory (not checked one by one: **inferred** that most do, since the row changed).

  | Release | Revision | Security advisories named by the release |
  |---|---|---|
  | microcode-20210608 | new, `0x2c` | SA-00442, SA-00464, SA-00465 |
  | microcode-2022027 (Feb 2022) | `0x2c` → `0x3c` | SA-00528, SA-00532 |
  | microcode-20220510 | → `0x3e` | SA-00617 |
  | microcode-20220809 | → `0x40` | SA-00657, SA-00614 |
  | microcode-20221108 | → `0x42` | none |
  | microcode-20230512 / -rev2 | → `0x44` | none ("INTEL-SA-NA") |
  | microcode-20230808 | → `0x46` | SA-00828, SA-00836, SA-00837 |
  | microcode-20231114 | → `0x4e` | SA-00950 |
  | microcode-20240312 | → `0x50` | SA-00972, SA-00982, SA-00898, SA-00960, SA-01045 |
  | microcode-20240813 | → `0x52` | SA-01083, SA-01118, SA-01100, SA-01038, SA-01046 |
  | microcode-20250512 | → `0x56` | SA-01153, SA-01244, SA-01247, SA-01322 |
  | microcode-20260210 | → `0x58` | SA-01083 (update), SA-01396 |
  | 20260227 ... 20261110 | unchanged | |

- **inferred**: `0x34` is a revision Intel never published in its Linux repository: the public line went from `0x2c`
  (June 2021) straight to `0x3c` (Feb 2022). Every public fix from `0x3c` on is missing from the image and present at runtime.
  What the image copy still governs is the window before Linux loads `0x58` (firmware phase, SMM set-up). Carrying
  `0x58` in a modified image would close that window; the 105,472-byte update fits the 139,280-byte FFS file that holds
  `0x34` today. Boot Guard does not verify the image (no KM/BPM in the FIT, INVESTIGATION 7.7), so a changed
  microcode file does not stop the boot (**inferred**).

### 2. Intel CSME

- **observed**: `/sys/class/mei/mei0/fw_ver` = `0:15.0.30.1776`; `fw_status` HFSTS1 `0x90000255`, HFSTS3 `0x00000020`
  (bits 4–6 = 2 = Consumer, per fwupd's `FuMeiFirmwareSku`); chipset `HM570` (`lspci`: `8086:438b`). The FTPR manifest in
  the image reads `15.0.30.1776`, date 2021-04-28, SVN 1 (`$MN2` scan, `mn2.py`). PMC firmware in the same region:
  `150.2.10.1017`.
- **observed**: `fwupdmgr security`: `csme11 v15.0.30.1776: Invalid` (`org.fwupd.hsi.Mei.Version`, flag `action-contact-oem`).
  The rule ([fwupd `fu-intel-me-device.c`](https://github.com/fwupd/fwupd/blob/bb7de036c0a2549ad09f913e77f76a4d84afc88f/libfwupdplugin/fu-intel-me-device.c)):
  CSME 15.0 is patched from `15.0.40`.
- **observed** (Intel advisories, "versions before" for the Intel 500 Series Chipset row):

  | Advisory | Published | Fixed in | CVEs on that row |
  |---|---|---|---|
  | [SA-00783](https://www.intel.com/content/www/us/en/security-center/advisory/intel-sa-00783.html) | 2023-08-08 | 15.0.45 | CVE-2022-36392 (8.6), CVE-2022-38102 (7.2), CVE-2022-29871 (6.7) |
  | [SA-00999](https://www.intel.com/content/www/us/en/security-center/advisory/intel-sa-00999.html) | 2024-08-13 | 15.0.49 | CVE-2023-38655, -34424, CVE-2024-21844, CVE-2023-40067, -48361, -35061 |
  | [SA-01152](https://www.intel.com/content/www/us/en/security-center/advisory/intel-sa-01152.html) | 2025-02-11 | 15.0.50 | CVE-2024-26021, CVE-2024-30211 |
  | [SA-01209](https://www.intel.com/content/www/us/en/security-center/advisory/intel-sa-01209.html) | 2025-06-10 | 15.0.50 | CVE-2025-2884 |
  | [SA-01280](https://www.intel.com/content/www/us/en/security-center/advisory/intel-sa-01280.html) | 2025-08-12 | 15.0.52 | CVE-2025-20067 |
  | [SA-01315](https://www.intel.com/content/www/us/en/security-center/advisory/intel-sa-01315.html) | 2026-02-10 | 15.0.55 | CVE-2025-32008 (8.7), CVE-2025-20080, CVE-2025-27708 |
  | [SA-01372](https://www.intel.com/content/www/us/en/security-center/advisory/intel-sa-01372.html) | 2026-08-11, rev. 2026-09-09 | 15.0.56 | CVE-2026-6726 |
  | [SA-01427](https://www.intel.com/content/www/us/en/security-center/advisory/intel-sa-01427.html) | 2026-08-11 | 15.0.56 (row names Tiger Lake-H) | CVE-2026-20715 (8.2), -20708, -20734 |

  Older ones (e.g. SA-00610, SA-00613, SA-00699, SA-00923) are listed by the OEM package below; not re-read here.
  No 2026.4 chipset advisory was found (search on 2026-11-16).
- **inferred**: some of these CVEs concern AMT or Standard Manageability, which a consumer SKU does not run, so not every
  row applies to this board; SA-01315, SA-01372 and SA-01427 name the CSME firmware itself.
- **observed** (OEM primary source): Panasonic's [CF-SV1R CSME package](https://global-pc-support.connect.panasonic.com/dldocs/84804)
  ships CSME `15.0.56.2834` (2026-10-27) against SA-01372 and SA-01427. That is a corporate Tiger Lake UP3 laptop;
  **unknown** whether a consumer-H build of the same version exists from an official source. Intel itself does not publish
  CSME firmware images.
- **observed**: the image has Intel's ME capsule driver (`FmpDxe` `0EAB05C1-...`, "TigerLake CSME System Firmware Device",
  REA `ev_bf6c46c9...`), and the ESRT entry `d1144042-...` reports version 1967856 = `0x1E06F0` = 30.1776 (**inferred**
  to be that driver's entry, since the ME's hotfix and build numbers fit). So the firmware can take an ME update as a
  UEFI capsule, but no signed capsule for this board exists anywhere found.
- **observed**: Monster's official download for the Abra A7 V11.4 lists an "ME (Management Engine)" item, version
  `2113.15.0.2242`, which links to another model's file
  (`https://mnstr-support-files.s3.eu-central-1.amazonaws.com/Drivers/Abra%20Serisi/Abra%20A5/Abra%20A5%20V18.1/12-ME.zip`,
  818,942,232 bytes, S3 ETag `167061a3821c36598c29874f04108515`). Only its zip directory and `V18/mup.xml` were fetched
  (byte ranges, into `/tmp/fwres-monster-me.*`; `mup.xml` SHA-256 `c4d4b66a7c7ddaeadc3cdace412bee441a6ab4a4e9785405915eeec9953e5919`):
  `releasetype DRVR`, version `2130.15.0.2380`, i.e. the **ME driver** package (`SetupME.exe`, `MUP3.zip`). It holds LMS
  partial-update files keyed by base version (`150030_PD_PFU.BIN` ... `150040_PP_PFU.BIN`, dated 2021-07-20), **inferred** to be
  partial updates for AMT components, not a full CSME image. Not downloaded whole, not run.

### 3. GOP, VBT, RST

- **observed**: one GOP driver, in AMI's option-ROM file `A0327FE0-...` (section `380B6B4F-...`); its only version text is
  the UTF-16 string `1064` next to `Intel(R) Graphics Controller` (`ctx.py`). **inferred**: build 1064 of the Tiger Lake
  GOP line, whose major/minor is not stored as text.
- **observed**: VBT `$VBT TIGERLAKE`, BDB 240, 8,607 bytes (file `56752DA9-...`). The same option-ROM file also holds a
  legacy Intel VGA BIOS (`$VBT ICELAKE`, BDB 223, "Build Number: 1003"), a PXE ROM and the RST SATA option ROM
  `17.3.0.4067` (all CSM, unused on a UEFI boot: **inferred**).
- **observed**: RST VMD UEFI driver `18.1.1.5201` (REA `ev_c1a3819b...`).
- **unknown**: newest official builds of these. Intel distributes GOP, VBT tooling and the RST UEFI driver to firmware
  vendors only; the public figures are on community forums, which are not a primary source. **inferred**: on Linux, i915
  replaces the GOP after boot and nothing uses VMD/RST, so a newer build would change at most the boot screen.

### 4. NVMe SSD

- **observed**: `nvme id-ctrl`: `mn` `NX-1TB 2280`, `fr` `VF100C6B`, PCI vendor and subsystem vendor both Realtek (`0x10ec`),
  `frmw` `0x2` (one firmware slot, slot 1 writable), `fwug` 1; `nvme fw-log`: active slot 1, `VF100C6B`. fwupd lists it
  as updatable (nvme plugin) with no release.
- **inferred**: a white-label Realtek reference design. The same `VF100C6B` string appears on drives sold by TeamGroup,
  AGI and ADATA (search results, not primary), and the `NX-1TB 2280` name matches KingSpec's NX series; Realtek and
  KingSpec publish no firmware for it, and the UNH NVMe Integrator's List has only `VF001C0B` (2022).
- **unknown**: whether a newer official firmware exists, and whether this drive came fitted by Avell.

### 5. Webcam

- **observed**: `04f2:b711` Chicony "HD Webcam", fwupd version `97.45` (bcdDevice), DFU plugin, no release (`fwupdmgr get-releases`).
  Chicony uploads to LVFS only on behalf of OEMs, and no OEM of this board is an LVFS vendor (vendor list, 2026-11-16).
- **unknown**: any newer firmware.

### 6. BIOS and EC from sibling boards

- **observed**: DMI `N.1.05AVE04`, 08/30/2022, "Embedded Controller Firmware Revision: 1.13" (`dmidecode -t 0`). ESRT system
  firmware entry `14d67f09-...` version 105, lowest supported 105.
- **observed**: Monster's official pages for the Abra A7 V11.4 and V11.5 list drivers only (latest 2026-04-27), no BIOS or EC.
  CyberPowerPC routes laptop BIOS updates through support only. Avell states (Reclame Aqui replies) that it does not
  publish BIOS files and updates them only at service centres.
- **inferred** (hardware probes and forums, not primary): Avell A70 MOB machines report `N.1.04AVE00` and `N.1.07AVE00`,
  both dated 07/23/2021, so Avell numbers each model separately and `N.1.07` there is not newer than `N.1.05AVE04` here.
  A complaint quotes `1.05MON05` (2021-12-13) as Monster's newest for the Abra A7 V11.4. Unofficial "GM5TGFX BIOS & EC"
  dumps circulate on repair forums and Telegram; none were downloaded (untrusted, not official).
- Conclusion: **no official BIOS or EC newer than this board's was found** (same result as INVESTIGATION 7.1; still
  inferred, since absence on public pages does not prove none exists).

### 7. Capsule update path (new)

- **observed**: four `FmpDxe` drivers: `69585D92` microcode ("Microcode"), `0EAB05C1` CSME, `4E88068B` "TigerLake BtGACM BIOS
  Firmware Device" (Boot Guard startup ACM; ESRT version 69140 = `0x10E14`, **inferred** S-ACM 1.14.20) and `EED54281`
  "TCSS Retimer Device" (version 1; fwupd labels it "Lenovo", **inferred** a quirk mismatch). Each embeds the same
  self-signed CA certificate: `C=TW, O=AMI, OU=CSP, CN=test1, emailAddress=test1@test.com`, valid 2020-10-15 to 2030-10-13,
  RSA 2048, SHA-256 fingerprint `CF:2B:8F:D3:1F:DA:AD:35:3E:5B:A5:E1:57:97:E0:CF:B2:1C:E2:A2:F1:D1:37:6A:02:95:3F:58:61:20:BC:20`
  (`certs.py`, `getcert.py`, then `openssl x509 -inform DER`).
- **inferred**: it is the PKCS#7 trust anchor these drivers check capsules against (it is the only certificate in each).
  If so, these capsules are protected by an AMI test key rather than an OEM key.
- **unknown**: whether that key's private half is public, and which key signs the system firmware capsule (AMI Secure
  Flash: `SecFlashUpdDXE`, key file `5B85965C-455D-4CC6-9C4C-7F086967D2B0`, 60 bytes).

### 8. Other firmware seen (not asked, for completeness)

- **observed**: NVIDIA vBIOS `94.07.3B.00.EA` (`nvidia-smi`): ticket #29. TPM is Intel PTT `600.7.0.0` (part of the ME).
  Wi-Fi/Bluetooth AX201 and the RTL8125 load their firmware from `linux-firmware 20260916-1` at runtime (nothing in flash).
- **observed**: Secure Boot KEK holds only Microsoft KEK CA 2011 next to Uniwill's 2019 KEK and PK; db has the 2023 UEFI CAs;
  dbx is current (`20260402`). A KEK 2023 update would need a payload signed with Uniwill's PK; LVFS has none.

## Commands

```
grep -m1 microcode /proc/cpuinfo                          # 0x58
sudo journalctl -k -b | grep -i microcode                 # updated early from 0x34
sha256sum /usr/lib/firmware/intel-ucode/06-8d-01          # 20b65b22...
cat /sys/class/mei/mei0/fw_ver /sys/class/mei/mei0/fw_status
fwupdmgr get-devices; fwupdmgr get-updates; fwupdmgr security --json
sudo cat /sys/firmware/efi/esrt/entries/entry*/{fw_class,fw_version}
sudo nvme id-ctrl /dev/nvme0; sudo nvme fw-log /dev/nvme0
sudo dmidecode -t 0
uefiextract image.bin all                                 # on a /tmp copy of the backup
python3 -I ucode.py|fit.py|mn2.py|vbtgop.py|certs.py ...  # scripts kept outside the repo, in /tmp/fwres-scripts.*
```

## Open unknowns

1. A consumer-H CSME `15.0.56` image from an official source, and whether moving from SVN 1 to a newer SVN can be undone
   (anti-rollback). This decides whether an ME update stays inside "nothing irreversible" (#28).
2. Newest official GOP/VBT/RST builds: no primary source.
3. NVMe and webcam: no published firmware; whether the SSD is Avell's factory part.
4. Whether the AMI `test1` certificate is the capsule trust anchor, whether its key is public, and what key the system
   firmware capsule trusts.
5. Sibling BIOS/EC newer than `N.1.05`: none found on public pages; only a request to Avell or Monster support could settle it.

## Decisions this surfaces for the owner

- **Microcode**: nothing to do at runtime. Whether a modified image should also carry `0x58` (closes the pre-OS window;
  small gain) is a choice for #35/#36.
- **ME**: whether to pursue CSME `15.0.56` at all, given the only routes are a modified ME region or Intel's update tool
  with an image obtained outside Avell, and given unknown 1 above. Belongs with #28.
- **Vendor requests**: whether to ask Avell (or Monster) support for a newer BIOS/EC/ME package; that would be the only
  official source, and it means contacting them with the machine's details.
