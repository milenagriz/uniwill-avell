# Which EC is on this board, and how can its firmware be obtained without writing to it? (#27)

Research for [#27](https://github.com/milenagriz5-dotcom/uniwill-avell/issues/27), map [#23](https://github.com/milenagriz5-dotcom/uniwill-avell/issues/23).
Read-only throughout: nothing was written to the EC, the SPI flash, an EFI variable or an MSR, and no new kind of EC access was
made. The only sources on this machine are the committed flash backup (`evidence/bios/`), the committed EC RAM dumps
(`evidence/ec/`), the ACPI tables (`evidence/acpi/`), `dmidecode` and `fwupdmgr get-devices`. 2026-11-15.

## Answer

**The EC is an ITE 8051-core embedded controller running ITE's EC firmware framework "EC-V14.6", OEM version 1.13. Its firmware
is already in hand: this board's own BIOS region carries it.** The committed backup image holds four 192 KiB EC images (EC 1.13,
one per board variant) that AMI's ReFlash driver writes into the EC during a BIOS flash update. The running EC reports version
byte `0x4D` (= 1.13) and variant byte `0x03`, and exactly one of the four images carries that pair (`207F94A8`, image offset
`0x18BA2FC`). So the firmware can be had with no access to the EC at all (observed), and it is very probably what the EC runs
(inferred: version and variant match, the bytes are not compared).

- **Chip**: ITE (observed: ITE's e-flash signature, "ITE EC-V14.6", 8051 code, ITE's flash-mode protocol in the BIOS). The exact
  part is **unknown**; the 192 KiB image does not fit a 128 KiB IT5570E-128, so an IT5570E-256-class part (256 KiB e-flash) is the
  likely candidate (inferred).
- **Where the firmware lives**: in the EC's own embedded flash, not in the SPI flash's EC region, which is unused (inferred from the
  BIOS's update routine, which programs and verifies 192 KiB through the EC itself). Copies of all four variants live in the BIOS
  region of the W25Q256 (observed).
- **How to prove a copy matches**: the running EC's version and variant bytes at EC RAM `0x0454`/`0x0455` (already in the
  committed dumps, and the version is what SMBIOS reports) against the `ECVER` record inside each image (done here). A byte-for-byte
  proof needs a read of the EC's flash, which is a new kind of EC access: proposed below for the owner, not done.

Two things this surfaces for the rest of the map: the BIOS's flash-update path **re-flashes the EC** (also at an equal version),
which collides with the map's "no EC flashing" ceiling; and EC firmware analysis is no longer blocked on obtaining the firmware,
only on an 8051 disassembler.

## Findings

### 1. The running EC: version 1.13, variant 3

| Finding | Status | Source |
|---|---|---|
| SMBIOS type 0 reports "Embedded Controller Firmware Revision: 1.13" (raw bytes `0x16`/`0x17` = `01 0D`) | observed | `sudo dmidecode -t 0`, `sudo dmidecode -t 0 -u` |
| The BIOS fills those two bytes from one EC byte: `SmbiosBoard` sends EC commands `0xA3 0x04`, `0xA2 0x54`, `0xA4` on ports `0x66`/`0x62`, reads one byte, and stores `byte >> 6` as major and `byte & 0x3F` as minor at type 0 offsets `0x16`/`0x17` | observed (static) | REA `analyze_function` on `SmbiosBoard` (SHA-256 `89a0f0be…`): `sub_814` Evidence `ev_7193a630de4851f124cc2ef47951cf1f1fb509a6061b9c8637f9d5a644b2bb78` (pseudocode view `ev_ab724d33f752c13bbf34269e950343a64f89d78bc5dc194681ef29b25e81d06d`), `sub_8d0` Evidence `ev_fadbb6b7c5ad3a7bd1c2568738c79cf2e45873eb961c3bbbf455bb0a445eeb01` |
| `0xA3` sets the EC RAM address high byte, `0xA2` the low byte, `0xA4` reads the byte there; so the byte is EC RAM `0x0454` | inferred: the same three-command helper reads `0x04xx` with a variable low byte in ReFlash (`sub_2b7c`, Evidence `ev_996f2bf5eeec0a7bcc4af5f600986bbaa32553f164ed74d6e4ed5601b5ab9815`), and the value matches (next row) |
| EC RAM `0x0454` = `0x4D` (`0b01_001101` → 1.13), `0x0455` = `0x03`, `0x047D` = `0x00`, identical in all four dumps | observed | `evidence/ec/ec_{a,b,load,load2}.bin` (byte offsets `0x454`, `0x455`, `0x47d`) |
| The ROM ID field the upstream driver names (`0x0770`–`0x077D`) is all `0xFF` on this EC | observed | same dumps |
| fwupd lists no EC device (only the SPI controller, the system firmware capsule `105`, ME 15.0.30.1776, SSD, peripherals) | observed | `fwupdmgr get-devices` |

