# Can a vBIOS raise the RTX 3050's power limit on this laptop? (#29)

Research for [#29](https://github.com/milenagriz5-dotcom/uniwill-avell/issues/29), map [#23](https://github.com/milenagriz5-dotcom/uniwill-avell/issues/23).
Read-only throughout: nothing was flashed or written to the GPU, the EC, the SPI flash or any EFI variable. 2026-11-15.

## Answer

**No, not usefully.** The installed vBIOS (`94.07.3B.00.EA`) already allows a 95 W maximum (observed), which is NVIDIA's top
configuration for this GPU: NVIDIA rates the RTX 3050 Laptop GPU at 35–80 W, and OEMs quote the maximum as TGP plus
Dynamic Boost, so 80 W + 15 W = 95 W (inferred). The stack already reaches it: the enforced limit hit 95 W under load with the
TGP offset at 20 W and `nvidia-powerd` running (INVESTIGATION 9.1). A signed vBIOS from another RTX 3050 laptop can at most move the
*default* limit from 60 W towards 80 W, which the TGP offset already does in software. No higher ceiling exists for this chip
within NVIDIA's signed images (inferred, bounded search), and a modified image is refused by the GPU's signature check (inferred).

So a vBIOS flash is all risk for no ceiling gain. What would help is knowing whether the GPU ever *uses* 95 W: the only load
run so far drew 65–69 W at 70 °C and was not power-limited (INVESTIGATION 9.1). That needs a power-bound GPU load (see Open unknowns).

If a flash were ever wanted anyway: the tool is NVIDIA's nvflash (closed source, with the nvidia modules unloaded), the
verification is a byte-identical `--save` read-back plus `nvidia-smi`, and the recovery is either a software re-flash (the laptop
boots on the Intel GPU, so a broken dGPU vBIOS should not stop the machine booting, inferred) or an external programmer on the
GPU's own EEPROM chip, whose part, package and voltage are unknown until the board is opened.

## Findings

### 1. The GPU and its vBIOS

| Finding | Status | Source |
|---|---|---|
| GPU `10de:25a2` (GA107M, RTX 3050 Laptop), subsystem `1d05:1140` (Tongfang), part `25A2-725-A1`, Board ID `0x100` | observed | `lspci -vvv -nn -s 01:00.0`, `nvidia-smi -q` |
| vBIOS `94.07.3B.00.EA`, build strings "JE4736 SKU 10 VGA BIOS", "Copyright (C) 1996-2021 NVIDIA Corp.", dates 03/29/21 and 05/09/21 | observed | `strings` on the dump; `/proc/driver/nvidia/gpus/0000:01:00.0/information` |
| InfoROM `G001.0000.03.03`; driver 615.78.08 (open kernel module) | observed | `nvidia-smi -q` |
| Power: default 60 W, maximum 95 W, minimum 1 W; enforced 80 W at idle today (TGP offset 20 set by the stack) | observed | `nvidia-smi --query-gpu=power.limit,power.default_limit,power.max_limit,enforced.power.limit --format=csv` → `[N/A], 60.00 W, 95.00 W, 80.00 W` |
| Temperatures from the vBIOS: target 87 °C, slowdown 97 °C, shutdown 100 °C, max operating 105 °C; max graphics clock 2100 MHz | observed | `nvidia-smi -q -d TEMPERATURE,CLOCK` |
| The internal panel runs on the Intel GPU (`boot_vga`: `00:02.0` = 1, `01:00.0` = 0; "Display Active: Disabled") | observed | `/sys/bus/pci/devices/*/boot_vga`, `nvidia-smi -q` |
| fwupd offers no update for the GPU: no NVIDIA device is listed. The five ESRT capsule entries are the system firmware (105), `69585d92` (`UcAcmFmp` in the NVAR store) and three unnamed ones | observed; the unnamed three are unknown | `fwupdmgr get-devices --json` |

### 2. The vBIOS dump (new evidence)

`evidence/gpu/vbios-rom-sysfs.bin`, 164,352 bytes, SHA-256 `c3c2fcf3dd56272a1d1a7a14ca932a85d7d7cd702b8de4fa54a99cc1b4a813c6`, read with:

    dev=/sys/bus/pci/devices/0000:01:00.0
    echo 1 | sudo tee $dev/rom; sudo cat $dev/rom > vbios-rom-sysfs.bin; echo 0 | sudo tee $dev/rom

REA `inspect_firmware_regions` (binwalk 3.1.0) over it: Evidence `ev_4e89a42f5e3c7e889728391d426daa70f518c90b18d4e0318cee9262347907de`
(only the NVIDIA copyright signature; binwalk has no NVIDIA vBIOS parser). Structure from `python -I evidence/scripts/vbios.py evidence/gpu/vbios-rom-sysfs.bin`,
which follows nouveau's parser (`drivers/gpu/drm/nouveau/nvkm/subdev/bios/{base,bit,image,npde,power_budget}.c`, torvalds/linux master):

    image 0x00000: type 0x00 ven 10de dev 25a2 PCIR size 0xfe00 last 0 | NPDE size 0xfe00 last 0
    image 0x0fe00: type 0x03 ven 0000 dev 0000 PCIR size 0x18400 last 1 | NPDE size 0x18400 last 0
    chain continues past the end of the file at 0x28200: the extended image is missing
    first 1d05 (Tongfang) subsystem ID in the image: 1d05:1140 at 0x1a0
    BIT at 0x1b0, 18 tokens
    BIT 'P' v2: 49 non-zero pointers, 0x4234..0x774e1; 46 point past the end of this file
    power budget table pointer (P+0x2c): 0x70744, outside this file

| Finding | Status |
|---|---|
| The dump holds the legacy x86 image (63.5 KiB) and the UEFI GOP image (97 KiB) | observed |
| It is **not** the whole vBIOS: the GOP image's PCIR says "last", but NVIDIA's NPDE extension says "not last", so an extended image follows on the chip. The kernel's sysfs `rom` stops at the PCIR flag, nouveau follows the NPDE flag (`image.c: nvbios_imagen`) | observed (flags); the extended image's size is unknown |
| The power and performance tables are in that missing part: 46 of the 49 BIT `P` pointers, including the power budget table nouveau reads at `P+0x2c`, point past the dump. So the vBIOS's own power table (base, maximum, Dynamic Boost entries) **cannot be read from this dump** | observed |
| The image carries subsystem ID `1d05:1140` at `0x1a0`, the same as the PCI header | observed |
| The image contains the string "BIOS Certificate Check Failed!!!": the GPU checks the vBIOS signature, so a hand-edited power table would not run | string observed; enforcement inferred |
| The vBIOS lives on the GPU's own EEPROM, not in the system BIOS: no NVIDIA option ROM (`PCIR` with vendor `10de`), no `94.07.` version and no `JE4736` string in any of the 5,827 bodies UEFIExtract unpacked from `evidence/bios/bios-N.1.05AVE04-20261115.bin`. The only NVIDIA content there is the Optimus ACPI tables | observed (bounded to what UEFIExtract decompresses) |
| ACPI `\_SB.PC00.PEG1.PEGP._ROM` (SSDT10) serves up to 192 KiB from memory region `NVHM` at `NVHA` (in the SA NVS area, `SANB` = `0x737E4C98` + 209). The firmware must fill it at boot from the GPU's chip. Reading it through `/dev/mem` is refused (`dd: Operation not permitted`, `CONFIG_STRICT_DEVMEM`), and 192 KiB is too small for the full image anyway | observed; how the buffer is filled is inferred |

### 3. How the platform ties into the GPU's identity and power

| Finding | Status | Source |
|---|---|---|
| The system BIOS's DXE driver `OemDgpuBoardIDDxe` (`C92E82A5-AD25-4800-9422-CBE8E7A30477`, PE32 SHA-256 `325283d4…6235`) finds the NVIDIA GPU behind the PEG port, maps its **device ID** to a Tongfang subsystem ID (`25a2`/`25a0` → `1d05:1140`, `25a0` → `1141`, `2520` → `1146`, `249f` → `1146`, `249d` → `1117`, `249c` → `1148`, `1f9d` → `114a`), stores it in the variable `UniWillVariable`, and writes it into the GPU's config space at offset `0x40` (NVIDIA's subsystem-ID write register) | inferred from decompilation (REA `decompile_all`, Hopper, Evidence `ev_08251193c1303025b6770e5c9e53e9e4149633a74daba4338fff3e48ff5501ff`: functions `0x7b0`, `0x928`, handlers `0xb00`–`0xc40`) |
| The same driver also sends the EC a GPU-type value through the EC command ports `0x62/0x66` (function `0x6e8`, value with bit `0x40`) | inferred from decompilation; what the EC does with it is unknown |
| Config space today: `0x2c` and `0x40` both read `05 1d 40 11` | observed (`sudo lspci -s 01:00.0 -xxx`) |
| So the platform keys on the **device ID**, not on the vBIOS: a cross-flashed vBIOS for another `25a2` laptop would still be shown to the OS as `1d05:1140`, and the EC/ACPI side (power table selection, TGP offset through `T1WR`/`CTWA`, Dynamic Boost through `NPCF`) would not notice the change | inferred |
| The TGP offset and Dynamic Boost come from the platform (EC/ACPI `NPCF`) and are applied by the graphics driver within the vBIOS's limits; the 95 W maximum reported by `nvidia-smi` is the vBIOS's ceiling, and the enforced limit stopped there with 60 + 20 + 25 W requested (INVESTIGATION 9.1) | inferred from the observed clamp at 95 W |

### 4. Is there a better signed vBIOS?

| Finding | Status | Source |
|---|---|---|
| NVIDIA rates the RTX 3050 Laptop GPU at "35 - 80 W" GPU Subsystem Power (also the 3050 Ti; the 3060 Laptop is 60–115 W) | observed (NVIDIA's page) | https://www.nvidia.com/en-in/geforce/laptops/compare/30-series/ ("GPU Subsystem Power (W)" row), fetched 2026-11-15 |
| OEMs quote the maximum as TGP + Dynamic Boost (ASUS: Zephyrus G15 GA503QM "80W TGP, 15W Dynamic Boost, 95W Max GPU Power") | observed (OEM page) | https://rog.asus.com/articles/rog-gaming-laptops/clarifying-geforce-rtx-30-series-gpu-power-in-asus-and-rog-gaming-laptops/ |
| This machine's 60 W + 20 W TGP offset = 80 W, + 15 W Dynamic Boost = 95 W: already NVIDIA's top configuration for the chip | inferred |
| No RTX 3050 Laptop configuration above 95 W was found; review sites list "RTX 3050 (Laptop, 95W)" as the top variant | inferred, secondary sources, bounded search (e.g. https://www.laptopmedia.com/video-card/nvidia-geforce-rtx-3050-laptop-95w/) |
| No Tongfang (`1d05`) RTX 3050 Laptop vBIOS was found in TechPowerUp's collection by web search; other OEMs' `94.07.3B.xx.xx` images exist (e.g. Asus `94.07.3B.40.0B`, Dell `94.07.3B.40.6E`, Gigabyte `94.07.3B.80.66`) | inferred: search snippets only. TechPowerUp's database answers 403 to automated fetches, so it was not browsed and nothing was downloaded |
| What another OEM's image could change is the **default** (base) limit, e.g. 80 W base instead of 60 W + 20 W offset. That only matters where the offset path is not applied (no driver, other OS); this stack applies it already | inferred |
| Cross-flash risks specific to laptops: memory straps, GPIO and power-sequencing tables, the display table (which outputs the dGPU drives) and the board's VRM design all belong to the other OEM's board. A wrong image on an RTX 3050 laptop is reported to end in error 43 (Dell G15 5515 owner) | inferred, owner reports (https://www.badcaps.net/forum/troubleshooting-hardware-devices-and-electronics-theory/troubleshooting-laptops-tablets-and-mobile-devices/3568504-dell-g15-5515-error-43-after-incorrect-vbios-flash-on-rtx-3050) |
| Thermals: the CPU side of the shared cooler is saturated (INVESTIGATION 7.4); the GPU reached only 70 °C at 65–69 W (9.1), far from its 87 °C target. Whether the GPU side has headroom at 95 W is unknown | observed (cited), headroom unknown |
| The GPU VRM's rating on this board | unknown: needs the board schematic or a look at the parts |

### 5. Tool, verification and recovery (if a flash is ever decided)

| Step | What | Status |
|---|---|---|
| Tool | NVIDIA's nvflash for Linux (closed source, distributed by TechPowerUp). It refuses to run while the nvidia kernel modules are loaded; it checks device ID, subsystem ID (override `-6`) and board ID (no override on Turing and later in the stock tool; patched builds exist) | inferred, forum and guide reports (search 2026-11-15); not run here |
| Full backup | `nvflash --save` reads the whole EEPROM including the extended image this dump lacks. It is a read, but it means running an untrusted proprietary tool as root with raw access to the GPU, and unloading the nvidia modules | owner decision |
| Verification | byte-identical `--save` read-back after writing; `nvidia-smi -q` version, power limits, temperature limits; a load run as in INVESTIGATION 9.1 | inferred |
| Recovery, soft | the internal panel and firmware run on the Intel GPU, so the laptop should still boot with a broken dGPU vBIOS and nvflash could write the backup back, if the GPU still enumerates | inferred |
| Recovery, hard | an external programmer on the GPU's own SPI EEPROM. Part number, package and voltage (1.8 V or 3.3 V) are unknown until the board is opened; the clip for the BIOS chip (#19) may not fit it | unknown |

## Open unknowns

1. **Does the GPU ever use its 95 W?** Needs a power-bound GPU load (for example a CUDA stress tool or a heavy game) on AC with
   the TGP offset at 20 and `nvidia-powerd` running, logging power, enforced limit, clocks, temperature and the
   "SW Power Cap" / thermal throttle reasons. If it sits at 95 W and power-capped, more power could help; if it is thermal- or
   voltage-limited first, no vBIOS would help.
2. **The vBIOS's own power table** (base, max, Dynamic Boost entries): in the extended image, which neither sysfs nor the ACPI
   `_ROM` buffer exposes. Only a full EEPROM read (`nvflash --save`, or nouveau's `vbios.rom` in debugfs with nouveau bound instead
   of nvidia) gets it.
3. **GPU EEPROM chip** (part, package, voltage), the GPU VRM rating and whether the external ports are wired to the dGPU: board inspection.
4. **What the EC does with the GPU-type value** `OemDgpuBoardIDDxe` sends it: EC firmware question (the map's EC firmware fog).
5. **The three unnamed ESRT capsule entries** (`d1144042-…`, `4e88068b-…`, `eed54281-…` "Lenovo 0.0.1"): which devices they are.

## Decisions for the owner

- Close the "vBIOS flash" fog as **not worth doing**: the ceiling is already the chip's maximum and the stack reaches it.
- Whether to take a **full vBIOS backup with nvflash** anyway (read only, but a proprietary root tool and an nvidia module
  unload), so the image is on file before any later GPU work. Optional: nothing on the map needs it now.
- Whether to run the **power-bound GPU load** (unknown 1), which decides if GPU power is worth pursuing at all.
