# evidence

Raw firmware evidence from this machine, committed so findings can be re-checked.

| Path | What |
|---|---|
| `acpi/` | ACPI tables (`acpidump` + `iasl -d`): `.dat` binaries and `.dsl` disassembly |
| `ec/` | 4 KiB dumps of the EC RAM window at physical `0xFE410000`: `ec_a`/`ec_b` idle, `ec_load`/`ec_load2` after an all-core load |
| `scripts/ecdump.py` | Hex dump of chosen ranges of an EC dump: `python ecdump.py ec/ec_a.bin 740-7c0` |
| `scripts/loadtest.py` | All-core load run logging package power, frequency, temperatures, fans, EC 0x751 and throttle status; saves an EC dump at the end. Reads `/dev/mem` and an MSR, so run as root. The hwmon indices are those of the boot it was written on |
