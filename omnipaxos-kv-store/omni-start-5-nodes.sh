#!/usr/bin/env bash

# OmniPaxos 5-node helper: start | stop | status | restart | clean
BIN="replicant/target/release/omni_replicant"
LOGDIR="logs"
NODES=(0 1 2 3 4)

check_bin() {
  [ -x "$BIN" ] || { echo "Error: $BIN not found or not executable."; echo "Build with: cd replicant && cargo build --release"; exit 1; }
}

start_one() {
  i="$1"
  consensus_port=$((10000 + i * 1000))
  client_port=$((consensus_port + 1))
  mkdir -p "$LOGDIR"
  RUST_LOG=info "$BIN" --id "$i" --config-path "config/config_node$i.json" >"$LOGDIR/node_$i.log" 2>&1 &
  echo $! >"$LOGDIR/node_$i.pid"
  echo "Node $i: consensus=localhost:$consensus_port, client=localhost:$client_port"
  sleep 1
}

stop_one() {
  i="$1"
  pidfile="$LOGDIR/node_$i.pid"
  if [ -f "$pidfile" ]; then
    pid="$(cat "$pidfile")"
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null
    fi
    rm -f "$pidfile"
  fi
  # Clean data directory
  rm -rf "/tmp/presistent_node$i"
}

clean_data() {
  for i in "${NODES[@]}"; do
    rm -rf "/tmp/presistent_node$i"
    echo "Cleaned data for node $i"
  done
}

start_all() {
  check_bin
  stop_all
  for i in "${NODES[@]}"; do start_one "$i"; done
  echo "Started ${#NODES[@]} nodes. Logs in $LOGDIR/node_X.log"
}

stop_all() {
  for i in "${NODES[@]}"; do stop_one "$i"; done
  echo "Stopped nodes and cleaned data directories."
}

status_all() {
  for i in "${NODES[@]}"; do
    consensus_port=$((10000 + i * 1000))
    client_port=$((consensus_port + 1))
    pidfile="$LOGDIR/node_$i.pid"
    if [ -f "$pidfile" ]; then
      pid="$(cat "$pidfile")"
      if ps -p "$pid" >/dev/null 2>&1; then
        echo "Node $i: running (pid $pid) - consensus=localhost:$consensus_port, client=localhost:$client_port"
      else
        echo "Node $i: not running (stale pid file removed) - ports: consensus=$consensus_port, client=$client_port"
        rm -f "$pidfile"
      fi
    else
      echo "Node $i: not running - ports: consensus=$consensus_port, client=$client_port"
    fi
  done
}

case "${1:-start}" in
  start)   start_all ;;
  stop)    stop_all ;;
  status)  status_all ;;
  restart) stop_all; start_all ;;
  clean)   clean_data ;;
  *) echo "Usage: $0 {start|stop|status|restart|clean}"; exit 1 ;;
esac