#!/usr/bin/env bash
# Installs Afterglow Twilight for Linux under ~/.local (or --prefix): no system folders, no sudo. `--uninstall` removes it.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
prefix="${HOME}/.local"
uninstall=0
build=1

while [ $# -gt 0 ]; do
	case "$1" in
	--prefix)
		prefix="$2"
		shift 2
		;;
	--uninstall)
		uninstall=1
		shift
		;;
	--no-build)
		build=0
		shift
		;;
	-h | --help)
		echo "usage: install.sh [--prefix DIR] [--no-build] [--uninstall]   (default prefix ~/.local)"
		exit 0
		;;
	*)
		echo "install.sh: unknown option $1" >&2
		exit 2
		;;
	esac
done

bin="$prefix/bin"
apps="$prefix/share/applications"
share="$prefix/share/afterglow-savers/xscreensaver"
units="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"

if [ "$uninstall" = 1 ]; then
	rm -f "$bin/afterglow-twilight-gpu" "$bin/afterglow-twilight-xscreensaver" "$bin/afterglow-twilight-idle"
	rm -f "$apps/afterglow-twilight.desktop" "$share/afterglow-twilight.xml" "$units/afterglow-twilight-idle.service"
	rmdir "$share" "$(dirname "$share")" 2>/dev/null || true
	echo "Removed. The settings file (~/.config/liminal-hq/afterglow-savers/twilight.toml) was left alone."
	exit 0
fi

built="$repo/target/release/afterglow-twilight-gpu"
if [ "$build" = 1 ]; then
	(cd "$repo" && cargo build --release --locked -p afterglow-twilight-gpu)
fi
[ -x "$built" ] || {
	echo "install.sh: $built is not built (run without --no-build)" >&2
	exit 1
}

mkdir -p "$bin" "$apps" "$share" "$units"
install -s -m 755 "$built" "$bin/afterglow-twilight-gpu"
install -m 755 "$here/xscreensaver/afterglow-twilight-xscreensaver" "$bin/afterglow-twilight-xscreensaver"
install -m 755 "$here/xscreensaver/afterglow-twilight-idle" "$bin/afterglow-twilight-idle"
install -m 644 "$here/xscreensaver/afterglow-twilight.xml" "$share/afterglow-twilight.xml"
sed "s|@BIN@|$bin|g" "$here/desktop/afterglow-twilight.desktop" >"$apps/afterglow-twilight.desktop"
sed "s|@BIN@|$bin|g" "$here/systemd/afterglow-twilight-idle.service" >"$units/afterglow-twilight-idle.service"

"$bin/afterglow-twilight-gpu" --write-config >/dev/null

cat <<MESSAGE
Installed under $prefix.

  Try it in a window:        $bin/afterglow-twilight-gpu --window 1280x720
  As a screensaver:          $bin/afterglow-twilight-gpu          (any key, click or 10 px of movement ends it)
  Settings:                  ~/.config/liminal-hq/afterglow-savers/twilight.toml

  xscreensaver: add this to \`programs:\` in ~/.xscreensaver
      "Afterglow Twilight"  $bin/afterglow-twilight-xscreensaver -root  \\n\\
  and, for its settings dialog, copy $share/afterglow-twilight.xml into xscreensaver's config folder
  (usually /usr/share/xscreensaver/config/, which needs administrator rights; skipping it only loses the dialog).

  GNOME, KDE and others (no hook): systemctl --user enable --now afterglow-twilight-idle.service
  swayidle:                  swayidle -w timeout 300 '$bin/afterglow-twilight-gpu'

Make sure $bin is on your PATH. See docs/linux-edition.md.
MESSAGE
