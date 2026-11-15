# BIOS flash backup (ticket #17)

| | |
|---|---|
| Image | `bios-N.1.05AVE04-20261115.bin` (33,554,432 bytes, the whole flash) |
| SHA-256 | `baa08b4a7a80efbb2cb3f8d841b4fb9a9728161b93dec2ca0e6eea6d30d52412` (also in `.sha256`) |
| How | `bios/backup.sh` on 2026-11-15: two full reads with `flashrom --programmer internal --read` (flashrom 1.8.0, through the kernel's `spi-intel` MTD driver), byte-identical |
| BIOS version | `N.1.05AVE04` |
| Flash chip | **Winbond W25Q256** (from the kernel's SPI NOR driver: `/sys/bus/spi/devices/spi0.0/spi-nor/` jedec_id `ef4019`, manufacturer `winbond`, partname `w25q256`) |
| Size, voltage | 256 Mbit = 32 MiB (observed); **3.3 V**, since device ID `0x40xx` is Winbond's 3 V W25Q series and the 1.8 V parts are `0x60xx` (inferred from the ID) |
| Package | **unknown**: SOIC-8/16 or WSON-8. Confirm by eye before buying a programmer clip (#19) |

flashrom saw an "Opaque flash chip" because the MTD path hides the chip. The ID above comes from the same chip through
the kernel's `spi-nor` driver. `flash-sfdp.bin` is the chip's SFDP table, read the same way.

## Region layout

From the flash descriptor (`ifdtool -p tgl -d`, saved in `ifdtool-dump.txt`), cross-checked against UEFIExtract NE A75
(`uefiextract-report.txt`):

| Region | Start | End | Size |
|---|---|---|---|
| Flash descriptor | `0x0000000` | `0x0000FFF` | 4 KiB |
| Intel ME (IFWI 1.7) | `0x0001000` | `0x1000FFF` | 16 MiB |
| Device Expansion 2 | `0x1001000` | `0x11FFFFF` | 2 MiB (descriptor only; UEFIExtract shows it as padding) |
| BIOS | `0x1200000` | `0x1FFFFFF` | 14 MiB |
| GbE, Platform Data, EC, other regions | unused | | |

`FLMSTR1` (host CPU/BIOS) is `0xffffffff`: read **and write** access to every region, including the descriptor and the ME.
This matches the flash-protection findings in INVESTIGATION section 7.7 (ADR-0002).

REA `inspect_firmware_regions` (binwalk 3.1.0) over the image: Evidence `ev_10ac21b9c0f7f8d5fb4017c58dc60314f76ddb8830c3046ce30fc2317bfb479d`.
