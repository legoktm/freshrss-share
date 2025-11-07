// iframe-resize.js
// This script runs inside the sandboxed iframe to communicate its height to the parent

function sendHeight() {
    const height = document.documentElement.scrollHeight;
    window.parent.postMessage({ type: 'resize', height: height }, '*');
}

window.addEventListener('load', sendHeight);

const observer = new MutationObserver(sendHeight);
observer.observe(document.body, {
    childList: true,
    subtree: true,
    attributes: true
});

sendHeight();
