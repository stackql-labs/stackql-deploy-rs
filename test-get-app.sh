#!/bin/sh
# Client-side smoke test for the stackql-deploy installer (mac/linux).
# Confirms the origin is Cloudflare, exercises every installer path / shell-guard,
# then runs the real installer and checks the binary is the right platform build,
# executable, and runnable.

set -eu

BIN=stackql-deploy
BASE=https://get-stackql-deploy.io
INSTALL_URL="$BASE/install.sh"

# User-Agents the worker routes on: a POSIX download tool vs PowerShell.
UA_CURL="curl/8.4.0"
UA_PS="Mozilla/5.0 (Windows NT 10.0) WindowsPowerShell/5.1"

rm -f stackql-deploy
rm -f stackql
rm -f stackql-deploy.exe
rm -f stackql-deploy.zip
rm -f stackql-*-shell.sh

print_box() {
  msg="$1"
  width=$(( ${#msg} + 4 ))
  line=$(printf '%*s' "$width" '' | tr ' ' '-')
  printf '+%s+\n' "$line"
  printf '|  %s  |\n' "$msg"
  printf '+%s+\n' "$line"
}

# Fetch a body with a given User-Agent and assert it contains a substring.
check_body() {
  name="$1"; url="$2"; ua="$3"; expect="$4"
  body=$(curl -fsSL -A "$ua" "$url")
  case "$body" in
    *"$expect"*) echo "  ok: $name" ;;
    *)
      echo "FAIL: $name"
      echo "      expected body to contain: $expect"
      echo "      got first line: $(printf '%s' "$body" | sed -n '1p')"
      exit 1
      ;;
  esac
}

# Assert a path redirects (no -L) to a Location containing a substring.
check_redirect() {
  name="$1"; url="$2"; ua="$3"; expect="$4"
  loc=$(curl -fsS -o /dev/null -D - -A "$ua" "$url" \
    | awk -F': ' 'tolower($1)=="location"{print $2}' | tr -d '\r')
  case "$loc" in
    *"$expect"*) echo "  ok: $name -> $loc" ;;
    *)
      echo "FAIL: $name"
      echo "      expected Location containing: $expect"
      echo "      got: ${loc:-<none>}"
      exit 1
      ;;
  esac
}

print_box "Installing StackQL Deploy for MacOS/Linux"

echo "Origin check:"
server=$(curl -fsSL -D - -o /dev/null "$INSTALL_URL" | awk -F': ' 'tolower($1)=="server"{print $2}' | tr -d '\r')
echo "  server: ${server:-<none>}"
case "$(printf '%s' "$server" | tr 'A-Z' 'a-z')" in
  *cloudflare*) echo "  ok: served by Cloudflare" ;;
  *) echo "FAIL: expected Cloudflare origin, got '${server:-<none>}'"; exit 1 ;;
esac
echo

echo "Endpoint routing:"
# /install auto-detects the calling shell.
check_body "/install (curl)        -> sh installer"        "$BASE/install"     "$UA_CURL" "#!/bin/sh"
check_body "/install (powershell)  -> ps1 installer"       "$BASE/install"     "$UA_PS"   "#Requires -Version 5"
# Explicit endpoints serve their real script for the matching shell.
check_body "/install.sh (curl)     -> sh installer"        "$BASE/install.sh"  "$UA_CURL" "#!/bin/sh"
check_body "/install.ps1 (ps)      -> ps1 installer"       "$BASE/install.ps1" "$UA_PS"   "#Requires -Version 5"
# Wrong-shell guards point at the correct command instead of erroring.
check_body "/install.sh (ps)       -> 'use install.ps1'"   "$BASE/install.sh"  "$UA_PS"   "install.ps1 | iex"
check_body "/install.ps1 (curl)    -> 'use install.sh'"    "$BASE/install.ps1" "$UA_CURL" "install.sh | sh"
echo

echo "Root + fallback redirects:"
check_redirect "/ (linux UA)" "$BASE/" "$UA_CURL"                                 "stackql-deploy-linux-x86_64.tar.gz"
check_redirect "/ (macOS UA)" "$BASE/" "Mozilla/5.0 (Macintosh; Intel Mac OS X)"  "stackql-deploy-macos-universal.tar.gz"
check_redirect "/ (windows UA)" "$BASE/" "Mozilla/5.0 (Windows NT 10.0; Win64)"   "stackql-deploy-windows-x86_64.zip"
check_redirect "/some/other/path" "$BASE/some/other/path" "$UA_CURL"              "stackql-deploy.io"
echo

echo "Running installer:"
curl -fsSL "$INSTALL_URL" | sh

if [ ! -e "$BIN" ]; then
  echo "FAIL: $BIN was not downloaded"
  exit 1
fi
echo

echo "Binary:"
if command -v file >/dev/null 2>&1; then
  file "$BIN"
else
  echo "  (file not available, skipping arch detail)"
fi
echo

echo "Permissions:"
ls -l "$BIN"
if [ ! -x "$BIN" ]; then
  echo "FAIL: $BIN is not executable"
  exit 1
fi
echo

echo "Execution check:"
if ./"$BIN" --version; then
  echo
  echo "PASS: runnable $BIN for $(uname -s)/$(uname -m)"
else
  echo
  echo "FAIL: $BIN did not run on this platform (wrong binary or exec format error)"
  exit 1
fi
