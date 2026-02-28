#!/bin/bash
# Diagnostic game runner — speaks engine protocol directly
# Plays a full-auto game for N rounds, with LogFile enabled

ENGINE="./target/release/odin-engine.exe"
LOGFILE="observer/logs/ffa_standard_d6_10rounds_$(date +%Y-%m-%d_%H%M%S).log"
DEPTH=6
MAX_PLY=40  # 10 rounds × 4 players

echo "=== Odin Diagnostic Runner ==="
echo "Engine: $ENGINE"
echo "Log: $LOGFILE"
echo "Depth: $DEPTH"
echo "Max ply: $MAX_PLY"
echo ""

# Create a temp fifo for engine communication
FIFO=$(mktemp -u)
mkfifo "$FIFO"

# Start engine with stdin from fifo, capture stdout
exec 3<>"$FIFO"
$ENGINE <&3 &
ENGINE_PID=$!

# Helper: send command to engine
send() {
    echo "$1" >&3
    echo ">>> $1"
}

# Helper: read lines until prefix found
read_until() {
    while IFS= read -r line; do
        echo "<<< $line"
        if [[ "$line" == "$1"* ]]; then
            break
        fi
    done
}

# Handshake
send "odin"
sleep 0.3
send "isready"
sleep 0.3

# Configure
send "setoption name LogFile value $LOGFILE"
send "setoption name gamemode value ffa"
send "setoption name evalprofile value standard"
send "isready"
sleep 0.5

MOVES=""
PLY=0
PLAYERS=("Red" "Blue" "Yellow" "Green")
GAMEOVER=0

echo ""
echo "=== Starting game ==="

while [ $PLY -lt $MAX_PLY ] && [ $GAMEOVER -eq 0 ]; do
    PLAYER=${PLAYERS[$((PLY % 4))]}
    
    # Send position
    if [ -z "$MOVES" ]; then
        send "position startpos"
    else
        send "position startpos moves $MOVES"
    fi
    
    send "go depth $DEPTH"
    
    # Read until bestmove
    BESTMOVE=""
    while IFS= read -r line <&0; do
        if [[ "$line" == bestmove* ]]; then
            BESTMOVE="${line#bestmove }"
            echo "  Ply $PLY ($PLAYER): $BESTMOVE"
            break
        elif [[ "$line" == *gameover* ]]; then
            echo "  GAME OVER: $line"
            GAMEOVER=1
            break
        fi
    done < <(timeout 30 cat /proc/$ENGINE_PID/fd/1 2>/dev/null || true)
    
    if [ -n "$BESTMOVE" ]; then
        if [ -z "$MOVES" ]; then
            MOVES="$BESTMOVE"
        else
            MOVES="$MOVES $BESTMOVE"
        fi
        PLY=$((PLY + 1))
    fi
    
    if [ $GAMEOVER -eq 1 ]; then
        break
    fi
done

# Disable logging and quit
send "setoption name LogFile value none"
send "quit"
sleep 0.5

# Cleanup
exec 3>&-
rm -f "$FIFO"
wait $ENGINE_PID 2>/dev/null

echo ""
echo "=== Done: $PLY ply ==="
echo "Log file: $LOGFILE"
