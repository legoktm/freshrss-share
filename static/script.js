// Auto-resize iframe to fit content without scrolling
const iframe = document.getElementById("content-frame");

// Listen for height messages from iframe
window.addEventListener("message", (e) => {
    if (e.data && e.data.type === "resize") {
        iframe.style.height = e.data.height + "px";
    }
});

// Append CSS link and resize script to srcdoc
iframe.srcdoc =
    `<link rel="stylesheet" href="/share/static/iframe.css">` +
    iframe.srcdoc +
    `<script src="/share/static/iframe.js"><\/script>`;
