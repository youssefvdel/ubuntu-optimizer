#!/usr/bin/env bash
# ============================================================
#  Ubuntu Optimizer & Debloater — one-shot, no install
#  Run:  bash <(curl -fsSL https://github.com/youssefvdel/ubuntu-optimizer/releases/latest/download/ubuntu-optimizer.sh)
#  or:   curl -fsSL <url> -o /tmp/ub && bash /tmp/ub
# ============================================================
set -uo pipefail

# ---- colors ----
GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; DIM='\033[2m'; NC='\033[0m'

echo -e "${BOLD}${CYAN}╔════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}${CYAN}║   Ubuntu Optimizer & Debloater (one-shot)   ║${NC}"
echo -e "${BOLD}${CYAN}╚════════════════════════════════════════════╝${NC}"
echo

# ---- detect system ----
DISTRO=$(grep PRETTY_NAME /etc/os-release 2>/dev/null | cut -d= -f2 | tr -d '"')
DESKTOP="${XDG_CURRENT_DESKTOP:-${DESKTOP_SESSION:-unknown}}"
echo -e "${DIM}System: ${DISTRO:-unknown} | Desktop: ${DESKTOP}${NC}"
echo

# ---- helpers ----
applied=0; skipped=0; failed=0
RESULTS=()

is_applied() { # fn returns 0 if the check says "already applied"
  case "$1" in
    remove_snap)      command -v snap >/dev/null 2>&1; [ $? -ne 0 ] || [ -f /etc/apt/preferences.d/nosnap.pref ];;
    install_flatpak)  command -v flatpak >/dev/null 2>&1;;
    firefox_ppa)      [ -f /etc/apt/sources.list.d/mozilla.list ] || [ -f /etc/apt/preferences.d/mozilla ];;
    telemetry_off)    ! command -v ubuntu-report >/dev/null 2>&1;;
    apport_off)       grep -q "enabled=0" /etc/default/apport 2>/dev/null || ! [ -e /usr/bin/apport-cli ];;
    motd_off)         ! [ -e /etc/update-motd.d/50-motd-news ];;
    swappiness_tuned) [ "$(cat /proc/sys/vm/swappiness 2>/dev/null)" -le 10 ];;
    shutdown_fast)    grep -q "DefaultTimeoutStopSec=10s" /etc/systemd/system.conf 2>/dev/null;;
    ssd_trim)         systemctl is-enabled fstrim.timer >/dev/null 2>&1;;
    tracker_off)      systemctl --user is-masked tracker-miner-fs-3.service >/dev/null 2>&1;;
    baloo_off)        balooctl status 2>/dev/null | grep -qiE "disabled|not running";;
    bloat_removed)    ! [ -e /usr/games/gnome-mines ] && ! [ -e /usr/games/kmines ];;
  esac
}

do_apply() { # fn label, key, apply-command...
  local label="$1" key="$2"; shift 2
  if is_applied "$key"; then
    printf "  ${GREEN}✓${NC} %s — ${DIM}already applied${NC}\n" "$label"
    RESULTS+=("✓ $label")
    applied=$((applied+1))
    return
  fi
  printf "  ${YELLOW}→${NC} %s... " "$label"
  if "$@" >/dev/null 2>&1; then
    printf "${GREEN}done${NC}\n"
    RESULTS+=("✓ $label")
    applied=$((applied+1))
  else
    printf "${RED}failed${NC}\n"
    RESULTS+=("✗ $label")
    failed=$((failed+1))
  fi
}

# ---- root check (for the actual changes) ----
if [ "$(id -u)" -ne 0 ]; then
  echo -e "${YELLOW}Note: applying changes needs root — sudo will be used.${NC}"
  SUDO="sudo"
else
  SUDO=""
fi

echo -e "${BOLD}${CYAN}── Packages ──────────────────────────────${NC}"
do_apply "Remove Snap" remove_snap bash -c "
  systemctl stop snapd.service snapd.socket snapd.seeded.service 2>/dev/null;
  systemctl disable snapd.service snapd.socket snapd.seeded.service 2>/dev/null;
  $SUDO apt-get purge -y snapd 2>/dev/null || $SUDO apt purge -y snapd 2>/dev/null;
  $SUDO rm -rf /var/snap /var/lib/snapd /var/cache/snapd \$HOME/snap;
  printf 'Package: snapd\\nPin: release *\\nPin-Priority: -10\\n' | $SUDO tee /etc/apt/preferences.d/nosnap.pref >/dev/null"

