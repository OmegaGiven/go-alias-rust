echo "1. Building the Rust application..."
cargo build

echo "2. Setting network capability on new binary..."
sudo setcap 'cap_net_bind_service=+ep' target/debug/go_service

echo "3. Running the application directly..."
target/debug/go_service