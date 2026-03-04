#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp_dir="$(mktemp -d)"
cleanup_pids=()

stop_pids() {
  for pid in "${cleanup_pids[@]:-}"; do
    if kill -0 "$pid" >/dev/null 2>&1; then
      kill "$pid" >/dev/null 2>&1 || true
      wait "$pid" >/dev/null 2>&1 || true
    fi
  done
  cleanup_pids=()
}

cleanup() {
  stop_pids
  rm -rf "$tmp_dir" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

pick_port() {
  if command -v python3 >/dev/null 2>&1; then
    python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
    return 0
  fi
  echo 24000
}

wait_port() {
  local host="$1"
  local port="$2"
  local deadline_s="${3:-10}"
  if ! command -v python3 >/dev/null 2>&1; then
    sleep 0.2
    return 0
  fi
  python3 - "$host" "$port" "$deadline_s" <<'PY'
import socket, sys, time
host = sys.argv[1]
port = int(sys.argv[2])
deadline = time.time() + float(sys.argv[3])
while time.time() < deadline:
    s = socket.socket()
    s.settimeout(0.2)
    try:
        s.connect((host, port))
        s.close()
        sys.exit(0)
    except Exception:
        s.close()
        time.sleep(0.1)
sys.exit(1)
PY
}

echo "build: rust examples"
(cd "$repo_root/rust" && cargo build --examples -q)

echo "build: go examples"
(cd "$repo_root/go" && go build -o "$tmp_dir/go_serve" ./examples/interop_echo_server)
(cd "$repo_root/go" && go build -o "$tmp_dir/go_call" ./examples/interop_echo_client)
(cd "$repo_root/go" && go build -o "$tmp_dir/go_raw_serve" ./examples/interop_raw_echo_server)
(cd "$repo_root/go" && go build -o "$tmp_dir/go_raw_call" ./examples/interop_raw_echo_client)

echo "build: node addon"
(cd "$repo_root/node" && npm run build:debug >/dev/null)

rust_echo_server="$repo_root/rust/target/debug/examples/interop_echo_server"
rust_echo_client="$repo_root/rust/target/debug/examples/interop_echo_client"
rust_raw_echo_server="$repo_root/rust/target/debug/examples/interop_raw_echo_server"
rust_raw_echo_client="$repo_root/rust/target/debug/examples/interop_raw_echo_client"

node_cmd="node --import $repo_root/node/node_modules/@oxc-node/core/register.mjs"
node_dir="$repo_root/node"
node_echo_server="$node_cmd $node_dir/scripts/interop_echo_server.js"
node_echo_client="$node_cmd $node_dir/scripts/interop_echo_client.js"
node_raw_server="$node_cmd $node_dir/scripts/interop_raw_echo_server.js"
node_raw_client="$node_cmd $node_dir/scripts/interop_raw_echo_client.js"

run_with_retries() {
  local name="$1"
  shift
  local deadline_s="${1:-10}"
  shift
  local i=0
  while true; do
    if "$@" >/dev/null 2>&1; then
      return 0
    fi
    i=$((i + 1))
    if [ "$i" -ge "$((deadline_s * 10))" ]; then
      echo "fail: $name"
      "$@" || true
      return 1
    fi
    sleep 0.1
  done
}

echo "case: rust server -> go client"
port="$(pick_port)"
url="tcp://127.0.0.1:$port"
"$rust_echo_server" "$url" >/dev/null 2>&1 &
cleanup_pids+=("$!")
wait_port 127.0.0.1 "$port" 15
out="$("$tmp_dir/go_call" --connect "$url" --msg "Hello")"
echo "$out" | grep -q "reply: Hello"
echo "ok: rust server -> go client"

stop_pids

echo "case: go server -> rust client"
port="$(pick_port)"
url="tcp://127.0.0.1:$port"
"$tmp_dir/go_serve" --listen "$url" >/dev/null 2>&1 &
cleanup_pids+=("$!")
wait_port 127.0.0.1 "$port" 15
out="$("$rust_echo_client" "$url" "Hello")"
echo "$out" | grep -q "reply: Hello"
echo "ok: go server -> rust client"

stop_pids

echo "case: go server -> go client"
port="$(pick_port)"
url="tcp://127.0.0.1:$port"
"$tmp_dir/go_serve" --listen "$url" >/dev/null 2>&1 &
cleanup_pids+=("$!")
wait_port 127.0.0.1 "$port" 15
out="$("$tmp_dir/go_call" --connect "$url" --msg "Hello")"
echo "$out" | grep -q "reply: Hello"
echo "ok: go server -> go client"

stop_pids

echo "case: rust server -> rust client"
port="$(pick_port)"
url="tcp://127.0.0.1:$port"
"$rust_echo_server" "$url" >/dev/null 2>&1 &
cleanup_pids+=("$!")
wait_port 127.0.0.1 "$port" 15
out="$("$rust_echo_client" "$url" "Hello")"
echo "$out" | grep -q "reply: Hello"
echo "ok: rust server -> rust client"

stop_pids

echo "case: rust server -> node client"
port="$(pick_port)"
url="tcp://127.0.0.1:$port"
"$rust_echo_server" "$url" >/dev/null 2>&1 &
cleanup_pids+=("$!")
wait_port 127.0.0.1 "$port" 15
out="$($node_echo_client "$url" "Hello")"
echo "$out" | grep -q "reply: Hello"
echo "ok: rust server -> node client"

stop_pids

echo "case: node server -> rust client"
port="$(pick_port)"
url="tcp://127.0.0.1:$port"
$node_echo_server "$url" >/dev/null 2>&1 &
cleanup_pids+=("$!")
wait_port 127.0.0.1 "$port" 15
out="$("$rust_echo_client" "$url" "Hello")"
echo "$out" | grep -q "reply: Hello"
echo "ok: node server -> rust client"

stop_pids

echo "case: go server -> node client"
port="$(pick_port)"
url="tcp://127.0.0.1:$port"
"$tmp_dir/go_serve" --listen "$url" >/dev/null 2>&1 &
cleanup_pids+=("$!")
wait_port 127.0.0.1 "$port" 15
out="$($node_echo_client "$url" "Hello")"
echo "$out" | grep -q "reply: Hello"
echo "ok: go server -> node client"

stop_pids

echo "case: node server -> go client"
port="$(pick_port)"
url="tcp://127.0.0.1:$port"
$node_echo_server "$url" >/dev/null 2>&1 &
cleanup_pids+=("$!")
wait_port 127.0.0.1 "$port" 15
out="$("$tmp_dir/go_call" --connect "$url" --msg "Hello")"
echo "$out" | grep -q "reply: Hello"
echo "ok: node server -> go client"

stop_pids

echo "case: node server -> node client"
port="$(pick_port)"
url="tcp://127.0.0.1:$port"
$node_echo_server "$url" >/dev/null 2>&1 &
cleanup_pids+=("$!")
wait_port 127.0.0.1 "$port" 15
out="$($node_echo_client "$url" "Hello")"
echo "$out" | grep -q "reply: Hello"
echo "ok: node server -> node client"

stop_pids

echo "case: rust raw server -> node raw client"
port="$(pick_port)"
url="tcp://127.0.0.1:$port"
"$rust_raw_echo_server" "$url" >/dev/null 2>&1 &
cleanup_pids+=("$!")
wait_port 127.0.0.1 "$port" 15
out="$($node_raw_client "$url" "Raw/Echo" "Hello")"
echo "$out" | grep -q "reply: Hello"
echo "ok: rust raw server -> node raw client"

stop_pids

echo "case: node raw server -> rust raw client"
port="$(pick_port)"
url="tcp://127.0.0.1:$port"
$node_raw_server "$url" "Raw/Echo" >/dev/null 2>&1 &
cleanup_pids+=("$!")
wait_port 127.0.0.1 "$port" 15
out="$("$rust_raw_echo_client" "$url" "Raw/Echo" "HelloNode")"
echo "$out" | grep -q "reply: HelloNode"
echo "ok: node raw server -> rust raw client"

stop_pids

echo "case: go raw server -> node raw client"
port="$(pick_port)"
url="tcp://127.0.0.1:$port"
"$tmp_dir/go_raw_serve" --listen "$url" --method "Raw/Echo" >/dev/null 2>&1 &
cleanup_pids+=("$!")
wait_port 127.0.0.1 "$port" 15
out="$($node_raw_client "$url" "Raw/Echo" "Hello")"
echo "$out" | grep -q "reply: Hello"
echo "ok: go raw server -> node raw client"

stop_pids

echo "case: node raw server -> go raw client"
port="$(pick_port)"
url="tcp://127.0.0.1:$port"
$node_raw_server "$url" "Raw/Echo" >/dev/null 2>&1 &
cleanup_pids+=("$!")
wait_port 127.0.0.1 "$port" 15
out="$("$tmp_dir/go_raw_call" --connect "$url" --method "Raw/Echo" --msg "HelloNode")"
echo "$out" | grep -q "reply: HelloNode"
echo "ok: node raw server -> go raw client"

stop_pids

echo "case: node raw server -> node raw client"
port="$(pick_port)"
url="tcp://127.0.0.1:$port"
$node_raw_server "$url" "Raw/Echo" >/dev/null 2>&1 &
cleanup_pids+=("$!")
wait_port 127.0.0.1 "$port" 15
out="$($node_raw_client "$url" "Raw/Echo" "HelloNode")"
echo "$out" | grep -q "reply: HelloNode"
echo "ok: node raw server -> node raw client"

echo "all ok"
