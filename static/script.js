const form = document.getElementById("download-form");
const progressText = document.getElementById("progress-text");
const progressFill = document.getElementById("progress-fill");
const downloadLinkContainer = document.getElementById(
  "download-link-container"
);
const downloadLink = document.getElementById("download-link");
const videoPreview = document.getElementById("preview-video");
const previewContainer = document.getElementById("preview-container");

let currentSource = null;
let retries = 0;
const maxRetries = 5;
let finished = false;

form.addEventListener("submit", async (e) => {
  e.preventDefault();

  progressText.textContent = "🚀 Mulai download...";
  progressFill.style.width = "0%";
  progressFill.setAttribute("data-percent", "0%");
  downloadLinkContainer.classList.add("hidden");
  downloadLink.href = "#";
  finished = false;

  const url = document.getElementById("url").value;
  const format = document.getElementById("format").value;

  const formData = new FormData();
  formData.append("url", url);
  formData.append("format", format);

  if (currentSource) {
    currentSource.close();
    currentSource = null;
  }

  try {
    const response = await fetch("/download", {
      method: "POST",
      body: formData,
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      const errorMsg = errorData.status || "Terjadi kesalahan tidak diketahui.";
      progressText.textContent = `❌ Gagal mulai: ${errorMsg}`;
      return;
    }

    const data = await response.json();
    const downloadId = data.id;
    retries = 0;

    connectEventSource(downloadId);
  } catch (error) {
    console.error("[JS] Jaringan error:", error);
    progressText.textContent = `❌ Jaringan error: ${error.message}`;
  }
});

function connectEventSource(downloadId) {
  if (currentSource) {
    currentSource.close();
  }

  currentSource = new EventSource(`/progress?id=${downloadId}`);
  console.log(`[JS] Opening EventSource untuk ID: ${downloadId}`);

  // Tangkap progress biasa
  currentSource.onmessage = (event) => {
    const msg = event.data.trim();
    console.log("[JS] onmessage:", msg);

    if (msg.startsWith("ERROR:")) {
      finished = true;
      currentSource.close();
      progressText.textContent = `❌ ${msg.replace("ERROR:", "")}`;
      return;
    }

    if (msg.startsWith("INFO:")) {
      progressText.textContent = `🔧 ${msg.replace("INFO:", "")}`;
      return;
    }

    const match = msg.match(/(\d+\.\d+)%|(\d+)%/);
    if (match) {
      const percent = parseFloat(match[1] || match[2]);
      progressFill.style.width = `${percent}%`;
      progressFill.setAttribute("data-percent", `${percent.toFixed(0)}%`);
      progressText.textContent = `⏳ Downloading... ${percent.toFixed(1)}%`;
    }
  };

  currentSource.addEventListener("complete", (event) => {
    finished = true;
    currentSource.close();

    const filename = event.data.trim();
    progressText.textContent = `Selesai: ${filename}`;
    progressFill.style.width = "100%";
    progressFill.setAttribute("data-percent", "100%");
    videoPreview.src = `/ambil_unduhan/${downloadId}/${filename}`;
    previewContainer.classList.remove("hidden");

    downloadLink.href = `/ambil_unduhan/${downloadId}/${filename}`;
    downloadLink.textContent = `Unduh ${filename} (60s)`;
    downloadLinkContainer.classList.remove("hidden");

    let remaining = 60;
    const timer = setInterval(() => {
      remaining--;
      downloadLink.textContent = `Unduh ${filename} (${remaining}s)`;
      if (remaining <= 0) {
        clearInterval(timer);
        downloadLink.href = "#";
        downloadLink.textContent = `⛔ Link Expired`;
        downloadLink.classList.add("opacity-50", "cursor-not-allowed");
        downloadLink.classList.remove("hover:bg-green-700");
        progressText.textContent =
          "⚠️ Link download sudah kedaluwarsa. Silakan ulangi.";
      }
    }, 1000);
  });

  currentSource.onerror = () => {
    if (finished) {
      console.log("[JS] EventSource error tapi udah selesai. Stop reconnect.");
      return;
    }

    currentSource.close();

    if (retries < maxRetries) {
      retries++;
      progressText.textContent = `Koneksi terputus... mencoba ulang (${retries}/${maxRetries})`;
      setTimeout(() => connectEventSource(downloadId), 2000);
    } else {
      progressText.textContent = "Gagal menyambung ulang ke server.";
    }
  };
}
