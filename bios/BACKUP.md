# BIOS flash backup procedure

Read-only. It takes the whole 32 MiB flash from inside the running laptop, twice, and keeps it only if both reads match.

## Needs

- `flashrom` 1.8 or later (`sudo pacman -S flashrom`)
- root (`sudo`)

## Run

```bash
bios/backup.sh
```

It reads the flash twice (`sudo flashrom --programmer internal --read <file>`), compares the two reads, and only if they match
saves `evidence/bios/bios-<BIOS version>-<date>.bin`, its `.sha256` and the flashrom log. If the reads differ, nothing is saved; run it again.

## Why it cannot write

- The script calls flashrom only with `--read`. There is no `--write`, `--erase` or `--verify` anywhere in it.
- Each read goes to a new file in a fresh temporary directory, and the script refuses to overwrite an existing file.
- Do not edit the script to add any other flashrom operation. Writes are separate, human-only procedures (ADR-0002 and spec #1:
  only with an external programmer in hand and two matching backups stored off the machine).

## Check a backup later

```bash
cd evidence/bios && sha256sum -c bios-N.1.05AVE04-20261115.bin.sha256
```

## Chip and layout

See `evidence/bios/README.md`: Winbond W25Q256 (3.3 V, 32 MiB), region layout, write-access settings.
