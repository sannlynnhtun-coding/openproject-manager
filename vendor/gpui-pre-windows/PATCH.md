# Local compatibility patch

Source: crates.io `gpui-pre-windows` 0.3.9 (Apache-2.0).

`src/destination_list.rs` uses a bounded Rust slice search instead of
`Globalization::u_strlen`, removing the `icuuc.dll` import that prevents startup
on Windows Server 2016.

`src/direct_write.rs` uses `IDWriteFactory4` and file-backed custom fonts instead
of the newer `IDWriteFactory5` in-memory loader. Temporary font files use the
already locked `tempfile` dependency, remain alive for lazy DirectWrite reads,
and are removed when the text system is dropped. Tests cover font loading,
cleanup, and invalid input.

`src/directx_devices.rs` and `src/directx_renderer.rs` request `IDXGIFactory2`
instead of `IDXGIFactory6`; only inherited factory methods were used. The
renderer also requests `IDWriteFactory4` for rendering parameters.

`src/window.rs` gates the `render_to_image` trait implementation on
`test-support`, matching the GPUI dependency's trait when compiling unit tests.

All other source files are unchanged. Keep these changes until upstream
supports the interfaces available on Windows Server 2016 (build 14393).
