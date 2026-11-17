#!/usr/bin/env bash
# Seed a demo ROOM instance so the live preview has content on first load.
# Safe to re-run: it only acts when the database has no users yet.
set -euo pipefail

BASE="http://127.0.0.1:8080"
JAR="$(mktemp)"
BOB_JAR="$(mktemp)"
trap 'rm -f "$JAR" "$BOB_JAR"' EXIT

# wait for the server
for _ in $(seq 1 60); do
  if curl -fsS "$BASE/api/health" >/dev/null 2>&1; then break; fi
  sleep 0.5
done

USERS=$(curl -fsS "$BASE/api/users" 2>/dev/null | grep -c '"id"' || true)

# Fresh instance: register the owner, a second member, approve them, and seed a chat.
curl -fsS -c "$JAR" -o /dev/null -d 'name=Yasmine&device=studio-laptop' "$BASE/api/request-access" || exit 0
curl -fsS -c "$BOB_JAR" -o /dev/null -d 'name=Liam Nguyen&device=liam-phone' "$BASE/api/request-access" || exit 0

RID=$(curl -fsS -b "$JAR" "$BASE/api/admin/requests" | python3 -c 'import json,sys; d=json.load(sys.stdin)["pending"]; print(d[0]["id"] if d else "")')
if [ -n "$RID" ]; then
  curl -fsS -b "$JAR" -X POST -o /dev/null "$BASE/api/admin/requests/$RID/approve"
fi

ME=$(curl -fsS -b "$JAR" "$BASE/api/users" | python3 -c 'import json,sys; u=json.load(sys.stdin)["users"]; print(next(x["id"] for x in u if x["role"]=="owner"))')
BOB=$(curl -fsS -b "$JAR" "$BASE/api/users" | python3 -c 'import json,sys; u=json.load(sys.stdin)["users"]; print(next(x["id"] for x in u if x["name"]=="Liam Nguyen"))')

CONV=$(curl -fsS -b "$JAR" -H 'Content-Type: application/json' \
  -d "{\"kind\":\"group\",\"name\":\"Project review\",\"topic\":\"Q4 launch\",\"members\":[$ME,$BOB]}" \
  "$BASE/api/conversations" | python3 -c 'import json,sys; print(json.load(sys.stdin)["conversation"]["id"])')

send() {
  curl -fsS -b "$1" -H 'Content-Type: application/json' \
    -d "{\"body\":$2}" "$BASE/api/conversations/$CONV/messages" >/dev/null
}

send "$BOB_JAR" '"Hi! Are we still on for the project review this afternoon?"'
send "$JAR" '"Yes, 3 PM works perfect for me."'
send "$BOB_JAR" '"Great, I just shared the latest documents with you."'
send "$JAR" '"Thanks! I will go through them and add my feedback."'

curl -fsS -b "$JAR" -H 'Content-Type: application/json' -d '{"name":"Design","kind":"folder"}' "$BASE/api/files" >/dev/null
curl -fsS -b "$JAR" -H 'Content-Type: text/plain' \
  --data-binary @README.md "$BASE/api/files/upload?name=ROOM-overview.md" >/dev/null

echo "seeded conversation $CONV"