### 2. Four EC images inside this board's BIOS region

`uefiextract evidence/bios/bios-N.1.05AVE04-20261115.bin all` (UEFIExtract, run on a copy in a `mktemp -d` directory) shows four
Freeform files in DXE volume `4F1C52D3-D824-4D2A-A2F0-EC40C23C5916`, each with one Raw section of 196,608 bytes (`0x30000`).
Each image is `image[offset : offset + 0x30000]` of the committed backup:

| FFS file GUID | Image offset | SHA-256 of the 192 KiB image | `ECVER` record: version, variant, `0x047D` byte |
|---|---|---|---|
| `207F94A8-238E-11E8-B467-0ED5F89F718B` | `0x18BA2FC` | `39f90a1991b3f2e2791067c55b00333580ff15190dfc1a8a0ff67a73cbf713fb` | `4D 03 00` (1.13, variant 3) |
| `207F989A-238E-11E8-B467-0ED5F89F718B` | `0x18EA31C` | `22a4e0227e370572a098dec6215e7e38f90411a4b7e98643861a84ac1f8d5f38` | `4D 01 00` (1.13, variant 1) |
| `207F9A0C-238E-11E8-B467-0ED5F89F718B` | `0x191A33C` | `cd53661b7b873a5e473c89aee1bc9117b425b407b21b0e9e4644175b6c1967e7` | `4D 00 00` (1.13, variant 0) |
| `B3396D33-08A2-4AD0-9508-FD58808D7F68` | `0x194A35C` | same as `207F9A0C` | `4D 00 00` |

