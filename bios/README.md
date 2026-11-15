# bios

BIOS backup and read-only analysis tools, and the written procedures for guarded BIOS changes (tickets #17 to #21).

| File | What |
|---|---|
| `backup.sh`, `BACKUP.md` | Read-only flash backup, twice and compared (#17) |
| `analyze.py`, `nvar.py` | Read-only image analysis: setup forms and NVRAM values for the three targets (#18). Needs `uefiextract` (uefitool-ng) and `ifrextractor` (ifrextractor-rs) |
| `test-analyze.sh` | Runs the analysis on the committed image and diffs it with `evidence/bios/analysis.md` |

No tool here writes to the flash chip. Every write waits on an external programmer and two matching off-machine backups (spec #1, ADR-0002).
