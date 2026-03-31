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
    --alt-autoscale
)

# Latency graph helper — uses CDEF to suppress NaN from rendering as filled areas
latency_graph() {
    local output="$1" title="$2" start="$3"
    shift 3

    rrdtool graph "$output" \
        "${OPTS[@]}" \
        --title "$title" \
        --start "$start" \
        "DEF:rmin=$RRD:min:AVERAGE" \
        "DEF:ravg=$RRD:avg:AVERAGE" \
        "DEF:rmax=$RRD:max:AVERAGE" \
        "CDEF:min=rmin,UN,0,rmin,IF" \
        "CDEF:avg=ravg,UN,0,ravg,IF" \
        "CDEF:max=rmax,UN,0,rmax,IF" \
        "CDEF:hasdata=ravg,UN,0,1,IF" \
        "CDEF:dmin=rmin,UN,UNKN,rmin,IF" \
        "CDEF:davg=ravg,UN,UNKN,ravg,IF" \
        "CDEF:dmax=rmax,UN,UNKN,rmax,IF" \
        "AREA:dmax#4a1942:Max" \
        "AREA:davg#2b6777:Avg" \
        "LINE1:davg#52d681:Avg " \
        "LINE1:dmax#c84b6a:Max " \
        "LINE1:dmin#4bc8c8:Min " \
        "$@" \
        "GPRINT:rmin:LAST:Min\: %5.1lf ms" \
        "GPRINT:ravg:LAST:Avg\: %5.1lf ms" \
        "GPRINT:rmax:LAST:Max\: %5.1lf ms\\n" \
        > /dev/null
}

# 6-hour view
latency_graph "$OUT/latency-6h.png" "Peer Latency (6 hours)" "-6h"

# 24-hour view
latency_graph "$OUT/latency-24h.png" "Peer Latency (24 hours)" "-24h"

# 7-day view
latency_graph "$OUT/latency-7d.png" "Peer Latency (7 days)" "-7d" \
    "DEF:rmaxpeak=$RRD:max:MAX" \
    "CDEF:maxpeak=rmaxpeak,UN,UNKN,rmaxpeak,IF" \
    "LINE1:maxpeak#ff6b6b:Peak:dashes"

# Peer count — use MAX not AVERAGE to get whole numbers
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
    --rigid \
    --units-exponent 0 \
    "DEF:peers=$RRD:peers:LAST" \
    "CDEF:ipeers=peers,UN,0,peers,IF" \
    "CDEF:dpeers=peers,UN,UNKN,peers,IF" \
    "AREA:dpeers#2b6777:Peers" \
    "LINE1:dpeers#52d681:" \
    "GPRINT:peers:LAST:Current\: %2.0lf" \
    > /dev/null
