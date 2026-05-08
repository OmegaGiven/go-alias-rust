<img width="1920" height="1010" alt="image" src="https://github.com/user-attachments/assets/785541c4-0656-474f-82ac-949dc5ee5eb1" />

# OGdevDesk

OGdevDesk is a focused developer desk for API requests, database work, JSON inspection, web aliases, scratch notes, and small utility tools.

## License

OGdevDesk is licensed under the Apache License, Version 2.0.

```text
Copyright 2026 OmegaGiven
```

See [LICENSE](LICENSE) and [NOTICE](NOTICE).

## Build Instructions

1. Run the app locally:
```sh
PORT=8080 cargo run
```
Then open:
```text
http://localhost:8080
```
If you do not set `PORT`, the app defaults to port 80.

2. Build the Rust app:
```sh
cargo build
```
or, if you want port 80, you may need to grant privileges first:
```sh
cargo build
sudo setcap 'cap_net_bind_service=+ep' target/debug/ogdevdesk_service
target/debug/ogdevdesk_service
```

For a system service, set the same env var before launching the binary, for example:
```text
Environment=PORT=8080
```

## Desktop App Development

The desktop app is a Tauri wrapper around the same Actix server, templates, static assets, and app database code used by the browser version. UI and backend changes should still be made in the shared Rust, `templates/`, and `static/` files so both launch modes stay unified.

Run the browser/server version from the repo root:

```sh
PORT=8080 cargo run
```

Run the desktop wrapper from the repo root:

```sh
npm run tauri:dev
```

or from `src-tauri/`:

```sh
cargo tauri dev
```

Desktop mode starts the shared server on `127.0.0.1` using an available local port, then opens a native Tauri window pointed at that server. The desktop database defaults to the operating system app-data directory and can still be overridden with `OGDEVDESK_DB_PATH`.

## Appearance Themes

Open `Tools -> Appearance` to adjust theme mode, typography, accent colors, window accent color, margins, and nav height. Use `Export` to download the current appearance as an `.ogdevdesk-theme.json` file. Use `Import` to load one of those files back into the Appearance window, then apply or save it.

The repo includes a starter preset based on the current OGdevDesk environment theme:

```text
presets/themes/im-blue.ogdevdesk-theme.json
presets/themes/helldiver.ogdevdesk-theme.json
```

## Making `go/alias` Work

The app already routes `/alias` paths. To type this in a browser:

```text
http://go/
http://go/gh
```

the hostname `go` must resolve to the machine running this app.

### Local Machine Only

If the app only needs to work on your own machine, add `go` beside `localhost` in your hosts file.

macOS/Linux:

```sh
sudo nano /etc/hosts
```

Example:

```text
127.0.0.1       localhost go
255.255.255.255 broadcasthost
::1             localhost go
```

Windows:

```text
C:\Windows\System32\drivers\etc\hosts
```

Example:

```text
127.0.0.1       localhost go
::1             localhost go
```

Then run the app on port 80:

```sh
PORT=80 cargo run
```

or run on another port and include it in the URL:

```text
http://go:8080/
http://go:8080/gh
```

### LAN or Private VPN

If other users should be able to type `go/alias` from their browsers, use internal DNS.

Create a DNS record:

```text
go -> <server LAN/VPN IP>
```

Then run the app on that server and expose port 80. The flow should be:

```text
Browser -> http://go/alias -> internal DNS resolves go -> server:80 -> app routes /alias
```

If you do not have internal DNS, each user can add a hosts-file entry pointing `go` to the server IP.

Example:

```text
10.20.30.40     go
```

Then users can open:

```text
http://go/
http://go/gh
```

### Docker Deployment

Docker works as long as port 80 on the host forwards to the app inside the container.

Example:

```sh
docker run -d \
  --name ogdevdesk \
  --restart unless-stopped \
  -p 80:8080 \
  -e PORT=8080 \
  ogdevdesk
```

Users still need DNS or a hosts-file entry for `go`.

### Browser Note

