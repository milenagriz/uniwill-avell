---
status: accepted
---

# Direct fan speed goes through the universal fan tables, with Max fan for anything urgent

The spec (#1) named the EC's direct duty registers `0x1804/0x1809` for direct fan speed. This board's EC reports the "universal fan control" feature (`0x078E`), and for such boards TUXEDO's driver loads a custom fan table instead: `0x07C5` bit 7 (split tables) and `0x07C6` bit 2 (enable), with a single zone covering every reachable temperature in `0x0F00-0x0F5F`, whose speed (0-200) becomes the duty. The owner chose this route because the firmware stays in charge of the fans (turning the table off is stock behaviour again) and it needs none of the ramp-up workaround or 50-minute restart timer the direct registers do.

On this board the EC slews the fans towards the table's duty slowly, about 1% of full speed per second. So whenever full speed is needed at once, the driver turns on Max fan (`0x0751` bit 6) on top of the table: when manual mode is entered, until the first duty is written, above the hard temperatures, and whenever both fans are asked for full speed (which keeps the Cooling widget's Max button instant). While the driver itself holds Max fan, the mode reads manual.

## Considered options

- **Direct registers `0x1804/0x1809`**, as in the spec: works only with Max fan on, needs TUXEDO's ramp-up workaround and a periodic restart, and the upstream maintainer calls them unstable. Rejected.
- **Tables only, no Max fan**: a dead daemon or a hot CPU would take about two minutes to reach full speed. Rejected as unsafe (ADR-0001).

## Consequences

- Fan curves in the daemon (#9) see the EC's slew: a step in the duty becomes a ramp. Curves should be written for that, not fight it.
- This lives in the local-only patch `0006`, on top of the upstreamable series.
