# 🎯 Studi Kasus Rust: YouTube Downloader

![Tampilan Aplikasi](https://camo.githubusercontent.com/2f06cb07c4eb1f22fa68bce73f2a53708000d276e755ac471595b8e555320165/68747470733a2f2f692e6962622e636f2f363762324e6467732f53637265656e73686f742d32303235303830342d3132343834302d4368726f6d652e6a7067)

Ini adalah salah satu studi kasus project yang saya kerjakan selama belajar bahasa pemrograman **Rust**. Tujuan dari project ini adalah membangun aplikasi Website Downloader sederhana untuk download video dari YouTube menggunakan kombinasi dari beberapa tools.

## 🛠️ Tools & Dependency

Untuk menjalankan project ini, pastikan kamu sudah install:

- [`cargo`](https://doc.rust-lang.org/cargo/) – package manager-nya Rust
- `rustc` – Rust compiler
- [`yt-dlp`](https://github.com/yt-dlp/yt-dlp) – downloader video berbasis YouTube-dl
- [`ffmpeg`](https://ffmpeg.org/) – buat proses konversi audio/video (dipakai di belakang layar oleh yt-dlp)

---

## 🚀 Cara Clone & Jalankan

```bash
git clone https://github.com/cowoksoftspoken/RusTube.git
cd RusTube
cargo run
```
---

## ✏ Cara Edit & Kustomisasi
```Rust
    let app_router = Router::new()
        .route("/", get(display_form))
        .route("/download", post(handle_unduhan))
        .route("/progress", get(handle_progress))
        .route(
            "/ambil_download/:id_unduhan_str/:nama_file",
            get(ambil_download),
        )
        .with_state(status_aplikasi.clone())
         // Jika kamu ingin mengubah apapun di folder static maka kamu juga perlu mengubah bagian ini
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/downloads", ServeDir::new("downloads"));
```
Semua hal yang ditampilkan kedalam browser harus berada dalam folder static kecuali sudah dikustom & menjalankan project ini berarti kamu sudah paham **Rust** Sedikit