| Finding | Status | Source |
|---|---|---|
| Each image starts with 8051 `LJMP` vectors and carries ITE's e-flash signature at `0x40` (`A5 A5 A5 A5 A5 A5 A4 95 85 12 5A 5A AA 7B 55 55`), then the string "ITE EC-V14.6" | observed | hex dump of the images; REA `inspect_firmware_regions` (binwalk 3.1.0) on `207F94A8` finds no known container signature: Evidence `ev_39bf8995b9e8fa49e289e8c0fd13ca9a588dadb06e4bc750e741d149c8930fd6` |
| The images are three 64 KiB banks. Variant 0 and 3 differ in 6 bytes (GPIO setup bytes at `0x5D8F`, `0x8D6A`–`0x8D73`, and the variant byte); variant 1 differs widely in banks 0–1. Bank 2 (`0x20000`–`0x2FFFF`) is identical in all four and is a separate image named "ITE8850-PD" 0.02 with "ProtoVer/DriverVer/UsbPdVer:01.00" | observed | byte diff of the four images |
| An `ECVER` marker at image offset `0x123A9` is followed by the version byte at `0x123AE` (`0x4D`, which happens to read as "M"), the variant byte and one more byte | observed | hex dump |
| At start-up the EC firmware copies those three bytes to EC RAM `0x0454`, `0x0455` and `0x047D`: code at image offset `0x107B3` (bank 1) is `CLR A; MOV DPTR,#0xA3AE; MOVC A,@A+DPTR; MOV DPTR,#0x0454; MOVX @DPTR,A`, and the same for `0xA3AF` → `0x0455` and `0xA3B0` → `0x047D`; `0xA3AE` in bank 1 is image offset `0x123AE` | observed bytes, hand-decoded 8051 (REA has no 8051 provider here); "at start-up" is inferred |
| **The running EC's `4D 03 00` matches only `207F94A8`** | observed (version and variant); that its code equals `207F94A8` byte for byte is inferred |
| "EC-V14.6" is ITE's firmware-framework version, not the OEM version (1.13) | inferred: the same "EC-V14.6" string appears on other Tongfang boards with other OEM versions (sibling sources below) |
| Bank 2 is firmware for an ITE IT885x USB-PD controller | inferred from the name only; ITE sells the IT885x as a USB PD 3.0 controller ([ITE product page](https://www.ite.com.tw/en/product/cate6/IT885X)). Whether such a chip is on this board, and whether the EC loads it, is **unknown** |

### 3. The BIOS flashes and verifies the EC itself (AMI ReFlash)

The four GUIDs are referenced only by `ReFlash` (AMI's flash-update driver, SHA-256 `3bf2fd4708d353c40f2e7c6a04aa4fc6821af0aa8c4da300c57b6b4d9ccd7138`),
from a pointer table at `0xDF50`. Its strings include "Updating EC Firmware", "Erasing...", "Programming...", "Verifying...",
"EC NOT Found!!", "EC F/W: %01d.%02d. It's the latest version" and a recovery-form option "EC FIRMWARE Update — Set this option to
update EC area of the firmware". Static analysis (REA, Hopper provider; addresses are ReFlash RVAs):

| Finding | Status | Source |
|---|---|---|
| The EC update routine (`sub_8248`) reads the running EC's version (`0x0454`) and variant (`0x0455`) with the `A3/A2/A4` commands, aborts with "EC NOT Found!!" unless the version is `0x01`–`0xFE`, then scans each of the four images in the *new* flash image for "ECVER" and picks the one whose variant byte equals the running EC's | observed (static) | REA `analyze_function` `0x8304` Evidence `ev_38b967e18c903d86434372174e560c0f63b7177bc5fa7ed70331e3ff70b3a1ca`; `sub_2b7c` above; xrefs to the GUIDs `ev_f0b405e3…`, `ev_a97fec5e…`, `ev_08023ba1…`, `ev_5251c483…` |
| It programs when the running version is **lower than or equal to** the image's (`cmp %cl,0x38(%rbp); jbe`), and prints "It's the latest version" only when the running EC is newer | observed (static); that an equal-version re-flash really happens on the device is inferred |
| Flash protocol, all through the ACPI EC ports `0x66`/`0x62`: command `0xDC` enters flash mode and the EC must answer `0x33`; then `0x01` (chip select on), `0x02`+byte (SPI command), `0x03`+byte (SPI data), `0x04` (read one SPI byte back on `0x62`), `0x05` (chip select off); `0x9F` JEDEC ID checked against a 12-entry table whose first entry is manufacturer `0xFF` (parameters `0x400`, `0x100`) ahead of eleven external SPI flash vendors (`0xBF`, `0xEF`, `0xC2`, `0xC8`, …); `0xFC` leaves flash mode; `0xFE` is sent after a successful update | observed (static) | `sub_86dc` `ev_48722903bdacb9eb7d944e9d0708a07aec04da5d49143481ea70412103f46236`, `sub_85f0` `ev_c99dfda8911973f985684b4a908d0e37f13d138169d20ed0f1ce9898cd4166ea`, `sub_8f28` `ev_69f713f306cda1aaf6d3c0d1c343cc56517af8347a869f8d0a73396030695f82`, `sub_8f50` `ev_91fbf04a9aee6ae9549b575a0c99a29896e4771f36d90d14d9f213b6e1fc1479`; table at ReFlash `.data` `0xD3D0` |
| "Verifying..." reads the whole `0x30000` bytes back with SPI `0x0B` (fast read) and `0x04` per byte, comparing against the image: so the EC's flash is readable by the host through this protocol, and it is 192 KiB or more | observed (static) | `sub_87d0` `ev_e03abe0edd8f67b180bf72aea8667783208acafdd2de364b5dc47d32f1fd1446` |
| The flash being the EC's own e-flash, not a separate SPI chip | inferred: the SPI flash has no EC region, the ID table lists `0xFF` first (not a JEDEC vendor), and System76 uses the same protocol on ITE ECs with internal flash (next table) |
| The routine runs as a ReFlash hook (pointer at `.data` `0xD608`), i.e. during an AMI flash update (recovery or capsule); whether the "EC FIRMWARE Update" option gates it is **unknown** | inferred | data reference to `0x8248` at `0xD608` |

The whole REA session (74 records, including the Evidence above) is exported to `evidence/bios/rea-ec-firmware-access.json`
(SHA-256 `4b29ed45d5874b7d3105f77e65086ecd756d209439a92c2eb4b4fbcf721c5f35`); the `local_path` fields point at the scratch copies in `/tmp`.

### 4. The chip, from primary sources

| Finding | Status | Source |
|---|---|---|
| The IT5570 comes as IT5570E/VG-128 (128 KiB embedded flash) and IT5570E/VG-256 (256 KiB), 8051 core, LPC and eSPI host interface | observed in the datasheet | ITE "IT5570E-128/IT5570VG-128 IT5570E-256/IT5570VG-256 Embedded Controller Specification for A Version" V0.3.1, [archive.org copy](https://ia800805.us.archive.org/18/items/it-5570-a-v-0.3.1-u/IT5570_A_V0.3.1_U.pdf) (SHA-256 `01666ebe44ece4a436b5ffaad40c9570c17f0b3480862caf224953585c59102f`), feature list and sections 6.1, 7.12.4, 7.17.4 |
| Host-side chip ID: Super I/O config registers `0x20`/`0x21` = `0x55`/`0x70`; EC-side copy `ECHIPID1`/`ECHIPID2`/`ECHIPVER` at EC address `0x2000`–`0x2002`, "accessible only by EC" | observed in the datasheet | sections 6.3.2.2–6.3.2.3 and 7.17.4 |
| PWM registers sit at EC address `0x1800`, duty cycles `DCR0`–`DCR7` at `0x1802`–`0x1809`: the upstream driver's `EC_ADDR_PWM_1_WRITEABLE` `0x1804` / `0x1809` are ITE's `DCR2` / `DCR7` | observed in the datasheet; that this proves an IT557x specifically is inferred (other ITE ECs share the map) | section 7.12.4, Table 7-23; `driver/src/uniwill-acpi.c` lines 331–339 |
| System76's `ecflash` implements the same protocol for ITE ECs (IT8587 and IT5570): `start()` = `0xDC` expecting 51 (`0x33`), follow mode `1`–`5`, `read()` = SPI `0x0B` per 64 KiB, `stop()` = `0x95`, `0xFC`; `examples/read.rs` dumps the whole flash to `read.rom`. It probes the chip ID through `0x2E`/`0x2F` index `0x20`/`0x21` and accepts `0x8587` and `0x5570`. Its `size()` stops at 128 KiB, so as written it would miss bank 2 of this EC | observed in source | [system76/ecflash](https://github.com/system76/ecflash) at `78d36fab85ada3b871427433540f6549b659ab66`: `src/flasher.rs`, `src/flash.rs` (`EcFlash::new`, `size`), `examples/read.rs` |
| The physical marking of the EC on this board | **unknown**: no photo or schematic text found; a GM5TGFX schematic ("Z3 AIR GM5TGFX Rev 1.2 11th HM570") and boardview exist on repair sites, behind downloads, not fetched |

### 5. Sibling boards (reference only, nothing downloaded)

| Source | What it says | Bearing on this board |
|---|---|---|
| [Win-Raid: Tongfang GM5TGFO](https://winraid.level1techs.com/t/request-unlocked-tongfang-gm5tgfo-laptop-bios/96204) | EC blob extracted with UEFIExtract from the vendor capsules at the same volume GUID `4F1C52D3-…` as a Raw section, 98,304 bytes, "ITE EC-V14.6"; running EC 1.16.09, capsule EC 1.33.09; poster guesses IT8528E from a file name | same packaging scheme (sibling, untrusted); the chip guess conflicts with the datasheet evidence above and is not adopted |
| [ElDavoo/tongfang-gm7mg7p-re](https://github.com/ElDavoo/tongfang-gm7mg7p-re) (Uniwill GM5MG7Y) | ITE 8051 banked EC firmware "V14.6" taken from the vendor BIOS/EC package; treats the "ITE8850-PD" image as a separate target | useful prior art for the analysis ticket; not a fact about this board |
| [Monster Abra A7 V11.5 downloads](https://support.monsternotebook.com/tr/product/abra-a7-v115-173-oyun-bilgisayari/drivers-and-downloads/) | no BIOS or EC item listed | no sibling EC package found from the listed siblings (bounded search) |
| Forum and Telegram posts offering "GM5TGFX MB V1.2 BIOS & EC" | unofficial dumps | untrusted; not needed now that the board's own image carries the EC |

## Routes to the firmware, their risk, and how a copy is proven

| Route | Risk to the running EC | Proof it matches this EC | Status |
|---|---|---|---|
| **A. Extract from the committed BIOS backup** (section 2) | none: file only | `ECVER` version and variant against EC RAM `0x0454`/`0x0455` (match for `207F94A8`) | **done** |
| B. Sibling vendor packages | none to the EC (download, analyse in isolation, never flash) | same `ECVER` check; a sibling image is another variant or version by construction | not needed; only useful to compare versions |
| C. Physical look at the EC's marking (with the cover off, e.g. during the #19 flash-package check) | none: no electrical access | settles the part number, not the content | proposed for the owner |
| D. Super I/O chip ID: write index `0x20`/`0x21` to port `0x2E`, read `0x2F` (what `ecflash` does) | low, but a new kind of access: it writes the index port the EC and SMM firmware may also be using; expected `0x55 0x70` for an IT5570 | settles the chip family only | proposed for the owner |
| E. **Read the EC's flash through flash mode** (`0xDC` … `0x0B` … `0xFC`), the protocol of ReFlash's "Verifying..." and `ecflash`'s `read.rs` | **high**: while in flash mode the EC's normal firmware does not run (fans, thermal shutdown, charging, keyboard and power button handled by the EC stop; inferred from the protocol, the 8051 runs a RAM loop). The Linux ACPI EC driver also uses ports `0x62`/`0x66` and would interleave with it; a stray byte in follow mode reaches the SPI command stream. If the EC does not leave flash mode, only a full power-off (AC and battery) is expected to reset it | byte-for-byte compare with `207F94A8` | **proposed, not done**; only with the owner's approval |
| F. ITE debug/ISP interface with an external adapter | needs the board opened and wires on the EC's debug pins; the EC is held while it is read | byte-for-byte | out of reach without hardware; listed for completeness |

If E is ever approved, it should not run from the live desktop: a minimal environment where nothing else touches `0x62`/`0x66`
(for example a UEFI-shell or initramfs tool, or Linux booted with the ACPI EC kept quiet), AC connected, idle, a read-only tool
that cannot issue erase (`0xD7`) or program (`0xAD`), `size` set to `0x30000`, and leave with `0x95`, `0xFC` as `ecflash` does.
A full power-off is the rollback.

## Decisions for the owner

1. **Is the version-and-variant match enough** to treat `207F94A8` as this EC's firmware for analysis (route A), or is a
   byte-for-byte proof wanted (route E, high risk)?
2. **Routes C and D**: approve a look at the chip marking when the cover is next off, and/or the Super I/O chip-ID read?
3. **BIOS flashing and the EC**: ReFlash re-flashes the EC even at an equal version. Under the map's "no EC flashing" ceiling,
   BIOS writes need a path that cannot reach this hook (the external programmer, which writes the SPI chip and never talks to the
   EC), and a modified image should leave the four EC images untouched. Whether a fwupd capsule (`system firmware 105`) or AFU
   would run the hook is unknown.

## Open unknowns

- The exact EC part number and its flash size (IT5570E-256 is the inference).
- Whether the EC's flash equals `207F94A8` byte for byte (needs route E or F).
- Which flash paths run ReFlash's EC hook: recovery, fwupd capsule, AFU, the "EC FIRMWARE Update" option.
- What bank 2 ("ITE8850-PD") is for: an IT885x PD controller on the board, loaded by the EC, or unused.
- What the four variants map to (variant 3 is this board; 0 and 1 are other SKUs of the same BIOS, inferred).
