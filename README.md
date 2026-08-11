# Better Ubuntu

A single-command Ubuntu optimizer and debloater. Scans your system, applies verified optimizations, and prints a report — no installation, no binary, no dependencies.

```bash
curl -fsSL https://github.com/youssefvdel/better-ubuntu/releases/latest/download/ubuntu-optimizer.sh -o /tmp/ubuntu-optimizer.sh && bash /tmp/ubuntu-optimizer.sh
```

## What it does

| Category | Changes |
|---|---|
| Packages | Remove Snap, install Flatpak/Flathub, native Firefox from Mozilla's repository |
| Privacy | Disable telemetry, crash popups, and terminal ads |
| Performance | Swappiness=10, 10s shutdown timeout, enable SSD TRIM |
| Desktop | Disable Tracker (GNOME) or Baloo (KDE), remove preinstalled bloat |

## How it works

- Detects your distribution and desktop environment first
- Checks each optimization before applying — already-applied items are skipped
- Applies everything in a single run and prints a summary report
- Uses `sudo` only when required; every step is safe to re-run

## Verification

Tested on Ubuntu 26.04 LTS with KDE Plasma 6 — all 10 optimizations verified applied, zero failures.

## Reversibility

Every change is reversible:

- Reinstall removed packages with `apt install <package>`
- Restore swappiness with `sysctl -w vm.swappiness=60`
- Re-enable Baloo with `balooctl6 enable`

## License

MIT
