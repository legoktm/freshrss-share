// Helper functions for base64 and UTF-8 encoding/decoding
function decodeBase64(base64) {
    const binary = atob(base64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
        bytes[i] = binary.charCodeAt(i);
    }
    return bytes;
}

function encodeBase64(bytes) {
    let binary = "";
    for (let i = 0; i < bytes.length; i++) {
        binary += String.fromCharCode(bytes[i]);
    }
    return btoa(binary);
}

function encodeUTF8(str) {
    const encoder = new TextEncoder();
    return encoder.encode(str);
}

function encryptAndSign(message, secretKeyBase64) {
    // Decode secret key (32 bytes)
    const secretKey = decodeBase64(secretKeyBase64);

    // Generate random nonce (24 bytes for secretbox)
    const nonce = nacl.randomBytes(24);

    // Encrypt with secretbox (XSalsa20-Poly1305)
    const messageBytes = encodeUTF8(message);
    const encrypted = nacl.secretbox(messageBytes, nonce, secretKey);

    // Combine: nonce + ciphertext
    const payload = new Uint8Array(nonce.length + encrypted.length);
    payload.set(nonce, 0);
    payload.set(encrypted, nonce.length);

    // Return as base64
    return encodeBase64(payload);
}

// Form handling for submit page
document.addEventListener("DOMContentLoaded", function () {
    const form = document.getElementById("submitForm");
    const resultDiv = document.getElementById("result");
    const secretKeyInput = document.getElementById("secretKey");
    const articleIdInput = document.getElementById("articleId");

    // Load article from URL parameter
    const fromHash = window.location.hash.slice(1);
    if (fromHash) {
        articleIdInput.value = fromHash;
    }

    // Load secret key from localStorage
    const savedSecretKey = localStorage.getItem("freshrss-share-secret");
    if (savedSecretKey) {
        secretKeyInput.value = savedSecretKey;
    }

    form.addEventListener("submit", function (e) {
        e.preventDefault();

        const articleId = articleIdInput.value;
        const secretKey = secretKeyInput.value;

        // Save secret key to localStorage
        localStorage.setItem("freshrss-share-secret", secretKey);

        try {
            // Create payload with current timestamp
            const currentTime = Math.floor(Date.now() / 1000);
            const payload = JSON.stringify({
                id: parseInt(articleId, 10),
                t: currentTime,
            });

            // Encrypt the payload
            const encrypted = encryptAndSign(payload, secretKey);

            // Generate the share link
            const baseUrl = window.location.origin;
            const shareUrl = `${baseUrl}/share/?p=${encodeURIComponent(encrypted)}`;

            // Display success with shareable link
            resultDiv.className = "result success";
            resultDiv.style.display = "block";
            resultDiv.innerHTML = `
                <strong>Success!</strong> Share link generated.
                <div class="encrypted-output">
                    <strong>Share Link:</strong><br>
                    <a href="${shareUrl}" target="_blank">${shareUrl}</a><br><br>
                    <button id="copyButton">Copy Link</button>
                </div>
            `;

            // Add event listener for copy button
            const copyButton = document.getElementById("copyButton");
            copyButton.addEventListener("click", function () {
                navigator.clipboard
                    .writeText(shareUrl)
                    .then(() => {
                        copyButton.textContent = "Copied!";
                        setTimeout(() => {
                            copyButton.textContent = "Copy Link";
                        }, 2000);
                    })
                    .catch((err) => {
                        console.error("Failed to copy:", err);
                        copyButton.textContent = "Failed to copy";
                        setTimeout(() => {
                            copyButton.textContent = "Copy Link";
                        }, 2000);
                    });
            });
        } catch (error) {
            // Display error
            resultDiv.className = "result error";
            resultDiv.style.display = "block";
            resultDiv.innerHTML = `<strong>Error:</strong> ${error.message}`;
        }
    });
});
