const form = document.getElementById("download-form");
const progressText = document.getElementById("progress-text");
const progressFill = document.getElementById("progress-fill");
const downloadLinkContainer = document.getElementById(
  "download-link-container"
);
const loginForm = document.getElementById("login-form");
const downloadLink = document.getElementById("download-link");
const videoPreview = document.getElementById("preview-video");
const previewContainer = document.getElementById("preview-container");
const progressContainer = document.getElementById("progress-container");
const cancelButton = document.getElementById("cancel-button");
const pratinjauMedia = document.querySelector("#preview-container h2");

let currentSource = null;
let retries = 0;
const maxRetries = 5;
let finished = false;
let currentDownloadId = null;
let countDownTimer = null;
let token = localStorage.getItem("jwt_access") || "";
document.querySelector("script").nonce =
  document.querySelector("script").getAttribute("data-nonce") ||
  "Tidak ada nonce";

function formatDuration(duration) {
  const minutes = Math.floor(duration / 60);
  const seconds = duration % 60;
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

if (token) {
  loginForm.classList.add("hidden");
  form.classList.remove("hidden");
}

async function refreshAccessToken() {
  const refreshToken = localStorage.getItem("jwt_refresh");
  if (!refreshToken) {
    alert("Session habis, silakan login ulang");
    localStorage.clear();
    location.reload();
    return null;
  }

  const res = await fetch("/auth/refresh", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ refresh_token: refreshToken }),
  });

  if (!res.ok) {
    alert("Gagal refresh token, silakan login ulang");
    localStorage.clear();
    location.reload();
    return null;
  }

  const data = await res.json();
  localStorage.setItem("jwt_access", data.access_token);
  console.log("[JS] Token akses diperbarui:", data.access_token);
  return data.access_token;
}

function detectMimeType(filename) {
  const ext = filename.split(".").pop().toLowerCase();
  if (ext === "mp4") return "video/mp4";
  if (ext === "mp3") return "audio/mpeg";
  return "application/octet-stream";
}

async function authFetch(url, options = {}) {
  let accessToken = localStorage.getItem("jwt_access");
  if (!options.headers) options.headers = {};
  options.headers.Authorization = `Bearer ${accessToken}`;

  let res = await fetch(url, options);

  if (res.status === 401) {
    accessToken = await refreshAccessToken();
    if (!accessToken) return res;
    options.headers.Authorization = `Bearer ${accessToken}`;
    res = await fetch(url, options);
  }

  return res;
}

function resetUI() {
  progressText.textContent = "⌛ Menunggu...";
  progressFill.style.width = "0%";
  progressFill.setAttribute("data-percent", "0%");
  downloadLinkContainer.classList.add("hidden");
  previewContainer.classList.add("hidden");
  cancelButton.classList.add("hidden");
  if (videoPreview) {
    videoPreview.src = "";
  }
  finished = false;
  retries = 0;
  currentDownloadId = null;
  if (currentSource) {
    currentSource.close();
    currentSource = null;
  }

  Array.from(previewContainer.children).forEach((child) => {
    if (child.tagName !== "H2") {
      child.remove();
    }
  });

  if (countDownTimer) {
    clearInterval(countDownTimer);
    countDownTimer = null;
  }
}

loginForm.addEventListener("submit", async (e) => {
  e.preventDefault();
  const username = document.getElementById("username").value;

  try {
    const res = await fetch("/auth/login", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ username }),
    });

    if (!res.ok) throw new Error("Login gagal!");

    const data = await res.json();
    localStorage.setItem("jwt_access", data.access_token);
    localStorage.setItem("jwt_refresh", data.refresh_token);

    alert("Login sukses! Token JWT disimpan.");
    loginForm.classList.add("hidden");
    form.classList.remove("hidden");
  } catch (err) {
    alert("Gagal login: " + err.message);
  }
});

form.addEventListener("submit", async (e) => {
  e.preventDefault();
  resetUI();

  // cek jika ada unduhan yang sedang berjalan
  if (currentSource && !finished) {
    alert("Masih ada unduhan yang sedang berjalan. Silakan tunggu.");
    return;
  }

  // revoke blob URL jika ada
  if (videoPreview && videoPreview.src) {
    URL.revokeObjectURL(videoPreview.src);
    videoPreview.src = "";
  }
  cancelButton.classList.remove("hidden");
  previewContainer.classList.add("hidden");
  progressContainer.classList.remove("hidden");
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
    const response = await authFetch("/download", {
      method: "POST",
      body: formData,
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      const errorMsg = errorData.status || "Terjadi kesalahan tidak diketahui.";
      progressText.textContent = `Gagal mulai: ${errorMsg}`;
      return;
    }

    const data = await response.json();
    const downloadId = data.id;
    retries = 0;
    currentDownloadId = downloadId;
    console.log("[JS] ID Unduhan:", downloadId);
    progressText.textContent = `🔗 ID Unduhan: ${downloadId}`;
    progressFill.setAttribute("data-thread-id", downloadId);
    connectEventSource(downloadId);
  } catch (error) {
    console.error("[JS] Jaringan error:", error);
    progressText.textContent = `Jaringan error: ${error.message}`;
  }
});

