// Auto-resize the article iframe to fit its content without scrolling.
//
// The iframe is sandboxed into an opaque origin, so its messages arrive with an
// origin of "null" and there is nothing useful to check there. Identify it by
// source instead. This script runs before the iframe exists, so look it up
// lazily -- by the time a message arrives it has been parsed.

// Guard against a runaway document pinning the page open forever.
const MAX_HEIGHT = 200000;

window.addEventListener("message", (e) => {
    const iframe = document.getElementById("content-frame");
    if (!iframe || e.source !== iframe.contentWindow) {
        return;
    }
    if (!e.data || e.data.type !== "resize" || !Number.isFinite(e.data.height)) {
        return;
    }

    iframe.style.height = Math.min(Math.max(e.data.height, 0), MAX_HEIGHT) + "px";
});
