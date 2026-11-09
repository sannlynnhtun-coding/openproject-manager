# Changelog

## v1.0 — 2026-11-09

- Native Windows Rust manager with start, stop, restart, logs, login details,
  tray controls, Myanmar/English and light/dark appearance.
- Offline installer bundling OpenProject 17.8.0, Ruby 4.0.6, PostgreSQL 17.11,
  production gems and compiled frontend assets.
- Separate least-privilege PostgreSQL, web and background-worker services.
- Windows OpenSSL/Prometheus compatibility, restricted-token database setup and
  concurrent cache handling.
- BoardUI appearance without changing application workflows.

Target: Windows Server 2016+ x64. Verified host: Windows 10 x64. Individual Server
versions, reboot startup and uninstall remain unverified. Installer is unsigned.
