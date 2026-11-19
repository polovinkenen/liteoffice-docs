#!/usr/bin/env bash
set -eo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
tauri_dir="$root/src-tauri"
binary_name="liteoffice-docs"
display_name="LiteOffice Docs"

cargo_args=()
app_args=()
in_app=0
for arg in "$@"; do
  if [ "$in_app" -eq 0 ] && [ "$arg" = "--" ]; then
    in_app=1
    continue
  fi
  if [ "$in_app" -eq 1 ]; then
    app_args+=("$arg")
  else
    cargo_args+=("$arg")
  fi
done

if [ "${#cargo_args[@]}" -eq 0 ] || [ "${cargo_args[0]}" != "run" ]; then
  echo "[dev-runner] ожидалась команда cargo run, получено: ${cargo_args[*]:-<пусто>}" >&2
  exit 1
fi
cargo_args[0]="build"

profile="debug"
target=""
index=0
while [ "$index" -lt "${#cargo_args[@]}" ]; do
  arg="${cargo_args[$index]}"
  case "$arg" in
    --release) profile="release" ;;
    --target) index=$((index + 1)); target="${cargo_args[$index]:-}" ;;
    --target=*) target="${arg#--target=}" ;;
    --profile) index=$((index + 1)); profile="${cargo_args[$index]:-}" ;;
    --profile=*) profile="${arg#--profile=}" ;;
  esac
  index=$((index + 1))
done

(cd "$tauri_dir" && cargo "${cargo_args[@]}")

target_dir="${CARGO_TARGET_DIR:-}"
if [ -z "$target_dir" ]; then
  target_dir="$(cd "$tauri_dir" && cargo metadata --no-deps --format-version 1 \
    | node -e 'let raw = ""; process.stdin.on("data", (chunk) => (raw += chunk)).on("end", () => console.log(JSON.parse(raw).target_directory))')"
fi

if [ -n "$target" ]; then
  out_dir="$target_dir/$target/$profile"
else
  out_dir="$target_dir/$profile"
fi

link_path="$out_dir/$display_name"
if [ -e "$link_path" ] && [ ! -L "$link_path" ]; then
  echo "[dev-runner] $link_path существует и не является symlink" >&2
  exit 1
fi
ln -sfn "$binary_name" "$link_path"

cd "$tauri_dir"
exec "$link_path" "${app_args[@]}"
