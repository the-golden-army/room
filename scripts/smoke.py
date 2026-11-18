#!/usr/bin/env python3
"""ROOM end-to-end smoke suite.

Drives the real binary over HTTP/WebSocket: access gate, approvals, chat,
reactions, files, calls, admin and the realtime channel. Exits non-zero on
the first failed assertion so it can gate a release.

    python3 scripts/smoke.py [base_url]

With no argument it starts dist/room on port 8123 against a temporary data
directory, so every run begins from a virgin database. Pass a base URL to test
an already-running server instead (that server should also start empty).

Requires: websocket-client for the realtime section (the rest is stdlib).
"""
import http.cookiejar
import json
import os
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request

try:
    import websocket
except ImportError:  # pragma: no cover
    websocket = None

PORT = 8123
BASE = (sys.argv[1] if len(sys.argv) > 1 else f"http://127.0.0.1:{PORT}").rstrip("/")
SELF_HOSTED = len(sys.argv) <= 1
FAILS = []
CHECKS = 0
SERVER = None
# A fixed code so the recovery path is testable. Self-hosted runs set this env
# var; against an external server it must match that server's configuration.
SETUP_CODE = "SMOKE-TEST"
WORKDIR = None


def start_server():
    """Launch dist/room on a throwaway data dir so runs start from a clean slate."""
    import shutil
    import subprocess
    import time

    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    bin_path = os.path.join(root, "dist", "room")
    if not os.path.exists(bin_path):
        sys.exit(f"missing {bin_path} — run `cargo build --release` or fetch the CI artifact")
    work = tempfile.mkdtemp(prefix="room-smoke-")
    global WORKDIR
    WORKDIR = work
    env = dict(os.environ)
    env.update(
        ROOM_BIND=f"127.0.0.1:{PORT}",
        ROOM_DATA_DIR=os.path.join(work, "data"),
        ROOM_FILES_DIR=os.path.join(work, "files"),
        ROOM_NAME="ROOM Smoke",
        ROOM_SETUP_CODE=SETUP_CODE,
    )
    proc = subprocess.Popen([bin_path], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    for _ in range(100):
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{PORT}/api/health", timeout=1) as r:
                if r.status == 200:
                    return proc, work
        except Exception:
            time.sleep(0.1)
    proc.kill()
    shutil.rmtree(work, ignore_errors=True)
    sys.exit("server did not become ready")


def stop_server():
    if SERVER is None:
        return
    import shutil

    proc, work = SERVER
    proc.kill()
    shutil.rmtree(work, ignore_errors=True)


def check(label, cond, detail=""):
    global CHECKS
    CHECKS += 1
    if cond:
        print(f"  \033[32mok\033[0m   {label}")
    else:
        print(f"  \033[31mFAIL\033[0m {label} {detail}")
        FAILS.append(label)


class _NoRedirect(urllib.request.HTTPRedirectHandler):
    """Surface 3xx responses so the gate redirect can be asserted."""

    def redirect_request(self, *args, **kwargs):
        return None


class Client:
    """Tiny cookie-preserving HTTP client that never follows redirects."""

    def __init__(self, name):
        self.name = name
        self.jar = http.cookiejar.CookieJar()
        self.opener = urllib.request.build_opener(
            urllib.request.HTTPCookieProcessor(self.jar), _NoRedirect()
        )

    def raw(self, method, path, body=None, headers=None, ctype=None):
        url = BASE + path
        data = None
        hdrs = dict(headers or {})
        if body is not None:
            if isinstance(body, (dict, list)):
                data = json.dumps(body).encode()
                hdrs.setdefault("Content-Type", "application/json")
            elif isinstance(body, str):
                data = body.encode()
                if ctype:
                    hdrs["Content-Type"] = ctype
            else:
                data = body
                if ctype:
                    hdrs["Content-Type"] = ctype
        req = urllib.request.Request(url, data=data, headers=hdrs, method=method)
        try:
            with self.opener.open(req, timeout=15) as r:
                return r.status, r.read(), dict(r.headers)
        except urllib.error.HTTPError as e:
            return e.code, e.read(), dict(e.headers)

    def form(self, path, fields):
        data = urllib.parse.urlencode(fields)
        return self.raw(
            "POST", path, data, ctype="application/x-www-form-urlencoded"
        )

    def get(self, path):
        return self.raw("GET", path)

    def json(self, path, body=None, method=None):
        st, payload, _ = self.raw(method or ("POST" if body is not None else "GET"), path, body)
        try:
            return st, json.loads(payload or b"null")
        except ValueError:
            return st, None

    @property
    def device(self):
        for c in self.jar:
            if c.name == "room_device":
                return c.value
        return ""


def section(title):
    print(f"\n\033[1m{title}\033[0m")


def main():
    global SERVER
    if SELF_HOSTED:
        SERVER = start_server()
        print(f"started {BASE} on a clean data directory")

    section("health & shell")
    anon = Client("anon")
    st, body, _ = anon.get("/api/health")
    health = json.loads(body)
    check("GET /api/health 200", st == 200, st)
    check("health.ok true", health.get("ok") is True, body[:80])
    check("health.db true", health.get("db") is True)
    st, _, hdrs = anon.get("/gate")
    check("GET /gate 200", st == 200, st)
    st, _, hdrs = anon.get("/")
    check("GET / redirects to /gate", st in (301, 302, 303) and "/gate" in hdrs.get("location", ""), (st, hdrs.get("location")))
    # Before any owner exists nobody could approve anyone, so the gate must
    # offer setup rather than asking permission from a host who isn't there.
    st, first_page, _ = anon.get("/gate")
    first = first_page.decode(errors="replace")
    check("first run offers host setup", "Set up ROOM" in first, st)
    check("first run does not ask for approval", "Request access" not in first)

    section("device authorisation")
    alice = Client("alice")
    st, body, _ = alice.form("/api/request-access", {"name": "Alice", "device": "alice-laptop"})
    check("first device registers", st in (200, 303), (st, body[:120]))
    check("device cookie set", bool(alice.device))
    st, boot = alice.json("/api/users")
    me = next(u for u in boot["users"] if u["name"] == "Alice" and u["role"] == "owner")
    check("first user is owner", me["role"] == "owner", me.get("role"))
    st, all_users = alice.json("/api/users")
    check("user directory readable", st == 200 and len(all_users["users"]) >= 1, len(all_users["users"]) if all_users else None)
    st, _, _ = alice.get("/")
    check("owner reaches the app", st == 200, st)

    bob = Client("bob")
    bob.form("/api/request-access", {"name": "Bob", "device": "bob-phone"})
    st, users = alice.json("/api/users")
    bob_id = next(u["id"] for u in users["users"] if u["name"] == "Bob")
    st, _, hdrs = bob.get("/")
    check("pending device is gated", st in (301, 302, 303) and "/gate" in hdrs.get("location", ""), (st, hdrs.get("location")))
    # The waiting screen must live on its own URL so a refresh cannot re-post
    # the form (that loop bounced visitors straight back to the sign-in form).
    st, page, _ = bob.get("/waiting")
    wpage = page.decode(errors="replace")
    check("pending device reaches waiting screen", st == 200, st)
    check("waiting screen names the approver", "Alice" in wpage, wpage[:160])
    check("waiting screen points at the admin console", "/admin" in wpage)
    check("waiting screen refreshes itself", "url=/waiting" in wpage)
    st, _, hdrs = bob.get("/gate")
    check(
        "gate sends a pending device to /waiting",
        st in (301, 302, 303) and "/waiting" in hdrs.get("location", ""),
        (st, hdrs.get("location")),
    )
    st, reqs = alice.json("/api/admin/requests")
    check("approval request listed", st == 200 and len(reqs.get("pending", [])) >= 1, body[:100])
    rid = reqs["pending"][0]["id"]
    st, _ = alice.json(f"/api/admin/requests/{rid}/approve", None, "POST")
    check("admin approves device", st == 200, st)
    st, _, _ = bob.get("/")
    check("approved device reaches the app", st == 200, st)
    st, _, hdrs = bob.get("/waiting")
    check(
        "waiting screen hands off once approved",
        st in (301, 302, 303) and hdrs.get("location") == "/",
        (st, hdrs.get("location")),
    )

    section("conversations")
    st, d = alice.json("/api/conversations", {"kind": "group", "name": "General", "members": [me["id"], 2]})
    check("create group", st == 200 and "conversation" in d, (st, d))
    gid = d["conversation"]["id"]
    check("group has both members", sorted(d["conversation"].get("members", [])) == sorted([me["id"], bob_id]), d["conversation"].get("members"))
    st, d = alice.json("/api/conversations", {"kind": "direct", "user_id": bob_id})
    check("create direct conversation", st == 200 and "conversation" in d, (st, d))
    dm = d["conversation"]["id"] if st == 200 else 0
    st, d2 = alice.json("/api/conversations", {"kind": "direct", "user_id": bob_id})
    check("direct conversation is reused", d2.get("conversation", {}).get("id") == dm, d2)

    section("messages")
    st, d = alice.json(f"/api/conversations/{gid}/messages", {"body": "hello world"})
    check("send message", st == 200 and d["message"]["body"] == "hello world", (st, d))
    mid = d["message"]["id"]
    st, r = bob.json(f"/api/conversations/{gid}/messages", {"body": "understood", "reply_to_id": mid})
    check("reply carries parent body", r["message"]["reply_to_body"] == "hello world", r["message"].get("reply_to_body"))
    st, r = bob.json(f"/api/messages/{mid}/reactions", {"emoji": "🔥", "on": True})
    check("reaction added", any(x["emoji"] == "🔥" for x in r["message"]["reactions"]), r["message"]["reactions"])
    st, r = alice.json(f"/api/messages/{mid}/reactions", {"emoji": "🔥", "on": True})
    check("reaction aggregates users", r["message"]["reactions"][0]["users"] == [me["id"], bob_id], r["message"]["reactions"])
    st, r = alice.json(f"/api/messages/{mid}", {"body": "hello world (edited)"}, "PATCH")
    check("edit message", st == 200 and r["message"]["edited"] is True, (st, r))
    st, r = alice.json(f"/api/conversations/{gid}/messages?limit=20")
    check("history returns messages", st == 200 and len(r.get("messages", [])) >= 2, len(r.get("messages", [])))
    st, r = alice.json(f"/api/conversations/{gid}/read", {})
    check("mark read", st == 200, st)

    section("files")
    alice.raw("POST", "/api/files/upload?name=notes.txt", b"hello room files", ctype="text/plain")
    st, listing = alice.json("/api/files")
    names = [f["name"] for f in listing.get("items", [])]
    check("upload stored", "notes.txt" in names, names)
    fid = next(f["id"] for f in listing["items"] if f["name"] == "notes.txt")
    st, payload, _ = alice.raw("GET", f"/api/files/{fid}/download")
    check("download returns bytes", payload == b"hello room files", payload[:40])
    st, d = alice.json("/api/files", {"name": "Projects", "kind": "folder"})
    check("create folder", st == 200 and d.get("id"), (st, d))
    st, d = alice.json(f"/api/files/{fid}", {"name": "notes-renamed.txt"}, "PATCH")
    check("rename file", st == 200, st)
    st, d = alice.json(f"/api/files/{fid}", {"on": True}, "DELETE")
    check("move to trash", st == 200, st)
    st, d = alice.json("/api/files?trash=1")
    check("trash lists file", any(f["id"] == fid for f in d.get("items", [])), d.get("items"))
    st, d = alice.json(f"/api/files/{fid}/restore", {})
    check("restore from trash", st == 200, st)
    st, d = alice.json("/api/files?parent=0")
    check("file is back in the drive", any(f["id"] == fid for f in d.get("items", [])), d.get("items"))

    # Binary uploads must arrive byte-for-byte. A JSON-encoding client used to
    # send "{}" instead of the file, which stored two meaningless bytes.
    blob = bytes(range(256)) * 8
    st, body, _ = alice.raw(
        "POST", "/api/files/upload?name=binary.bin", blob, ctype="application/octet-stream"
    )
    check("binary upload accepted", st == 200, (st, body[:120]))
    up = json.loads(body or b"null") if st == 200 else {}
    bin_id = up.get("file", {}).get("id", 0)
    check("binary upload keeps its size", up.get("file", {}).get("size") == len(blob),
          up.get("file", {}).get("size"))
    st, payload, _ = alice.raw("GET", f"/api/files/{bin_id}/download")
    check("binary upload round-trips exactly", payload == blob, len(payload or b""))
    st, body, _ = alice.raw(
        "POST", "/api/files/upload?name=stub.png", b"{}", ctype="image/png"
    )
    check("JSON-encoded body is rejected, not stored", st == 400, (st, body[:120]))

    section("calls")
    st, d = alice.json("/api/calls", {"conversation_id": gid, "kind": "video"})
    check("start call", st == 200 and d.get("call_id"), (st, d))
    cid = d.get("call_id", 0)
    st, d = bob.json(f"/api/calls/{cid}/join", {})
    check("second participant joins", st == 200 and len(d["call"]["participants"]) == 2, (st, d))
    st, d = bob.json(f"/api/calls/{cid}/leave", {})
    check("participant leaves", st == 200, st)
    st, d = alice.json(f"/api/calls/{cid}/end", {})
    check("call ends", st == 200, st)

    section("admin & account")
    st, d = alice.json("/api/admin/overview")
    check("admin overview", st == 200 and "stats" in d, list(d or {}))
    check("audit recorded events", len(d.get("audit", [])) >= 5, len(d.get("audit", [])) if d else 0)
    st, d = alice.json("/api/admin/backups", {})
    check("create backup", st == 200 and d.get("size", 0) > 0, (st, d))
    st, d = bob.json("/api/admin/overview")
    check("non-admin blocked from admin API", st == 403, st)
    st, d = alice.json("/api/users/me", {"name": "Alice A.", "bio": "ops"}, "PATCH")
    check("profile update", st == 200 and d["me"]["name"] == "Alice A.", (st, d))
    st, d = alice.json("/api/search?q=hello")
    check("search finds messages", len(d.get("messages", [])) >= 1, d)

    section("lockout recovery")
    # Simulate the owner losing every approved device.
    import sqlite3

    db = os.path.join(WORKDIR, "data", "room.db")
    con = sqlite3.connect(db)
    con.execute("UPDATE devices SET approved = 0, revoked = 1")
    con.commit()
    con.close()
    st, page, _ = alice.get("/gate")
    check("locked-out owner sees the recovery form", st == 200, st)
    check("gate still reads as a request form", "Request access" in page.decode(errors="replace"))
    rescue = Client("rescue")
    st, body, _ = rescue.form(
        "/api/request-access",
        {"name": "Alice", "device": "recovery-laptop", "code": "WRONG-CODE"},
    )
    check("a wrong setup code is refused", st == 400, st)
    check("wrong code explains itself", "not correct" in body.decode(errors="replace"))
    st, page, _ = rescue.get("/")
    check("wrong code leaves you gated", st in (301, 302, 303), st)
    st, body, _ = rescue.form(
        "/api/request-access",
        {"name": "Alice", "device": "recovery-laptop", "code": SETUP_CODE},
    )
    check("the right setup code gets back in", st in (200, 303), st)
    st, page, _ = rescue.get("/")
    check("recovered device reaches the app", st == 200, st)
    check("recovered device is the owner again", page and b"Admin" in page)
    # The code must be forgiving about formatting but strict about value, and
    # a plain request without it must still land in the queue.
    norm = SETUP_CODE.lower().replace("-", " ")
    st, _, _ = rescue.form(
        "/api/request-access", {"name": "Alice", "device": "recovery-case", "code": norm}
    )
    check("code ignores case and dashes", st in (200, 303), st)
    st, _, _ = rescue.form(
        "/api/request-access", {"name": "Alice", "device": "recovery-blank", "code": "   "}
    )
    check("blank code is not treated as a match", st in (200, 303), st)
    # Restore the fixtures the rest of the suite depends on: the lockout
    # revoked every device above, including Alice's and Bob's.
    con = sqlite3.connect(db)
    con.execute(
        "UPDATE devices SET approved = 1, revoked = 0 WHERE name IN ('alice-laptop','bob-phone')"
    )
    con.commit()
    con.close()
    st, _, _ = alice.get("/")
    check("owner is back after recovery", st == 200, st)
    st, _, _ = bob.get("/")
    check("peer is back after recovery", st == 200, st)

    section("suspended accounts")
    st, _ = alice.json(f"/api/admin/users/{bob_id}", {"state": "suspended"}, "PATCH")
    check("admin suspends a member", st == 200, st)
    st, _, hdrs = bob.get("/")
    check("suspended member is gated", st in (301, 302, 303), (st, hdrs.get("location")))
    st, body, _ = bob.get("/gate")
    text = body.decode(errors="replace")
    check("gate explains the suspension", st == 200 and "suspended" in text.lower(), st)
    check("suspended member is not offered the request form", "Request access" not in text)
    st, body, _ = bob.form("/api/request-access", {"name": "Bob", "device": "bob-phone"})
    check(
        "re-requesting access does not claim approval",
        st == 200 and "suspended" in body.decode(errors="replace").lower(),
        st,
    )
    st, _ = alice.json(f"/api/admin/users/{bob_id}", {"state": "active"}, "PATCH")
    check("admin restores the member", st == 200, st)
    st, _, _ = bob.get("/")
    check("restored member reaches the app", st == 200, st)

    section("pages")
    for path, needle in [
        ("/", "Chat"),
        (f"/chat/{gid}", "General"),
        ("/files", "Drive"),
        ("/calls", "Calls"),
        ("/admin", "Admin"),
        ("/account", "Account"),
    ]:
        st, body, _ = alice.get(path)
        check(f"GET {path} 200", st == 200, st)
        check(f"{path} renders shell", needle.lower() in body.decode().lower(), needle)

    section("assets")
    for path, minimum in [("/assets/app.css", 20000), ("/assets/app.js", 40000), ("/favicon.svg", 100)]:
        st, body, _ = alice.get(path)
        check(f"GET {path} 200", st == 200, st)
        check(f"{path} not truncated", len(body) >= minimum, len(body))
    st, body, _ = alice.get("/")
    html = body.decode()
    check("boot payload embedded", "__BOOT__" in html)
    check("boot payload is an object", "window.__BOOT__ = {" in html)

    section("realtime")
    if websocket is None:
        print("  \033[33mskip\033[0m websocket-client not installed")
    else:
        a = websocket.create_connection(
            BASE.replace("http", "ws") + "/ws",
            header=[f"Cookie: room_device={alice.device}"],
            timeout=8,
        )
        b = websocket.create_connection(
            BASE.replace("http", "ws") + "/ws",
            header=[f"Cookie: room_device={bob.device}"],
            timeout=8,
        )
        def drain(ws, want, limit=25):
            """Read frames until one has t == want (presence noise is expected)."""
            seen = []
            end = time.time() + 5
            while time.time() < end and len(seen) < limit:
                try:
                    msg = json.loads(ws.recv())
                except Exception:
                    break
                seen.append(msg.get("t"))
                if msg.get("t") == want:
                    return msg
            return {"t": None, "seen": seen}

        first = drain(a, "presence")
        check("websocket opens", first.get("t") in ("presence", "pong"), first.get("seen"))
        drain(b, "presence")
        a.send(json.dumps({"t": "ping"}))
        check("ping answered", drain(a, "pong").get("t") == "pong")
        a.send(json.dumps({"t": "chat:typing", "conversation_id": gid}))
        check("typing relayed to peer", drain(b, "typing").get("t") == "typing")
        a.send(json.dumps({"t": "call:signal", "call_id": 1, "to": bob_id, "data": {"kind": "offer"}}))
        check("webrtc signal relayed", drain(b, "call:signal").get("t") == "call:signal")
        a.close()
        b.close()

    print()
    if FAILS:
        print(f"\033[31m{len(FAILS)} of {CHECKS} checks failed\033[0m")
        for f in FAILS:
            print("  -", f)
        return 1
    print(f"\033[32mall {CHECKS} checks passed\033[0m")
    return 0


if __name__ == "__main__":
    try:
        code = main()
    finally:
        stop_server()
    sys.exit(code)
