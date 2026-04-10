#!/bin/bash
# runb-docker-shim: Adapter between Docker/containerd OCI runtime interface and runb
set -e

# Parse global flags (before command)
BUNDLE_DIR="."
CONTAINER_ID=""
PID_FILE=""
LOG_FILE=""
ROOT=""
SIGNAL=""
COMMAND=""

ARGS=("$@")
i=0
while [ $i -lt ${#ARGS[@]} ]; do
    arg="${ARGS[$i]}"
    case "$arg" in
        --root)      ROOT="${ARGS[$((i+1))]}"; i=$((i+2)) ;;
        --root=*)    ROOT="${arg#*=}"; i=$((i+1)) ;;
        --log)       LOG_FILE="${ARGS[$((i+1))]}"; i=$((i+2)) ;;
        --log=*)     LOG_FILE="${arg#*=}"; i=$((i+1)) ;;
        --log-format|--log-format=*|--log-level|--log-level=*|--systemd-cgroup|--debug)
            [[ "$arg" == *=* ]] || { [[ "$arg" == --log-format || "$arg" == --log-level ]] && i=$((i+1)); }
            i=$((i+1)) ;;
        --*)
            i=$((i+1))
            [ $i -lt ${#ARGS[@]} ] && [[ "${ARGS[$i]}" != --* ]] && i=$((i+1)) 2>/dev/null || true
            ;;
        features|create|start|state|kill|delete|delete-all|list|exec|spec)
            COMMAND="$arg"; i=$((i+1)); break ;;
        *) COMMAND="$arg"; i=$((i+1)); break ;;
    esac
done

# Parse command-specific flags
while [ $i -lt ${#ARGS[@]} ]; do
    arg="${ARGS[$i]}"
    case "$arg" in
        --bundle)      BUNDLE_DIR="${ARGS[$((i+1))]}"; i=$((i+2)) ;;
        --bundle=*)    BUNDLE_DIR="${arg#*=}"; i=$((i+1)) ;;
        --pid-file)    PID_FILE="${ARGS[$((i+1))]}"; i=$((i+2)) ;;
        --pid-file=*)  PID_FILE="${arg#*=}"; i=$((i+1)) ;;
        --signal)      SIGNAL="${ARGS[$((i+1))]}"; i=$((i+2)) ;;
        --signal=*)    SIGNAL="${arg#*=}"; i=$((i+1)) ;;
        --force|--console-socket) i=$((i+1)) ;;
        --*=*)         i=$((i+1)) ;;
        --*)           i=$((i+1)); [ $i -lt ${#ARGS[@]} ] && i=$((i+1)) ;;
        *)
            [ -z "$CONTAINER_ID" ] && CONTAINER_ID="$arg"
            i=$((i+1)) ;;
    esac
done

log_msg() {
    if [ -n "$LOG_FILE" ]; then
        mkdir -p "$(dirname "$LOG_FILE")"
        echo "{\"level\":\"info\",\"msg\":\"$1\",\"time\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}" >> "$LOG_FILE"
    fi
}

case "$COMMAND" in
    features)
        echo '{"ociVersionMin":"1.0.0","ociVersionMax":"1.0.2-dev","hooks":true,"mountOptions":true,"linux":true}'
        ;;

    create)
        log_msg "create container $CONTAINER_ID bundle=$BUNDLE_DIR"
        # Create container state
        /usr/local/bin/runb create "$CONTAINER_ID" --bundle "$BUNDLE_DIR"
        # Fork a holding process so Docker gets a real PID
        if [ -n "$PID_FILE" ]; then
            mkdir -p "$(dirname "$PID_FILE")"
            HOLD_PID=$(/usr/local/bin/runb-hold "$PID_FILE")
            # Save hold PID for later cleanup
            mkdir -p /run/runb/$CONTAINER_ID
            echo "$HOLD_PID" > /run/runb/$CONTAINER_ID/hold-pid
        fi
        log_msg "create done (hold PID: ${HOLD_PID:-none})"
        ;;

    start)
        log_msg "start container $CONTAINER_ID"
        # Kill the holding process first
        if [ -f "/run/runb/$CONTAINER_ID/hold-pid" ]; then
            HOLD_PID=$(cat "/run/runb/$CONTAINER_ID/hold-pid")
            kill "$HOLD_PID" 2>/dev/null || true
            rm -f "/run/runb/$CONTAINER_ID/hold-pid"
        fi
        # Start the real container
        /usr/local/bin/runb start "$CONTAINER_ID"
        log_msg "start done"
        ;;

    state)
        /usr/local/bin/runb state "$CONTAINER_ID"
        ;;

    kill)
        log_msg "kill container $CONTAINER_ID signal=$SIGNAL"
        if [ -n "$SIGNAL" ]; then
            /usr/local/bin/runb stop "$CONTAINER_ID" --signal "$SIGNAL"
        else
            /usr/local/bin/runb stop "$CONTAINER_ID"
        fi
        # Also kill hold process if still alive
        if [ -f "/run/runb/$CONTAINER_ID/hold-pid" ]; then
            HOLD_PID=$(cat "/run/runb/$CONTAINER_ID/hold-pid")
            kill "$HOLD_PID" 2>/dev/null || true
        fi
        log_msg "kill done"
        ;;

    delete)
        log_msg "delete container $CONTAINER_ID"
        /usr/local/bin/runb delete "$CONTAINER_ID" 2>/dev/null || true
        log_msg "delete done"
        ;;

    delete-all)
        for id in "$CONTAINER_ID" "$@"; do
            [ -n "$id" ] && /usr/local/bin/runb delete "$id" 2>/dev/null || true
        done
        ;;

    list)
        /usr/local/bin/runb list
        ;;

    *)
        echo "runb-docker-shim: Unknown command: $COMMAND" >&2
        exit 1
        ;;
esac
