Build Instructions
1. add the following file
  shortcuts.json
  the content is the following format
  {
    "alias": "url",
    "alias": "url"
  }
2. build rust app: cargo run
or if you have to set privilages for the localhost port it uses (the run.sh file is committed into git repo if you want to just use that.)
cargo build
sudo setcap 'cap_net_bind_service=+ep' target/debug/go_service
target/debug/go_service
3. edit to add your own alias as localhost, i personally like go but you can use anything.
file found at
/etc/hosts


## USAGE: In browser type localhost or whatever alas you may use for localhost/alias
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

# /note

- if you paste a json or dict it will auto format
- has a basic save button just historically store whatever notes you want
- previews a note below and when clicked it loads it into the text section
- delete button next to the note
- has a way to send as a composed message to google
- be able to open .txt and .md files and preview .md files

# /calc or /calculator

- top bar asking for basic, scientific
- calculator buttons are below the input output line of what is being input.
- below is a history of what has been inputted.

# /paint

- basic paint functionality

# /request

- basically a simple postman where you can save post requests if you need to
