#!/usr/bin/env bash
set -euo pipefail

app_id="app.liteoffice.docs"
binary_name="liteoffice-docs"
script_path=$(realpath "$0")

if [ "${1:-}" = "e2e" ]; then
  binary="${2:?укажите путь к исполняемому файлу}"
  data_dir="${XDG_DATA_HOME:-$HOME/.local/share}/$app_id"
  recent="$data_dir/recent.json"
  work=$(mktemp -d)
  first_pid=""

  cleanup() {
    if [ -n "$first_pid" ]; then
      kill "$first_pid" 2>/dev/null || true
      wait "$first_pid" 2>/dev/null || true
    fi
  }
  trap cleanup EXIT

  wait_recent() {
    local needle="$1"
    for _ in $(seq 1 60); do
      if [ -n "$first_pid" ] && ! kill -0 "$first_pid" 2>/dev/null; then
        echo "приложение завершилось, не открыв $needle" >&2
        cat "$work/first.log" >&2 || true
        return 1
      fi
      if [ -f "$recent" ] && grep -qF "$needle" "$recent"; then
        return 0
      fi
      sleep 1
    done
    echo "не дождались открытия $needle" >&2
    [ -f "$recent" ] && cat "$recent" >&2 || true
    cat "$work/first.log" >&2 || true
    return 1
  }

  rm -rf "$data_dir"

  first="$work/первый.md"
  printf '# Первый\n' >"$first"
  WEBKIT_DISABLE_COMPOSITING_MODE=1 WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 GDK_BACKEND=x11 \
    "$binary" "$first" >"$work/first.log" 2>&1 &
  first_pid=$!
  wait_recent "$(basename "$first")"

  second="$work/второй.md"
  printf '# Второй\n' >"$second"
  status=0
  WEBKIT_DISABLE_COMPOSITING_MODE=1 WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 GDK_BACKEND=x11 \
    timeout 30 "$binary" "$second" >"$work/second.log" 2>&1 || status=$?
  if [ "$status" -ne 0 ]; then
    echo "второй экземпляр не завершился после передачи пути (код $status)" >&2
    cat "$work/second.log" >&2 || true
    exit 1
  fi
  wait_recent "$(basename "$second")"

  echo "e2e: ${binary} открыл оба файла"
  exit 0
fi

artifacts="${1:?укажите каталог артефактов}"

deb=$(find "$artifacts" -type f -name "*.deb" -print -quit)
appimage=$(find "$artifacts" -type f -name "*.AppImage" -print -quit)

if [ -z "$deb" ] || [ -z "$appimage" ]; then
  echo "не найдены .deb и .AppImage в $artifacts" >&2
  exit 1
fi

sudo apt-get update
sudo apt-get install -y xvfb dbus-x11 desktop-file-utils
cp "$(realpath "$deb")" /tmp/liteoffice-docs-smoke.deb
sudo apt-get install -y /tmp/liteoffice-docs-smoke.deb

installed_binary=$(command -v "$binary_name")
package=$(dpkg -S "$installed_binary" | cut -d: -f1)
echo "пакет: $package, бинарник: $installed_binary"

desktop="/usr/share/applications/LiteOffice Docs.desktop"
if [ ! -f "$desktop" ]; then
  echo "не найден $desktop" >&2
  exit 1
fi

grep -Eq '^Exec=.* %F$' "$desktop" || {
  echo "в $desktop нет Exec с %F" >&2
  exit 1
}
grep -q '^MimeType=.*application/vnd.openxmlformats-officedocument.wordprocessingml.document' "$desktop" || {
  echo "в $desktop нет MimeType DOCX" >&2
  exit 1
}

desktop_errors=$(desktop-file-validate "$desktop" 2>&1 | grep 'error:' || true)
if [ -n "$desktop_errors" ]; then
  echo "$desktop_errors" >&2
  exit 1
fi

sudo update-desktop-database /usr/share/applications || true
echo "xdg-mime text/markdown: $(xdg-mime query default text/markdown || true)"

dbus-run-session -- xvfb-run -a "$script_path" e2e "$installed_binary"

sudo dpkg -r "$package"
if command -v "$binary_name" >/dev/null 2>&1; then
  echo "бинарник остался после удаления пакета" >&2
  exit 1
fi

appimage=$(realpath "$appimage")
chmod +x "$appimage"
extract_dir=$(mktemp -d)
(
  cd "$extract_dir"
  "$appimage" --appimage-extract >/dev/null
)

appimage_desktop=$(find "$extract_dir/squashfs-root" -maxdepth 5 -type f -name "*.desktop" -print -quit)
if [ -z "$appimage_desktop" ]; then
  echo "в AppImage нет .desktop" >&2
  exit 1
fi

if ! grep -Eq '^Exec=.* %F$' "$appimage_desktop"; then
  echo "предупреждение: в AppImage .desktop нет Exec с %F" >&2
fi

appimage_binary="$extract_dir/squashfs-root/AppRun"
if [ ! -x "$appimage_binary" ]; then
  appimage_binary=$(find "$extract_dir/squashfs-root/usr/bin" -type f -name "$binary_name" -print -quit)
fi
if [ -z "$appimage_binary" ] || [ ! -x "$appimage_binary" ]; then
  echo "в AppImage не найден исполняемый файл" >&2
  exit 1
fi

dbus-run-session -- xvfb-run -a "$script_path" e2e "$appimage_binary"

echo "smoke linux: ok"
