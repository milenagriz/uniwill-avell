# bar-plugin

The Omarchy bar widget for the Avell A52 MOB control stack (plugin id `avell.control`, ticket #14).

It is a thin client of the daemon's control API: every value comes from `avellctl --json status`, polled every 3 seconds, and every action is an `avellctl` command. It holds no machine logic of its own, and it never touches the graphics card: `avellctl status` asks the card only while it is awake, so an open panel does not wake a sleeping one.

## What it shows

In the bar:

- an icon for the active Power profile (power-saver, balanced, performance)
- the hotter of the processor and graphics card temperatures
- the speed of the faster fan in rpm (the target percent when no fan reports a speed)

The tooltip adds both temperatures, who controls the fans and any performance hold. On a vertical bar, or with "Show temperature and fan speed beside the icon" turned off, only the icon is drawn.

The panel (click the widget, or `omarchy-shell avell.control toggle`):

- three buttons to switch the Power profile, the active one marked, and whether a performance hold is active and why
- a Max fan toggle (off hands the fans back to the profile's fan curve), who controls the fans and at what target, and each fan's rpm and duty
- both temperatures
- the graphics card's state, its power draw and limit while it is awake, and the TGP offset
- the processor's power limits

Changes made elsewhere (CLI, Fn key, power menu) show up at the next poll. When `avelld` is not running the icon dims and the tooltip and panel say "avelld is not running" instead of showing old values.

Keys in the panel: arrows or h/j/k/l move, Enter or Space activates, Esc closes.

The two actions are also IPC calls, for keybindings:

```sh
omarchy-shell avell.control setProfile balanced   # power-saver, balanced or performance
omarchy-shell avell.control toggleMaxFan
```

## Install

Needs `avellctl` on the PATH (`scripts/install.sh` installs it to `/usr/local/bin`).

```sh
ln -s "$PWD/bar-plugin" ~/.config/omarchy/plugins/avell.control   # or copy the folder there
omarchy plugin enable avell.control                              # lands in the bar's right section
```

With a symlinked install the shell does not see edits to the plugin's files: run `omarchy restart shell` after changing them.

Move it with `omarchy bar move avell.control --section <section>`; turn it off with `omarchy plugin disable avell.control`.

## Files

- `manifest.json`: the plugin manifest (a bar widget and a service)
- `Panel.qml`: the bar button and the panel
- `Service.qml`, `Poller.qml`: the one poller all monitors share, and the avellctl calls
- `Model.js`: parsing and validation of avellctl's output (treated as untrusted)
- `i18n/en.json`: the strings
- `test/model.test.js`: `node bar-plugin/test/model.test.js`

Check the manifest with `omarchy plugin validate bar-plugin`.
