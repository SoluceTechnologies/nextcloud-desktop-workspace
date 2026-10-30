# Architecture and decisions

How the app is put together, and the decisions behind it, so nobody has to dig through history to learn why something is done a certain way. The folder layout is described in the [README](README.md#development).

## Shape of the app

- **One window, many webviews.** A React *shell* webview draws the sidebar and the tab bar. Each open tab is its own child webview over the content area.
- **Separate profiles.** Each workspace (Nextcloud server) has its own browser profile, so cookies and storage never mix between servers.
- **Pure core.** `domain/` holds the whole app state and a pure state machine (`Engine`). Every user action is an `Engine` method that returns `Effect`s. It contains no Tauri code and is unit-tested.
- **One effect worker.** A single worker thread (`runtime.rs`) applies those effects to Tauri: create, show, hide and destroy webviews, open external links, clear profiles, apply the theme. Commands and webview callbacks only lock the engine for the pure transition. No lock is ever held across a main-thread round trip.
- **Bridge script.** `features/tabs/bridge.js` is injected into Nextcloud pages. It reports location, metadata, fullscreen and media support back to the app through a capability limited to the workspace's own origin.

## Decisions

### Linux child webviews use a vendored `tauri-runtime-wry`

Upstream GTK child webviews are stacked in a box and ignore their bounds ([tauri#10420](https://github.com/tauri-apps/tauri/issues/10420)). The upstream fix ([tauri#16133](https://github.com/tauri-apps/tauri/pull/16133)) was closed without being merged.

We vendor the crate and mount child webviews in a `gtk::Layout` instead. The change is kept in `src-tauri/vendor/tauri-runtime-wry.linux-child-webviews.patch`.

To upgrade:

1. Unpack the new crates.io release into the vendor folder.
2. Apply the patch with `patch -p1`.

Remove the vendored copy once upstream fixes the bug.

### Failures are visible and survivable

- **Log file:** `tauri-plugin-log` writes a rotating log; locations are in the README. It also catches the runtime's own `log::error!` messages, and GLib, GTK and WebKit warnings on Linux.
- **Panics:** release builds unwind instead of aborting. A panic hook logs the message and backtrace. The effect worker and the background threads catch a panic per effect or per iteration and keep going.
- **Symbols:** release binaries keep their symbol names (`strip = "debuginfo"`) so backtraces are readable. This costs about 4 MB.
- **Crashed web process (Linux):** the tab shows "Page stopped" with a Reload button. On macOS the runtime already reloads the page; on Windows, WebView2 shows its own error page.

### Shared-memory rendering on NVIDIA displays (Linux)

Ubuntu's and Debian's WebKitGTK carry a patch that refuses every buffer transport when the display runs on NVIDIA's driver ([webkit#262607](https://bugs.webkit.org/show_bug.cgi?id=262607)), so web views get no accelerated backing store. WebKitGTK 2.52 does not check for that: when a page enters accelerated compositing anyway, the UI process dereferences the missing backing store and the whole app crashes (SIGSEGV in `AcceleratedBackingStore::update`).

Pages reach accelerated compositing through code paths that ignore the "compositing disabled" setting. View Transitions are one. Ordinary layout is another: clicking Download in Nextcloud Files triggered it (traced to `RenderLayerCompositor::computeCompositingRequirements`). Chasing each trigger does not scale, so the app restores the backing store instead.

When the boot GPU uses the `nvidia` driver, `shared::rendering::prepare` sets `WEBKIT_FORCE_DMABUF_RENDERER=1` and `WEBKIT_DMABUF_RENDERER_FORCE_SHM=1` before GTK starts:

- Shared memory avoids the NVIDIA DMA-BUF bug the patch guards against. Forcing DMA-BUF alone fails with Wayland "Error 71 (Protocol error)".
- Pages now composite on the GPU, as WebKit intends. Measured at 1800×1000 with a full-window animation: same 62 fps as before, 40 % CPU instead of 102 %, about 70 MB more memory per web view.
- The hardware acceleration policy must stay at its default. With policy Never, a page that enters compositing anyway still attaches no content and the tab goes blank.
- Any of those variables, `WEBKIT_DISABLE_DMABUF_RENDERER` or `WEBKIT_DISABLE_COMPOSITING_MODE` set by the user wins; the app then changes nothing.

Remove this once WebKitGTK checks for the missing backing store.

### View Transitions are off on Linux

WebKitGTK 2.52 crashes the whole app (SIGSEGV in `AcceleratedBackingStore::update`) when a page starts a View Transition in a web view without an accelerated backing store. That was the case for Ubuntu's WebKitGTK on NVIDIA until the shared-memory renderer above, and still is wherever WebKit has no backing store (for example with `WEBKIT_DISABLE_DMABUF_RENDERER=1`, or without OpenGL) ([webkit#321683](https://bugs.webkit.org/show_bug.cgi?id=321683)).

Content webviews therefore turn off the `ViewTransitions` and `CrossDocumentViewTransitions` WebKit features before their first page loads. Only the animation is lost. Remove this once WebKitGTK checks for the missing backing store.

### Linux tabs start blank, then navigate

WebKit decides which features a document gets (WebRTC, the flags above) from the settings in place when the document is created. Linux tabs therefore open `about:blank`, get configured, and only then navigate to their page.

### Downloads

- **One handler per profile on Linux.** wry's default webview settings register a download handler on the profile's WebKit context for *every* webview. The oldest one wins: it never creates a missing Downloads folder, and once its tab is closed it cancels every later download in that workspace. On Linux the app removes those handlers and keeps exactly one of its own per context.
- **Incomplete transfers.** A transfer shorter than its `Content-Length` counts as failed, and the partial file is removed.
- **Registry and UI.** A pure registry (`features/downloads/registry.rs`) keeps the recent downloads and throttles progress updates to every 250 ms or 1 %. The tab bar shows a progress ring and opens a downloads panel.
- **Platform limits.** Linux has real progress and Cancel. On Windows and macOS, Tauri only reports when a download starts and finishes, so those downloads show as in progress without a percentage. Native progress for them (`ICoreWebView2DownloadOperation`, `WKDownload.progress`) is a follow-up.

### Camera, microphone and screen capture

These are granted without a prompt only to pages of the tab's own workspace, and refused to any other origin, through Tauri's permission hook (`features/media`). Before this, Linux allowed every origin, and wry on macOS granted every request.

### Talk calls on Linux open in the browser

Distribution WebKitGTK packages (Ubuntu, Arch, …) are built without WebRTC, so `RTCPeerConnection` does not exist and Talk refuses to start calls.

When a Talk page reports this, the app explains it and offers **Open in browser** for the call. Chat keeps working in the app.

We do not ship our own WebKitGTK build: that would be a large build and security-maintenance burden. Calls will work in the app with no further change once distributions ship WebKitGTK with WebRTC (planned upstream for 2.56). The Tauri CEF runtime (Chromium) is worth re-evaluating once it leaves Tauri 3 alpha.

### Devtools only in debug builds

All webviews are created through `webviews::builder`, which sets `devtools(cfg!(debug_assertions))`. CI fails if the `devtools` Cargo feature is enabled or if any code builds a webview without that constructor.
