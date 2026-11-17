# ROOM

ROOM is a self-hosted, local-first communication and collaboration workspace. The current branch starts with an Actix Web server and a responsive chat interface based on the supplied reference design. The HTML, styles, and browser-side interactions are embedded in `src/main.rs`; there are no standalone `.html` files.

## Run

Install Rust, then start the server:

```sh
cargo run
```

By default, ROOM listens on `0.0.0.0:8080` so other devices on the local network can reach it. Set `ROOM_BIND` to change the bind address, for example `ROOM_BIND=127.0.0.1:8080 cargo run`.

- `/` — chat workspace
- `/health` — server health JSON
- `/api/info` — basic server information

The interface is mobile-first and includes responsive conversation navigation, seeded sample conversations, chat search and filters, theme switching, and client-side message composition. Tailwind's browser CDN is included for utility styling; the custom interface styles are embedded alongside the page so the primary layout still has its own styling if that CDN is unavailable.

## Current implementation boundary

This is the first UI/server foundation, not yet a complete collaboration server. Conversation examples and newly composed messages currently live only in the browser and reset on reload. Authentication and device approval, durable SQLite storage, WebSockets, file uploads, calls, real encryption, and administration are not implemented yet. The UI deliberately avoids claiming that messages are end-to-end encrypted.
