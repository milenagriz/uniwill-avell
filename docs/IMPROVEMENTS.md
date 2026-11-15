# Improvements and known limits

Most of the earlier ideas here (manual mode starting at full speed, intermediate fan speeds, the untested
features, upstreaming) are now part of the spec (#1) and its tickets. What is left:

## 1. Rebase when the kernel moves past 7.2.15

`driver/src/` is frozen. On a kernel update that changes this driver, either:
- Take the new kernel's `uniwill-acpi.c` (from the stable tree plus the matching Omarchy `08xx` patches),
  apply our patches from `driver/patches/`, fix any rejected hunks, bump `PACKAGE_VERSION` and `MODULE_VERSION`, then
  run `scripts/install.sh`; or
- Turn this into a small rebase script that downloads the right sources for `uname -r` automatically.

## 2. Notification for Max fan changed by a hotkey

Uniwill firmware usually reports `FAN_BOOST_STATUS` (`0x0768` bit 2). Reading `pwmN_enable` already follows
the EC, but an input or uevent notification would let tools update without polling.

## 3. Features not covered by the spec

USB-C power priority, AC auto boot and USB power share: `force=1` exposed these, but they are not verified on
this laptop and no ticket asks for them. Read-only check first, then toggle and observe. Add each to
`avell_a52_mob_descriptor` only after it is confirmed.
