# Nextcloud Workspace

All your Nextcloud servers in one desktop app. Each server is a workspace in the sidebar, each Nextcloud app opens in its own tab, and every server keeps its own session.

## Download

The buttons download the installer from the [latest release](https://github.com/SoluceTechnologies/nextcloud-desktop-workspace/releases/latest).

| Platform | Download |
|---|---|
| macOS · Apple Silicon | [![macOS Apple Silicon](https://img.shields.io/badge/macOS-Apple%20Silicon-0082C9?style=for-the-badge&logo=apple&logoColor=white)](https://github.com/SoluceTechnologies/nextcloud-desktop-workspace/releases/latest/download/nextcloud-workspace-aarch64-apple-darwin.dmg) |
| macOS · Intel | [![macOS Intel](https://img.shields.io/badge/macOS-Intel-0082C9?style=for-the-badge&logo=apple&logoColor=white)](https://github.com/SoluceTechnologies/nextcloud-desktop-workspace/releases/latest/download/nextcloud-workspace-x86_64-apple-darwin.dmg) |
| Windows · x64 | [![Windows x64](https://img.shields.io/badge/Windows-x64-0082C9?style=for-the-badge)](https://github.com/SoluceTechnologies/nextcloud-desktop-workspace/releases/latest/download/nextcloud-workspace-x86_64-pc-windows-msvc.exe) [![Windows x64 MSI](https://img.shields.io/badge/Windows-x64%20MSI-0082C9?style=for-the-badge)](https://github.com/SoluceTechnologies/nextcloud-desktop-workspace/releases/latest/download/nextcloud-workspace-x86_64-pc-windows-msvc.msi) |
| Windows · ARM64 | [![Windows ARM64](https://img.shields.io/badge/Windows-ARM64-0082C9?style=for-the-badge)](https://github.com/SoluceTechnologies/nextcloud-desktop-workspace/releases/latest/download/nextcloud-workspace-aarch64-pc-windows-msvc.exe) |
| Linux · x64 | [![Linux x64 AppImage](https://img.shields.io/badge/Linux-AppImage-0082C9?style=for-the-badge&logo=linux&logoColor=white)](https://github.com/SoluceTechnologies/nextcloud-desktop-workspace/releases/latest/download/nextcloud-workspace-x86_64-unknown-linux-gnu.AppImage) [![Linux x64 deb](https://img.shields.io/badge/.deb-x64-0082C9?style=for-the-badge&logo=debian&logoColor=white)](https://github.com/SoluceTechnologies/nextcloud-desktop-workspace/releases/latest/download/nextcloud-workspace-x86_64-unknown-linux-gnu.deb) [![Linux x64 rpm](https://img.shields.io/badge/.rpm-x64-0082C9?style=for-the-badge&logo=fedora&logoColor=white)](https://github.com/SoluceTechnologies/nextcloud-desktop-workspace/releases/latest/download/nextcloud-workspace-x86_64-unknown-linux-gnu.rpm) |
| Linux · ARM64 | [![Linux ARM64 AppImage](https://img.shields.io/badge/Linux-AppImage-0082C9?style=for-the-badge&logo=linux&logoColor=white)](https://github.com/SoluceTechnologies/nextcloud-desktop-workspace/releases/latest/download/nextcloud-workspace-aarch64-unknown-linux-gnu.AppImage) [![Linux ARM64 deb](https://img.shields.io/badge/.deb-ARM64-0082C9?style=for-the-badge&logo=debian&logoColor=white)](https://github.com/SoluceTechnologies/nextcloud-desktop-workspace/releases/latest/download/nextcloud-workspace-aarch64-unknown-linux-gnu.deb) [![Linux ARM64 rpm](https://img.shields.io/badge/.rpm-ARM64-0082C9?style=for-the-badge&logo=fedora&logoColor=white)](https://github.com/SoluceTechnologies/nextcloud-desktop-workspace/releases/latest/download/nextcloud-workspace-aarch64-unknown-linux-gnu.rpm) |

- **Linux:** the AppImage runs on most distributions, Arch Linux included (`chmod +x` it, then run it). Use the `.deb` on Debian and Ubuntu, the `.rpm` on Fedora and openSUSE.
- **Not signed yet:** on first launch, macOS asks for confirmation (right-click the app → **Open**) and Windows shows SmartScreen (**More info** → **Run anyway**).
- Each file has a `.sha256` checksum next to it in the release.

## Features

- Several Nextcloud servers side by side, each shown with its logo (or its initials) in the sidebar
- One tab per Nextcloud app; documents and other links opened in a new window get their own tab
- A separate browser profile per server: logins, cookies and storage never mix
- Downloads go to your Downloads folder; Talk calls with microphone and camera
- Light, dark or system appearance

## Staying signed in

A new workspace signs in with Nextcloud's Login Flow v2 (the same “Grant access” page the desktop client uses). The app password it gets is kept in the system keychain (Keychain on macOS, Credential Manager on Windows, Secret Service on Linux) and signs the workspace in again whenever the browser session expires, so the login page only comes back if the password is revoked. Existing workspaces: right-click the workspace → **Stay signed in…**. Signing out from Nextcloud, **Sign out**, **Clear browsing data** or removing the workspace deletes the app password on the server too. Signed-in workspaces also get an unread badge and system notifications.

Server administrators can make web sessions last longer in `config/config.php`:

```php
'session_lifetime' => 60 * 60 * 24 * 7,           // idle web session, default 1 day
'session_keepalive' => true,                       // open pages keep the session alive (default)
'remember_login_cookie_lifetime' => 60 * 60 * 24 * 30, // "remember me" cookie, default 15 days
```

## Requirements

macOS 14 or later, Windows 10/11 (WebView2), or Linux with WebKitGTK 4.1.

## Development

Requires Node 22 and Rust (stable).

```bash
npm ci
npm run tauri dev
```

```bash
npm test
cargo test --manifest-path src-tauri/Cargo.toml
```

```bash
npx tauri build
```

- `src/`: the shell UI (React): `app/`, `features/`, `components/`, `lib/`, `styles/`; tests in `tests/`
- `src-tauri/`: the Rust side: workspaces, tabs, webviews, and the bridge injected into Nextcloud pages

## Releases

Merging a pull request into `main` publishes a release: the version comes from the conventional commits, installers are built for every platform above and attached to the GitHub release. Put `[skip-release]` in the pull request title to merge without releasing.

## License

[GNU General Public License v3.0](LICENSE)

## Disclaimer

Nextcloud Workspace is an independent project, not affiliated with or endorsed by Nextcloud GmbH.
