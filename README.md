<img width="1794" height="1092" alt="image" src="https://github.com/user-attachments/assets/d5548357-ee9f-457b-ad93-79b2ff5f5760" />


# Build Instructions

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
sudo setcap 'cap_net_bind_service=+ep' target/debug/go_service
target/debug/go_service
```

For a system service, set the same env var before launching the binary, for example:
```text
Environment=PORT=8080
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
  --name go-alias \
  --restart unless-stopped \
  -p 80:8080 \
  -e PORT=8080 \
  go-alias-rust
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

### macOS and Linux

Run from the repo root:

```sh
sudo ./scripts/install-local-unix.sh
```

Linux installs to:

```text
/opt/go-alias
/etc/systemd/system/go-alias.service
```

macOS installs to:

```text
/usr/local/go-alias
/Library/LaunchDaemons/com.go-alias.plist
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
C:\Program Files\GoAlias
```

and registers a `GoAlias` startup scheduled task. The scheduled task is used instead of a plain Windows service so the app starts with `C:\Program Files\GoAlias` as its working directory and can find its `static/` assets.

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
Mistype any shortcuts to see all your shortcuts
- has a table of all the shortcuts from the shortcuts.json
- has a nav bar at top of all other tools with this tool

# /sql

have form with submit button to input a new connection that contains
- Nickname
- Host
- Database name
- user
- password
then
- securely save on encrypted file 
- offer as selection of all the connections and default to last used

# /calc or /calculator

- top bar asking for basic, scientific
- calculator buttons are below the input output line of what is being input.
- below is a history of what has been inputted.

# /request

- basically a simple postman where you can save post requests if you need to
