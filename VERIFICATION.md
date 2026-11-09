# v1.0 release verification — 2026-11-09

Installer: `OpenProjectManager-Setup.exe` (434359317 bytes).

SHA256: `2E4B72B3BEA3AFB15594EB7055959E21E64633FCE143280876C1970824F585C2`.

Manager executable and installer product versions: `1.0.0`. Release tag: `v1.0`.

Verified on Windows 10 x64:

- Rust backend tests: 3 passed; GUI/resource tests: 2 passed; GPUI Server compatibility tests: 2 passed.
- Executable import check excludes system ICU dependencies.
- Windows PowerShell service XML, quoting and ACL-descriptor tests passed. Descriptor tests mock ACL application.
- Native relocated Ruby/OpenSSL/Prometheus and concurrent FileStore checks passed with a restricted PATH.
- All three ordered patches apply to the pinned upstream files and reproduce the Windows port, including the binary Inter font.
- Source preparation parses and refuses to overwrite an existing directory.
- Installer compilation succeeded; copied artifact SHA256 matches the build output.
- Publish source excludes local credentials, database state, build output and runtime secrets.

Earlier builds were installed on this PC: PostgreSQL as NetworkService and web/worker as LocalService. Fresh database migration/seed, CSRF-protected admin login, frontend delivery and background jobs passed. The installed login test also verifies `/api/v3/users/me` reports `admin`. The user confirmed browser login succeeded. Repair preserved database credentials and initialization state.

The BoardUI appearance was inspected on home, Projects, Work packages/filter, Kanban, account settings and isolated sign-in at desktop/tablet/mobile sizes. Opening and cancelling the creation form did not save a work package. Themed and unthemed controls retained their link/form semantics. The final theme refinements are included in this installer; the current PC installation was not automatically upgraded.

Not verified: installing this exact v1.0 installer again, reboot startup, uninstall, or execution on Windows Server 2016/2019/2022/2025. The installer is unsigned. The GUI requires Desktop Experience. Targeting a Server version does not certify execution on it. Fresh source builds require the documented native runtime/gem/asset prerequisites; this release does not automate those prerequisites or claim pixel-identical BoardUI Pro templates.
