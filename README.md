# ROOM

**Private. Local. Fast. Reliable.**

ROOM is a self-hosted, local-first communication and collaboration platform. It turns an
ordinary computer into a private server that authorised devices on the same Wi-Fi or local
network can use to chat, call, share files and manage documents — with no cloud account, no
external database and no third-party storage.

- **Discord-style messaging** with real-time WebSockets, reactions, replies, edits, typing
  indicators, presence and read state.
- **Voice & video calls** with WebRTC (mesh) and a Rust signalling server.
- **Drive-style file management**: streaming uploads, nested folders, rename/move/copy,
  trash, search, previews and a storage meter.
- **Host console**: server status, storage, connected devices, pending access requests,
  members, audit log, backups, configuration, safe restart/shutdown.
- **Persistent device authorisation**: a device asks once, the host approves it, and the
  credential survives restarts until it is revoked.

## Stack

| Layer      | Choice                                                        |
|------------|---------------------------------------------------------------|
| Server     | Rust · Actix Web · Tokio · WebSockets (actix-web-actors)      |
| Media      | WebRTC signalling (Rust), peer-to-peer media on the LAN       |
| Storage    | SQLite (SQLx, WAL) + files on the host filesystem             |
| Frontend   | Server-rendered HTML **inside `.rs` files** + Tailwind CSS + vanilla JS |

No Next.js, no React runtime, no build step for the browser.

## Run

```bash
cargo run --release
```

Then open `http://<host-ip>:8080` from any device on the same network.

Environment variables:

| Variable             | Default        | Meaning                                  |
|----------------------|----------------|------------------------------------------|
| `ROOM_BIND`          | `0.0.0.0:8080` | Bind address                             |
| `ROOM_DATA_DIR`      | `./data`       | SQLite database + backups                |
| `ROOM_FILES_DIR`     | `./data/files` | Uploaded blobs                           |
| `ROOM_NAME`          | `ROOM`         | Server name shown in the UI              |
| `ROOM_MAX_UPLOAD_MB` | `2048`         | Per-file upload limit                    |
| `ROOM_NETWORK_NAME`  | –              | Wi-Fi name hint on the sign-in screen    |
| `ROOM_SETUP_CODE`    | generated      | Recovery code; see *First run* below     |

## First run

The **first** device to open the server becomes the **owner** (host) with no code and no
approval — before an owner exists there is nobody who could approve anyone, so the gate reads
"Set up ROOM" instead of asking for permission. Every later device must be approved from the
host console (`/admin?tab=requests`). Approvals persist across restarts; revoking a device
invalidates its credential immediately.

### Setup code (lockout recovery)

On first start ROOM generates a setup code and prints it:

```
   +--------------------------------------------+
   |  Setup code: K7QM-3XR9                     |
   +--------------------------------------------+
```

It is also shown in the admin overview. Typing it on the gate signs you in as the **owner**,
without approval, and is the only way back in if every approved device is lost — a stolen
laptop, cleared cookies, or a device revoked by mistake. Without it, a server whose owner has
no approved device has no way in at all, short of editing the database by hand.

To pin your own code, set `ROOM_SETUP_CODE` before starting:

```bash
ROOM_SETUP_CODE="my-recovery-code" ./room
```

The code is case-insensitive and ignores dashes. Treat it like a password: anyone who has it
and can reach the server becomes the owner.

## Development

```bash
npm install @tailwindcss/cli   # one-off: compiles assets/app.css -> assets/app.dist.css
npx @tailwindcss/cli -i assets/app.css -o assets/app.dist.css --minify
cargo fmt && cargo clippy
```

HTML lives in `src/templates/*.rs` (inline `const` strings), the stylesheet in
`assets/app.css` (Tailwind v4 CSS-first config) and the client behaviour in `assets/app.js`.

## Testing

Two suites run against the compiled binary. Run them with no arguments and each one starts
`dist/room` on port 8123 against a temporary data directory, so every run begins from a virgin
database (the first device to register becomes the owner). Pass a base URL to test a server that
is already running instead.

```bash
# end-to-end API + WebSocket coverage (74 checks)
pip install websocket-client
python3 scripts/smoke.py                # or: python3 scripts/smoke.py http://host:8080

# client interaction coverage in jsdom (46 checks)
npm install --no-save jsdom
node scripts/dom.js                     # or: node scripts/dom.js http://host:8080
```

`scripts/smoke.py` covers the access gate, device approval, conversations (group and direct),
messages, replies, reactions, edits, files (upload, download, rename, trash, restore), calls,
admin (overview, audit, backups, role enforcement), every page, the served assets and the
realtime channel (presence, ping, typing, WebRTC signalling).

`scripts/dom.js` boots each page with the real `assets/app.js` and asserts the `__BOOT__`
payload, the bubble/day-divider rendering and the conversation filter pills, then drives the
new-conversation and new-group pickers, the composer, the theme toggle, conversation search and
filters, the reaction picker and the file row menu.
