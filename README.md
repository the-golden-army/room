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

## First run

The first device that requests access becomes the **owner** (host). Every later device must be
approved from the host console (`/admin?tab=requests`). Approvals persist across restarts;
revoking a device invalidates its credential immediately.

## Development

```bash
npm install @tailwindcss/cli   # one-off: compiles assets/app.css -> assets/app.dist.css
npx @tailwindcss/cli -i assets/app.css -o assets/app.dist.css --minify
cargo fmt && cargo clippy
```

HTML lives in `src/templates/*.rs` (inline `const` strings), the stylesheet in
`assets/app.css` (Tailwind v4 CSS-first config) and the client behaviour in `assets/app.js`.
