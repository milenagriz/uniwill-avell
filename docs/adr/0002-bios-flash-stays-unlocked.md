---
status: accepted
---

# The BIOS flash stays unlocked

This laptop shipped with its BIOS flash chip unprotected: the flash lock is off, the BIOS region is writable, the management engine is still in manufacturing mode, and Boot Guard is not enforcing. We deliberately leave it that way permanently, because the vendor publishes no firmware updates and the unlocked flash is the only route to changing BIOS settings and images ourselves, now and later. The cost we accept is that software running as root could persist in the firmware.

## Considered options

- **Lock it as a final step** once firmware changes are done. Rejected: it would close the door on future changes.
