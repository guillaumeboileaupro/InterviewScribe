#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: $0 OLD.deb NEW.deb" >&2
  exit 2
fi

old_deb=$1
new_deb=$2
for deb in "$old_deb" "$new_deb"; do
  if [[ ! -f "$deb" || "$deb" != *.deb ]]; then
    echo "Paquet Debian introuvable: $deb" >&2
    exit 2
  fi
done

old_package=$(dpkg-deb -f "$old_deb" Package)
new_package=$(dpkg-deb -f "$new_deb" Package)
old_version=$(dpkg-deb -f "$old_deb" Version)
new_version=$(dpkg-deb -f "$new_deb" Version)
if [[ "$old_package" != "$new_package" ]]; then
  echo "Les paquets ne ciblent pas la meme application." >&2
  exit 2
fi
if dpkg --compare-versions "$new_version" le "$old_version"; then
  echo "La version N ($new_version) doit etre superieure a N-1 ($old_version)." >&2
  exit 2
fi

profile_root=$(mktemp -d /tmp/interviewscribe-upgrade.XXXXXX)
cleanup() {
  sudo dpkg -r "$new_package" >/dev/null 2>&1 || true
  case "$profile_root" in
    /tmp/interviewscribe-upgrade.*) rm -rf -- "$profile_root" ;;
  esac
}
trap cleanup EXIT

export XDG_DATA_HOME="$profile_root/data"
export XDG_CONFIG_HOME="$profile_root/config"
export XDG_CACHE_HOME="$profile_root/cache"
mkdir -p "$XDG_DATA_HOME/com.guillaumeboileau.interviewscribe"
database="$XDG_DATA_HOME/com.guillaumeboileau.interviewscribe/interviewscribe.sqlite3"

sudo dpkg -i "$old_deb"
installed_old=$(dpkg-query -W -f='${Version}' "$old_package")
[[ "$installed_old" == "$old_version" ]]
sqlite3 "$database" < tests/fixtures/db/schema-v0.sql

sudo dpkg -i "$new_deb"
installed_new=$(dpkg-query -W -f='${Version}' "$new_package")
[[ "$installed_new" == "$new_version" ]]
binary=$(dpkg -L "$new_package" | awk '/^\/usr\/bin\// { print; exit }')
[[ -n "$binary" && -x "$binary" ]]

assert_eq() {
  local label=$1 expected=$2 actual=$3
  if [[ "$actual" != "$expected" ]]; then
    echo "Echec verification post-mise a niveau [$label]: attendu '$expected', obtenu '$actual'" >&2
    exit 1
  fi
}

# Poll for the migration's own completion signal instead of waiting a fixed
# duration then killing and hoping: a GUI app never exits on its own, so a
# fixed-wait-then-kill race can inspect the database before `schema::init`
# has actually committed `user_version` - this was the real, confirmed cause
# of a prior release failure (docs/TEST_IMPLEMENTATION_PLAN.md section 9).
xvfb-run --auto-servernum "$binary" &
app_pid=$!
migrated=0
for _ in $(seq 1 40); do
  if [[ "$(sqlite3 "$database" 'PRAGMA user_version' 2>/dev/null)" == "1" ]]; then
    migrated=1
    break
  fi
  if ! kill -0 "$app_pid" 2>/dev/null; then
    echo "Le processus de la version N s'est arrete avant la fin de la migration" >&2
    break
  fi
  sleep 0.5
done
kill "$app_pid" 2>/dev/null || true
wait "$app_pid" 2>/dev/null || true
if [[ $migrated -ne 1 ]]; then
  echo "La migration SQLite n'a pas atteint user_version=1 dans le delai imparti (20s)" >&2
  exit 1
fi

assert_eq "user_version" "1" "$(sqlite3 "$database" 'PRAGMA user_version')"
assert_eq "colonne interview.notes" "1" \
  "$(sqlite3 "$database" "SELECT COUNT(*) FROM pragma_table_info('interview') WHERE name='notes'")"
assert_eq "colonne edit.reverted_at" "1" \
  "$(sqlite3 "$database" "SELECT COUNT(*) FROM pragma_table_info('edit') WHERE name='reverted_at'")"
for table in interview speaker segment edit setting; do
  assert_eq "lignes conservees dans $table" "1" \
    "$(sqlite3 "$database" "SELECT COUNT(*) FROM $table")"
done
assert_eq "raw_text du segment 1 (immuable)" "Texte synthetique immuable." \
  "$(sqlite3 "$database" 'SELECT raw_text FROM segment WHERE id=1')"

echo "Mise a niveau validee: $old_version -> $new_version; schema et donnees conserves."