Some browsers may search for `go/alias` instead of navigating to it. `http://go/alias` is the reliable form. Internal DNS or hosts-file entries make `go` resolvable, but browser behavior can still vary for bare single-label names.

## Local Installers

This repo includes installer scripts for local-machine installs. They are not signed OS package installers yet, but they perform the important setup steps:

- build the release binary
- copy the app binary and `static/` assets into a system install directory
- add `go` to the local hosts file
- install a startup service
- run the app on port 80 so `http://go/` and `http://go/<alias>` work

### GitHub Release Downloads

For the easiest browser/server install, download the one server release asset that matches your system:

```text
OGdevDesk-server-linux-x64.tar.gz
OGdevDesk-server-macos-arm64.tar.gz
OGdevDesk-server-windows-x64.zip
```

Each archive contains the prebuilt app, static assets, README, and one installer script at the top level.

Linux:

```sh
tar -xzf OGdevDesk-server-linux-x64.tar.gz
cd OGdevDesk-server-linux-x64
sudo ./install-linux.sh
```

macOS:

```sh
tar -xzf OGdevDesk-server-macos-arm64.tar.gz
cd OGdevDesk-server-macos-arm64
sudo ./install-macos.sh
```

Windows:

```powershell
Expand-Archive .\OGdevDesk-server-windows-x64.zip
cd .\OGdevDesk-server-windows-x64\OGdevDesk-server-windows-x64
.\install-windows.ps1
```

Desktop app releases are published separately with one obvious file per platform:

```text
OGdevDesk-desktop-macos-arm64.zip
OGdevDesk-desktop-windows-x64.msi
OGdevDesk-desktop-linux-x64.AppImage
```

### macOS and Linux

Run from the repo root:

```sh
sudo ./scripts/install-local-unix.sh
```

Linux installs to:

```text
/opt/ogdevdesk
/etc/systemd/system/ogdevdesk.service
```

macOS installs to:

```text
/usr/local/ogdevdesk
/Library/LaunchDaemons/com.ogdevdesk.plist
```

After install:

```text
http://go/
http://go/gh
```

### Windows

Run from an elevated PowerShell prompt:

```powershell
.\scripts\install-local-windows.ps1
```

Windows installs to:

```text
C:\Program Files\OGdevDesk
```

and registers an `OGdevDesk` startup scheduled task. The scheduled task is used instead of a plain Windows service so the app starts with `C:\Program Files\OGdevDesk` as its working directory and can find its `static/` assets.

After install:

```text
http://go/
http://go/gh
```

### Installer Limits

These scripts set up `go` for the local machine only. For multiple users on a LAN or private VPN, use internal DNS so `go` points to the shared server IP, or add the same hosts-file mapping on each client machine.

The app must use port 80 for `http://go/` with no port in the URL. If port 80 is already used by another app, either stop the conflicting service or use an explicit port such as `http://go:8080/`.


## USAGE: In browser type localhost:PORT or whatever alias you use for localhost
# /

<img width="1920" height="1010" alt="image" src="https://github.com/user-attachments/assets/377fa6fe-f09d-466c-8171-99569d7287f6" />

Mistype any shortcuts to see all your shortcuts
- has a table of all the shortcuts from the shortcuts.json
- has a nav bar at top of all other tools with this tool

# /sql

<img width="1920" height="1010" alt="image" src="https://github.com/user-attachments/assets/e2eb4f7c-c7f3-42ad-8850-2216e76550df" />

have form with submit button to input a new connection that contains
- Nickname
- Host
- Database name
- user
- password
then
- securely save on encrypted file 
- offer as selection of all the connections and default to last used

# /request

<img width="1920" height="1013" alt="image" src="https://github.com/user-attachments/assets/50280ef3-d00a-418b-9ebd-374b41bc37e8" />


- basically a simple postman where you can save post requests if you need to

# /Inspector

<img width="1920" height="1013" alt="image" src="https://github.com/user-attachments/assets/077c59d6-f585-4b8f-b419-d53d1c908388" />

- able to parse / prettify Json for easy searching and viewing
