#!/bin/bash
# Generate latency graphs from RRD data.
# Run via cron every 30 seconds or as needed.
# Usage: generate-graphs.sh [output_dir]

set -euo pipefail

RRD="/var/lib/ergo-proxy/latency.rrd"
OUT="${1:-/var/www/ergo-proxy}"

mkdir -p "$OUT"

if [ ! -f "$RRD" ]; then
    echo "RRD not found: $RRD"
    exit 1
fi

# Common graph options
OPTS=(
    --width 800 --height 200
    --color "BACK#1a1a2e"
    --color "CANVAS#16213e"
    --color "FONT#e0e0e0"
    --color "GRID#333355"
    --color "MGRID#555577"
    --color "AXIS#e0e0e0"
    --color "ARROW#e0e0e0"
    --border 0
    --font "DEFAULT:10:monospace"
    --imgformat PNG
    --vertical-label "ms"
)

# 6-hour view
rrdtool graph "$OUT/latency-6h.png" \
    "${OPTS[@]}" \
    --title "Peer Latency (6 hours)" \
    --start -6h \
    "DEF:min=$RRD:min:AVERAGE" \
    "DEF:avg=$RRD:avg:AVERAGE" \
    "DEF:max=$RRD:max:AVERAGE" \
    "AREA:max#4a1942:Max" \
    "AREA:avg#2b6777:Avg" \
    "AREA:min#1a1a2e:" \
    "LINE1:avg#52d681:Avg " \
    "LINE1:max#c84b6a:Max " \
    "LINE1:min#4bc8c8:Min " \
    "GPRINT:min:LAST:Min\: %5.1lf ms" \
    "GPRINT:avg:LAST:Avg\: %5.1lf ms" \
    "GPRINT:max:LAST:Max\: %5.1lf ms\\n" \
    > /dev/null

# 24-hour view
rrdtool graph "$OUT/latency-24h.png" \
    "${OPTS[@]}" \
    --title "Peer Latency (24 hours)" \
    --start -24h \
    "DEF:min=$RRD:min:AVERAGE" \
    "DEF:avg=$RRD:avg:AVERAGE" \
    "DEF:max=$RRD:max:AVERAGE" \
    "AREA:max#4a1942:Max" \
    "AREA:avg#2b6777:Avg" \
    "AREA:min#1a1a2e:" \
    "LINE1:avg#52d681:Avg " \
    "LINE1:max#c84b6a:Max " \
    "LINE1:min#4bc8c8:Min " \
    "GPRINT:min:LAST:Min\: %5.1lf ms" \
    "GPRINT:avg:LAST:Avg\: %5.1lf ms" \
    "GPRINT:max:LAST:Max\: %5.1lf ms\\n" \
    > /dev/null

# 7-day view
rrdtool graph "$OUT/latency-7d.png" \
    "${OPTS[@]}" \
    --title "Peer Latency (7 days)" \
    --start -7d \
    "DEF:min=$RRD:min:AVERAGE" \
    "DEF:avg=$RRD:avg:AVERAGE" \
    "DEF:max=$RRD:max:AVERAGE" \
    "DEF:maxpeak=$RRD:max:MAX" \
    "AREA:max#4a1942:Max" \
    "AREA:avg#2b6777:Avg" \
    "AREA:min#1a1a2e:" \
    "LINE1:avg#52d681:Avg " \
    "LINE1:max#c84b6a:Max " \
    "LINE1:min#4bc8c8:Min " \
    "LINE1:maxpeak#ff6b6b:Peak:dashes" \
    "GPRINT:min:LAST:Min\: %5.1lf ms" \
    "GPRINT:avg:LAST:Avg\: %5.1lf ms" \
    "GPRINT:max:LAST:Max\: %5.1lf ms\\n" \
    > /dev/null

# Peer count
rrdtool graph "$OUT/peers-24h.png" \
    --width 800 --height 100 \
    --color "BACK#1a1a2e" \
    --color "CANVAS#16213e" \
    --color "FONT#e0e0e0" \
    --color "GRID#333355" \
    --color "MGRID#555577" \
    --color "AXIS#e0e0e0" \
    --color "ARROW#e0e0e0" \
    --border 0 \
    --font "DEFAULT:10:monospace" \
    --imgformat PNG \
    --vertical-label "peers" \
    --title "Connected Outbound Peers (24 hours)" \
    --start -24h \
    --lower-limit 0 \
    "DEF:peers=$RRD:peers:AVERAGE" \
    "AREA:peers#2b6777:Peers" \
    "LINE1:peers#52d681:" \
    "GPRINT:peers:LAST:Current\: %2.0lf" \
    > /dev/null
