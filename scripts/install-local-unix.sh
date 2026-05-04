#!/usr/bin/env bash
set -euo pipefail

APP_NAME="go-alias"
BIN_NAME="go_service"
PORT="${PORT:-80}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ "${EUID}" -ne 0 ]]; then
  echo "This installer needs sudo/root so it can write hosts and install a startup service."
  echo "Run: sudo $0"
  exit 1
fi

ORIGINAL_USER="${SUDO_USER:-$(id -un)}"

if [[ -f "${SCRIPT_DIR}/Cargo.toml" ]]; then
  REPO_DIR="${SCRIPT_DIR}"
elif [[ -f "${SCRIPT_DIR}/../Cargo.toml" ]]; then
  REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
else
  REPO_DIR=""
fi

if [[ -n "${REPO_DIR}" ]]; then
  SOURCE_BINARY="${REPO_DIR}/target/release/${BIN_NAME}"
  STATIC_SOURCE="${REPO_DIR}/static"
else
  SOURCE_BINARY="${SCRIPT_DIR}/${BIN_NAME}"
  STATIC_SOURCE="${SCRIPT_DIR}/static"
fi

case "$(uname -s)" in
  Darwin)
    INSTALL_DIR="/usr/local/${APP_NAME}"
    SERVICE_FILE="/Library/LaunchDaemons/com.${APP_NAME}.plist"
    ;;
  Linux)
    INSTALL_DIR="/opt/${APP_NAME}"
    SERVICE_FILE="/etc/systemd/system/${APP_NAME}.service"
    ;;
  *)
    echo "Unsupported OS: $(uname -s)"
    exit 1
    ;;
esac

if [[ -n "${REPO_DIR}" ]]; then
  echo "Building release binary..."
  if command -v sudo >/dev/null 2>&1 && [[ "${ORIGINAL_USER}" != "root" ]]; then
    sudo -u "${ORIGINAL_USER}" cargo build --release --manifest-path "${REPO_DIR}/Cargo.toml"
  else
    cargo build --release --manifest-path "${REPO_DIR}/Cargo.toml"
  fi
else
  echo "Using bundled release binary..."
fi

echo "Installing files into ${INSTALL_DIR}..."
mkdir -p "${INSTALL_DIR}"
install -m 0755 "${SOURCE_BINARY}" "${INSTALL_DIR}/${BIN_NAME}"
rm -rf "${INSTALL_DIR}/static"
cp -R "${STATIC_SOURCE}" "${INSTALL_DIR}/static"

echo "Ensuring local hostname 'go' resolves to this machine..."
if ! grep -Eq '(^|[[:space:]])go([[:space:]]|$)' /etc/hosts; then
  {
    echo ""
    echo "# ${APP_NAME} local browser alias"
    echo "127.0.0.1 go"
    echo "::1 go"
  } >> /etc/hosts
fi

if [[ "$(uname -s)" == "Linux" ]]; then
  echo "Installing systemd service..."
  cat > "${SERVICE_FILE}" <<EOF
[Unit]
Description=Go Alias developer tool
After=network.target

[Service]
Type=simple
WorkingDirectory=${INSTALL_DIR}
Environment=PORT=${PORT}
ExecStart=${INSTALL_DIR}/${BIN_NAME}
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
EOF

  systemctl daemon-reload
  systemctl enable --now "${APP_NAME}.service"
  systemctl status "${APP_NAME}.service" --no-pager || true
else
  echo "Installing launchd service..."
  cat > "${SERVICE_FILE}" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.${APP_NAME}</string>
  <key>ProgramArguments</key>
  <array>
    <string>${INSTALL_DIR}/${BIN_NAME}</string>
  </array>
  <key>WorkingDirectory</key>
  <string>${INSTALL_DIR}</string>
  <key>EnvironmentVariables</key>
  <dict>
    <key>PORT</key>
    <string>${PORT}</string>
  </dict>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
</dict>
</plist>
EOF

  chmod 0644 "${SERVICE_FILE}"
  chown root:wheel "${SERVICE_FILE}"
  launchctl unload "${SERVICE_FILE}" >/dev/null 2>&1 || true
  launchctl load -w "${SERVICE_FILE}"
fi

echo ""
echo "Installed. Open http://go/ or http://go/<alias> in a browser."
