# Uniwill Avell

Firmware-level control of the Avell A52 MOB laptop under Linux: fans, power and battery behaviour that the vendor never exposed outside its Windows software.

## Language

**Max fan**:
The firmware's own full-speed fan state, in which both fans run flat out regardless of temperature.
_Avoid_: Fan boost, boost, turbo, cooler boost

**Power profile**:
One of the firmware's operating modes (quiet, balanced, performance) that together set fan behaviour and power budgets.
_Avoid_: Performance mode, fan mode, turbo, overboost

**TGP offset**:
The extra watts granted to the graphics card on top of its base power budget.
_Avoid_: cTGP, GPU boost, boost

**Dynamic Boost**:
NVIDIA's feature that moves spare power budget from the processor to the graphics card while both are under load.
_Avoid_: Boost, DB

**Boot-time setting change**:
A change to a stored BIOS setting made before the operating system starts, leaving the firmware's code untouched.
_Avoid_: NVRAM hack, setup_var tweak, BIOS mod

**Modified image**:
A copy of this machine's own flash backup with only intended edits, written back to the flash by the owner.
_Avoid_: Modded BIOS, custom BIOS, unlocked BIOS

**Sibling board**:
Another vendor's laptop built on the same Tongfang mainboard and BIOS base as this one, whose firmware is reference material only.
_Avoid_: Clone, rebrand, same model
