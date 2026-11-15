# bios

BIOS backup and read-only analysis tools, and the written procedures for guarded BIOS changes (tickets #17 to #21).

No tool here writes to the flash chip. Every write waits on an external programmer and two matching off-machine backups (spec #1, ADR-0002).