do_apply "Install Flatpak + Flathub" install_flatpak bash -c "
  $SUDO apt-get update -qq && $SUDO apt-get install -y flatpak 2>/dev/null;
  flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo 2>/dev/null"

do_apply "Native Firefox (Mozilla repo)" firefox_ppa bash -c "
  $SUDO install -d -m 0755 /etc/apt/keyrings;
  $SUDO wget -q https://packages.mozilla.org/apt/repo-signing-key.gpg -O /etc/apt/keyrings/packages.mozilla.org.asc 2>/dev/null;
  echo 'deb [signed-by=/etc/apt/keyrings/packages.mozilla.org.asc] https://packages.mozilla.org/apt mozilla main' | $SUDO tee /etc/apt/sources.list.d/mozilla.list >/dev/null;
  printf 'Package: *\\nPin: origin packages.mozilla.org\\nPin-Priority: 1000\\n' | $SUDO tee /etc/apt/preferences.d/mozilla >/dev/null;
  $SUDO apt-get update -qq && $SUDO apt-get install -y firefox 2>/dev/null"

echo -e "${BOLD}${CYAN}── Privacy ───────────────────────────────${NC}"
do_apply "Disable telemetry" telemetry_off bash -c "$SUDO apt-get purge -y ubuntu-report popularity-contest geoclue 2>/dev/null"

do_apply "Disable crash popups" apport_off bash -c "
  $SUDO systemctl stop apport whoopsie 2>/dev/null;
  $SUDO systemctl disable apport whoopsie 2>/dev/null;
  $SUDO sed -i 's/enabled=1/enabled=0/g' /etc/default/apport 2>/dev/null"

do_apply "Remove terminal ads" motd_off bash -c "
  $SUDO chmod -x /etc/update-motd.d/50-motd-news /etc/update-motd.d/80-livepatch 2>/dev/null;
  $SUDO sed -i 's/ENABLED=1/ENABLED=0/g' /etc/default/motd-news 2>/dev/null"

echo -e "${BOLD}${CYAN}── Performance ────────────────────────────${NC}"
do_apply "Tune swappiness (10)" swappiness_tuned bash -c "
  $SUDO sysctl -w vm.swappiness=10;
  grep -q 'vm.swappiness' /etc/sysctl.conf && $SUDO sed -i 's/vm.swappiness=.*/vm.swappiness=10/' /etc/sysctl.conf || echo 'vm.swappiness=10' | $SUDO tee -a /etc/sysctl.conf >/dev/null"

do_apply "Shorten shutdown (10s)" shutdown_fast bash -c "
  grep -q 'DefaultTimeoutStopSec' /etc/systemd/system.conf && $SUDO sed -i 's/.*DefaultTimeoutStopSec=.*/DefaultTimeoutStopSec=10s/' /etc/systemd/system.conf || echo 'DefaultTimeoutStopSec=10s' | $SUDO tee -a /etc/systemd/system.conf >/dev/null"

do_apply "Enable SSD TRIM" ssd_trim bash -c "$SUDO systemctl enable --now fstrim.timer 2>/dev/null"

echo -e "${BOLD}${CYAN}── Desktop (${DESKTOP}) ───────────────────${NC}"
case "$DESKTOP" in
  *GNOME*|*gnome*|*Unity*|*Budgie*)
    do_apply "Disable file indexing (Tracker)" tracker_off bash -c "
      systemctl --user stop tracker-miner-fs-3.service tracker-extract-3.service 2>/dev/null;
      systemctl --user mask tracker-miner-fs-3.service tracker-extract-3.service 2>/dev/null";;
  *KDE*|*kde*|*Plasma*|*plasma*)
    do_apply "Disable Baloo indexing" baloo_off bash -c "balooctl disable 2>/dev/null || balooctl purge 2>/dev/null";;
esac

do_apply "Remove desktop bloat" bloat_removed bash -c "$SUDO apt-get purge -y aisleriot gnome-mahjongg gnome-mines gnome-sudoku shotwell kmines ksudoku 2>/dev/null; $SUDO apt-get autoremove -y 2>/dev/null"

# ---- final report ----
echo
echo -e "${BOLD}${CYAN}════════════════════════════════════════════${NC}"
echo -e "  ${GREEN}Applied: ${applied}${NC}   ${RED}Failed: ${failed}${NC}"
if [ "$failed" -gt 0 ]; then
  echo -e "  ${YELLOW}Some steps failed — run with sudo or check logs.${NC}"
fi
echo -e "${BOLD}${CYAN}════════════════════════════════════════════${NC}"
echo
echo -e "${DIM}Changes are reversible: reinstall packages or set swappiness back to 60.${NC}"
