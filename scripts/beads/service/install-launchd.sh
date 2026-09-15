#!/bin/sh
# Install and start the FlashTeX Beads sync loop as a launchd user agent (macOS).
# Usage: scripts/beads/service/install-launchd.sh /absolute/path/to/main/clone
set -eu
repo="$(cd "${1:?usage: install-launchd.sh /path/to/main/clone}" && pwd)"
here="$(cd "$(dirname "$0")" && pwd)"
dst="$HOME/Library/LaunchAgents/dev.flashtex.beads-sync.plist"
mkdir -p "$HOME/Library/LaunchAgents" "$HOME/.local/state/flashtex-beads"
sed -e "s#@REPO@#$repo#g" -e "s#@HOME@#$HOME#g" "$here/dev.flashtex.beads-sync.plist" > "$dst"
launchctl bootout "gui/$(id -u)/dev.flashtex.beads-sync" 2>/dev/null || true
launchctl bootstrap "gui/$(id -u)" "$dst"
launchctl print "gui/$(id -u)/dev.flashtex.beads-sync" | grep -E 'state|pid' | head -3
