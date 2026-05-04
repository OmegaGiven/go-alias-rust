<img width="1794" height="1092" alt="image" src="https://github.com/user-attachments/assets/d5548357-ee9f-457b-ad93-79b2ff5f5760" />


# Build Instructions
1. add the following file
  shortcuts.json
  the content is the following format
  {
    "alias": "url",
    "alias": "url"
  }
2. run the app locally:
```sh
PORT=8080 cargo run
```
Then open:
```text
http://localhost:8080
```
If you do not set `PORT`, the app defaults to port 80.

3. build rust app:
```sh
cargo build
```
or if you want port 80, you may need to grant privileges first (the run.sh file is committed into git repo if you want to just use that.)
```sh
cargo build
sudo setcap 'cap_net_bind_service=+ep' target/debug/go_service
target/debug/go_service
```
for a system service, set the same env var before launching the binary, for example:
```text
Environment=PORT=8080
```
4. edit to add your own alias as localhost, i personally like "go" but you can use anything.
file found at
/etc/hosts

here is what my mac system looks like:
127.0.0.1	localhost go
255.255.255.255	broadcasthost
::1             localhost go


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
