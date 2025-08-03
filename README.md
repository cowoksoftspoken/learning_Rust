# 🎯 Studi Kasus Rust: YouTube Downloader CLI

![Tampilan Aplikasi](https://i.ibb.co/d0dwvMTh/Screenshot-20250803-170903-Chrome.jpg)

Ini adalah salah satu studi kasus project yang saya kerjakan selama belajar bahasa pemrograman **Rust**. Tujuan dari project ini adalah membangun aplikasi CLI sederhana untuk download video dari YouTube menggunakan kombinasi dari beberapa tools.

## 🛠️ Tools & Dependency

Untuk menjalankan project ini, pastikan kamu sudah install:

- [`cargo`](https://doc.rust-lang.org/cargo/) – package manager-nya Rust
- `rustc` – Rust compiler
- [`yt-dlp`](https://github.com/yt-dlp/yt-dlp) – downloader video berbasis YouTube-dl
- [`ffmpeg`](https://ffmpeg.org/) – buat proses konversi audio/video (dipakai di belakang layar oleh yt-dlp)

---

## 🚀 Cara Clone & Jalankan

```bash
git clone https://github.com/cowoksoftspoken/Learning_Rust.git
cd Learning_Rust
cargo run
