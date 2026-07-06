# go-alias-rust

Tiny Rust web service that lets you type `go/<alias>` in a browser URL bar
and get redirected to whatever URL you've saved for that alias. Visiting
`go/` (or any alias that doesn't match) shows a table of every saved
shortcut.

## How it works

- Binds to port 80 on `0.0.0.0`.
- Shortcuts are stored in flat JSON files (`shortcuts.json`,
  `hidden-shortcuts.json`, `work-shortcuts.json`) as `{ "alias": "url" }`
  pairs, loaded on startup and rewritten whenever you add/delete one from
  the web UI.
- Visiting `/<alias>` looks up the alias and issues an HTTP redirect to its
  URL. `/<alias>/<extra>` also works — it appends `<extra>` onto the saved
  URL (e.g. `go/gh/OmegaGiven` -> `https://github.com/OmegaGiven`).
- Anything that doesn't match a saved alias renders the shortcuts table
  instead of a blank 404.
- A basic theme system (colors/fonts) is included, editable from the
  Settings button in the nav bar.

## Requirements

- Rust (edition 2024) + Cargo
- Linux/macOS to bind port 80 without extra config (see below); Windows
  works too but the "run on port 80 as non-root" step differs.

## Install & Run

```bash
git clone https://github.com/OmegaGiven/go-alias-rust.git
cd go-alias-rust
cargo build --release
```

### Running on port 80 without root

Port 80 needs elevated privileges on Linux. Rather than running the whole
service as root, grant just the one binary permission to bind low ports:

```bash
sudo setcap 'cap_net_bind_service=+ep' target/release/go_service
./target/release/go_service
```

(macOS: just run it with `sudo` directly, or use a reverse proxy/launchd
socket activation instead.)

### Aliasing "go" to localhost

So that typing `go/<alias>` in your browser actually resolves, add an entry
to your hosts file pointing `go` at wherever the service runs:

- Linux/macOS: `/etc/hosts`
- Windows: `C:\Windows\System32\drivers\etc\hosts`

```
127.0.0.1   localhost go
```

If you're running the service on a different machine on your network, use
its IP instead of `127.0.0.1`.

### Running as a systemd service (Linux, recommended for a home server)

Create `/etc/systemd/system/go.service`:

```ini
[Unit]
Description=Go Alias Redirect Service
After=network.target

[Service]
Type=simple
User=<your-username>
WorkingDirectory=/path/to/go-alias-rust
ExecStart=/path/to/go-alias-rust/target/release/go_service
Restart=always

[Install]
WantedBy=multi-user.target
```

Then:

```bash
sudo setcap 'cap_net_bind_service=+ep' /path/to/go-alias-rust/target/release/go_service
sudo systemctl daemon-reload
sudo systemctl enable --now go.service
```

## Configuring shortcuts

Edit `shortcuts.json` directly, or use the **+ Add Shortcut** button on the
home page once the service is running. Format:

```json
{
  "gh": "https://github.com/",
  "yt": "https://www.youtube.com/"
}
```

Restart isn't required — shortcuts save to disk immediately and take effect
on the next request.
