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

## Install (pick your OS)

Every [release](https://github.com/OmegaGiven/go-alias-rust/releases) ships a
one-step installer for each platform. Download the file for your OS, run it,
and the service is installed, registered to start at boot, and running —
no Rust toolchain, no manual setup.

- **Linux**: download the `.deb`, then `sudo apt install ./go-alias-rust_*.deb`
  (or `sudo dpkg -i ./go-alias-rust_*.deb`). Installs to `/usr/bin`, sets up
  a `go-alias-rust` systemd service, enabled and started automatically.
- **macOS**: download the `.pkg`, double-click it, follow the installer.
  Installs a LaunchDaemon that starts at boot and runs as root (needed for
  port 80).
- **Windows**: download the `.msi`, run it. Installs to
  `Program Files\go-alias-rust` and registers a Scheduled Task that starts
  the service at boot running as `SYSTEM` (needed for port 80).

Shortcut data lives outside the install location so upgrades don't touch it:
`/var/lib/go-alias-rust` (Linux), `/usr/local/var/go-alias-rust` (macOS),
`%ProgramData%\go-alias-rust` (Windows).

To uninstall: use your OS's normal package manager / "Add or Remove
Programs" — each installer registers a proper uninstaller that stops the
service and (optionally, on Linux via `purge`) removes the data directory.

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

### Building from source instead

If you'd rather not use the installer (e.g. you're modifying the code):

```bash
git clone https://github.com/OmegaGiven/go-alias-rust.git
cd go-alias-rust
cargo build --release
```

Port 80 needs elevated privileges. Rather than running the whole process as
root, grant just the binary permission to bind low ports:

```bash
sudo setcap 'cap_net_bind_service=+ep' target/release/go_service
./target/release/go_service
```

To run it persistently at boot yourself (this is what the Linux installer's
`.deb` already sets up automatically), use the systemd unit at
`packaging/linux/go-alias-rust.service` as a template — update its
`WorkingDirectory` and `ExecStart` paths to wherever you built the binary,
place it in `/etc/systemd/system/`, then:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now go-alias-rust.service
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
