#!/bin/sh
# usage: sh lidtest.sh arm|disarm
# Sets state, logs a heartbeat every 5s, then on Ctrl-C reports gaps + sleep events.
set -eu
mode=${1:?usage: sh lidtest.sh arm|disarm}
dir=$(cd "$(dirname "$0")" && pwd)
log=$dir/heartbeat.log
"$dir/target/debug/ku-spike" "$mode"
: > "$log"
start=$(date '+%Y-%m-%d %H:%M:%S')
echo "[$mode] heartbeat -> $log. Close lid >= 60s, reopen, then Ctrl-C."
report() {
  echo "--- heartbeat gaps > 10s ---"
  awk 'NR>1 && $1-prev>10 {printf "gap %ds ending epoch %d\n", $1-prev, $1} {prev=$1}' "$log"
  echo "--- sleep/wake/display events since $start ---"
  pmset -g log | awk -v s="$start" '/^20/ && substr($0,1,19) >= s' | grep -E " Sleep | Wake |DarkWake|Display is turned" | cut -c1-140
  echo "--- AppleClamshellCausesSleep (as of last lid event) ---"
  ioreg -r -n IOPMrootDomain -d 1 | grep AppleClamshellCausesSleep
  exit 0
}
trap report INT TERM
while :; do date +%s >> "$log"; sleep 5; done
