# Source provenance

OpenProject upstream: https://github.com/opf/openproject

Pinned revision: `36d730dca55d092336b5f6d806e9b2a56eecd70c` (OpenProject 17.8.0).

The ordered patches contain the Windows Ruby/OpenSSL/Prometheus changes,
Windows FileStore compatibility and BoardUI appearance. They include their new
files, tests, stylesheet, local Inter font and font license. Upstream application
workflows remain intact. `scripts/prepare-source.ps1` applies them to a new
directory and refuses to overwrite an existing checkout.

OpenProject is GPL-3.0; its license is reproduced in LICENSE. The installer
includes the patched application source and copyright notices. The manager is
distributed under GPL-3.0 as well. The vendored gpui-pre-windows compatibility
patch retains its Apache-2.0 license. WinSW retains its MIT license in the bundle;
Ruby, PostgreSQL and gem notices remain in their runtime/vendor directories.
Inter's OFL license is included in the appearance patch and installed font folder.

This repository is an independent Windows port/manager, not an official
OpenProject Windows distribution. BoardUI is a visual reference; its React
component packages and paid templates are not redistributed.
