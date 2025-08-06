# 🎯 Studi Kasus Rust: YouTube Downloader

![Tampilan Aplikasi](https://i.ibb.co/67b2Ndgs/Screenshot-20250804-124840-Chrome.jpg)

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
git clone https://github.com/cowoksoftspoken/learning_Rust.git
cd learning_Rust
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
         // Jika kamu ingin mengubah apapund di folder static maka kamu juga perlu mengubah bagian ini
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/downloads", ServeDir::new("downloads"));
```