cancelButton.addEventListener("click", async () => {
  if (!currentDownloadId) {
    alert("Tidak ada unduhan yang sedang berjalan.");
    return;
  }

  if (localStorage.getItem(`${currentDownloadId}`) === "completed") {
    alert("Unduhan sudah selesai, tidak bisa dibatalkan.");
    return;
  }

  try {
    await cancel_download(currentDownloadId);
    resetUI();
    if (currentSource) {
      currentSource.close();
      currentSource = null;
    }
    progressText.textContent = "Pembatalan unduhan berhasil.";
    progressFill.style.width = "0%";
    progressFill.setAttribute("data-percent", "0%");
    downloadLinkContainer.classList.add("hidden");
    previewContainer.classList.add("hidden");
  } catch (error) {
    console.error("[JS] Gagal membatalkan unduhan:", error);
    alert(`Gagal membatalkan unduhan: ${error.message}`);
  }
});

function connectEventSource(downloadId) {
  if (currentSource) {
    currentSource.close();
  }

  currentSource = new EventSource(`/progress?id=${downloadId}`);
  console.log(`[JS] Opening EventSource untuk ID: ${downloadId}`);

  currentSource.onmessage = (event) => {
    const msg = event.data.trim();
    console.log("[JS] onmessage:", msg);

    if (msg.startsWith("ERROR:")) {
      finished = true;
      currentSource.close();
      progressText.textContent = `${msg.replace("ERROR:", "")}`;
      cancelButton.classList.add("hidden");
      progressFill.style.width = "0%";
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

  function generateElement(type) {
    if (type.startsWith("video/")) {
      const vid = document.createElement("video");
      vid.id = "preview-video";
      vid.className = "w-full rounded-xl border border-white/20 shadow-lg";
      vid.controls = true;
      return vid;
    } else if (type.startsWith("audio/")) {
      const aud = document.createElement("audio");
      aud.id = "preview-video";
      aud.className = "w-full rounded-xl border border-white/20 shadow-lg";
      aud.controls = true;
      return aud;
    }

    const div = document.createElement("div");
    div.className =
      "w-full h-64 bg-gray-800 rounded-xl flex items-center justify-center text-gray-400";
    div.textContent = "Pratinjau tidak tersedia";
    return div;
  }

  currentSource.addEventListener("complete", (event) => {
    finished = true;
    currentSource.close();

    const filename = event.data.trim();
    let mimeType = detectMimeType(filename);

    localStorage.setItem(`${currentDownloadId}`, "completed");

    Array.from(previewContainer.children).forEach((child) => {
      if (child.tagName !== "H2") {
        child.remove();
      }
    });

    pratinjauMedia.textContent = `Pratinjau ${
      mimeType.startsWith("video")
        ? "Video"
        : mimeType.startsWith("audio")
        ? "Audio"
        : "Media"
    }:`;
    let dinamicEl = generateElement(mimeType);
    previewContainer.appendChild(dinamicEl);
    progressText.textContent = `Selesai: ${filename}`;
    progressFill.style.width = "100%";
    progressFill.setAttribute("data-percent", "100%");
    setDownloadLink(downloadId, filename);
    previewContainer.classList.remove("hidden");
    downloadLink.textContent = `Unduh ${filename} (600s)`;
    downloadLinkContainer.classList.remove("hidden");
    cancelButton.classList.add("hidden");
    let remaining = 600;
    countDownTimer = setInterval(() => {
      remaining--;
      downloadLink.textContent = `Unduh ${filename} (${formatDuration(
        remaining
      )})`;
      if (remaining <= 0) {
        clearInterval(countDownTimer);
        downloadLink.href = "#";
        downloadLink.textContent = `⛔ Link Expired`;
        downloadLink.classList.add("opacity-50", "cursor-not-allowed");
        downloadLink.classList.remove("hover:bg-green-700");
        progressText.textContent =
          "Link download sudah kedaluwarsa. Silakan ulangi.";
        console.warn("[Warn] Link expired, tidak bisa diunduh lagi.");
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

async function cancel_download(downloadId) {
  const response = await authFetch(`/cancel_download?id=${downloadId}`, {
    method: "POST",
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({}));
    const errorMsg = errorData.status || "Terjadi kesalahan tidak diketahui.";
    alert(`Gagal membatalkan unduhan: ${errorMsg}`);
    return;
  }

  console.log(
    "[JS] Pembatalan unduhan berhasil." + (downloadId, response.text())
  );
  progressText.textContent = "Pembatalan unduhan berhasil.";
}

async function setDownloadLink(downloadId, filename) {
  const fileUrl = `/ambil_download/${downloadId}/${filename}`;
  const res = await authFetch(fileUrl);

  if (!res.ok) {
    throw new Error("Gagal ambil file");
  }

  const blob = await res.blob();
  const blobUrl = URL.createObjectURL(blob);

  const videoPreview = document.getElementById("preview-video");
  if (videoPreview) {
    videoPreview.src = blobUrl;
  }

  downloadLink.href = blobUrl;
  downloadLink.download = `RustTube_${filename}`;
}
downloadLink.addEventListener("click", (e) => {
  if (
    downloadLink.href === "#" ||
    downloadLink.classList.contains("opacity-50")
  ) {
    e.preventDefault();
    alert("Link unduhan tidak valid atau sudah kedaluwarsa.");
  }
});
