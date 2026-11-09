# OpenProject Manager for Windows v1.0

Independent Rust desktop manager for the native Windows OpenProject port in this
repository. Docker, WSL, Linux, Java, Node and a separate database installation are
not required on the destination machine.

The web application includes the BoardUI visual adaptation: locally bundled Inter,
rounded neutral navigation, blue selections and common component styling in light
and dark appearance. Existing OpenProject actions and workflows remain intact.
See `BOARDUI-THEME.md` in the installation directory for scope and verification.

## Install and use

Run `OpenProjectManager-Setup.exe` and approve the Windows administrator prompt.
Download the installer and SHA-256 checksums from the
[v1.0 release](https://github.com/sannlynnhtun-coding/openproject-manager/releases/tag/v1.0).
The offline installer includes OpenProject 17.8.0 with the Windows patches, Ruby
4.0.6, the production gems, precompiled frontend assets, PostgreSQL 17.11 and
WinSW 2.12.0 (.NET Framework 4.6.1, included in Server 2016 and later).

First setup creates a private SCRAM-authenticated database, runs migrations and
seeds, and installs these automatically starting services:

- `OpenProjectPostgreSQL` as NetworkService on `127.0.0.1:15434`.
- `OpenProjectWeb` as LocalService, initially at `http://127.0.0.1:8088`.
- `OpenProjectWorker` as LocalService, for GoodJob background processing.

Wait for **Web: Ready**, then click **Open OpenProject**. Startup can take several
minutes. **Login details** opens the generated initial admin credentials, readable
only by administrators and SYSTEM. A password subsequently changed in OpenProject
is not written back to this file.

The manager supports Start / Stop / Restart, host/port settings, logs, tray
controls, English/Myanmar and light/dark appearance. Closing its window hides it
in the tray; Exit closes the manager while the services continue running.
Stop all services before changing settings. `0.0.0.0` binds all IPv4 interfaces;
firewall and HTTPS configuration remain the administrator's responsibility.

Data lives under `%PROGRAMDATA%\OpenProjectManager`: PostgreSQL data, attachments,
logs, and private credentials/signing keys. Back up the database, files and private
credentials together. The application role is not a database superuser. The
application account can read the application credentials but not the database
files, separate database-administrator credential or initial-login file.

Reinstalling this version preserves existing files, database and credentials and
restores missing bundled files, while refreshing the setup and service launch
scripts and BoardUI theme code. Restart the web service from the manager after
repair to reload cached templates if upgrading from an earlier appearance.
Automatic version upgrades and migration of the
earlier per-user prototype at port 8087 are not implemented. A service with one
of the same names but a different executable path is rejected. Uninstall asks
whether to stop/remove the services; data and runtime files are retained either
way so running or retained services are not left without their runtimes.

## Build

Requires Rust MSVC, Visual Studio C++ Build Tools, Inno Setup 6.5+ on PATH, and
the verified native runtime/gems/assets from the preceding native build under
`%LOCALAPPDATA%\OpenProject-native-probe`. No prototype database, attachments or
credentials are packaged. Source and build scripts are in this repository.

From a standalone clone of this repository:

```powershell
git clone https://github.com/sannlynnhtun-coding/openproject-manager.git
cd openproject-manager
.\scripts\prepare-source.ps1
.\scripts\build.ps1 -SourceRoot "$env:LOCALAPPDATA\OpenProjectManager-build\source"
```

Source preparation pins OpenProject to the revision in [UPSTREAM.md](UPSTREAM.md)
and applies the three ordered port patches. The build still requires the verified
portable Ruby, production gems, PostgreSQL and compiled assets described above;
it does not download or build those runtime prerequisites automatically. In the
original parent OpenProject checkout, `scripts/build.ps1` also accepts that
checkout as `-SourceRoot` (and defaults to the parent directory).

Staging, Cargo output and installer compilation default to
`%LOCALAPPDATA%\OpenProjectManager-build`. Cargo respects an existing
`CARGO_TARGET_DIR`. The installer is copied to `windows-manager\dist` when there
is enough disk space; otherwise the build prints its location on the build drive.

The GPUI Windows dependency includes the existing Server 2016 compatibility
patch for DirectWrite, DXGI and system ICU. The build runs Rust tests, the GPUI
compatibility tests, an executable import check, native bundle relocation checks
with a restricted PATH, and service XML checks. Ruby and gem licenses remain in
their vendor directories; OpenProject source, license and copyright are included.

## Verification limits

Target: Windows Server 2016, 2019, 2022 and 2025 x64 with Desktop Experience for
the manager GUI. The backend services do not require an interactive desktop.
Actual execution on each Server version still requires verification. Elevated
installation on the Windows 10 build host verified SCM registration, startup as
NetworkService/LocalService, admin login, frontend delivery and completed jobs.
Automatic startup after reboot and uninstall remain untested. The local native login does not
certify email, BIM converters, external integrations, PDF tools or backup/restore.
