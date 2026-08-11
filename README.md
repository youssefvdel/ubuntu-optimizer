# Better Ubuntu 🚀

**Clean, fast, private Ubuntu — one command, no install.**

```bash
curl -fsSL https://github.com/youssefvdel/better-ubuntu/releases/latest/download/ubuntu-optimizer.sh -o /tmp/ubuntu-optimizer.sh && bash /tmp/ubuntu-optimizer.sh
```

## What it does (one-shot)

| Category | Changes |
|---|---|
| 📦 Packages | Remove Snap · Install Flatpak/Flathub · Native Firefox from Mozilla |
| 🛡️ Privacy | Disable telemetry · Disable crash popups · Remove terminal ads |
| ⚡ Performance | Swappiness=10 · 10s shutdown · SSD TRIM |
| 🖥️ Desktop | Tracker off (GNOME) · Baloo off (KDE) · Remove bloat |

## How it works

- Scans your system first — skips what's already applied
- Applies everything in one run, prints a report
- No install, no binary, no TUI — just a bash script
- Reversible: reinstall packages or set swappiness back to 60

## Verified

Tested and verified on Ubuntu 26.04 LTS (KDE Plasma 6): **10/10 applied, 0 failures**.
