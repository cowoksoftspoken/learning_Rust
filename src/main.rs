use axum::{
    Router,
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use mime_guess;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::{
    fs::{self, File},
    io::{AsyncBufReadExt, BufReader},
    net::TcpListener,
    process::Command,
    sync::broadcast,
};
use tower_http::services::ServeDir;
use uuid::Uuid;

#[derive(Clone)]
struct StatusAplikasi {
    saluran_progres_unduhan: Arc<Mutex<HashMap<Uuid, broadcast::Sender<String>>>>,
}

#[tokio::main]
async fn main() {
    let _ = fs::remove_dir_all("downloads").await;
    fs::create_dir_all("downloads").await.unwrap();

    let status_aplikasi = StatusAplikasi {
        saluran_progres_unduhan: Arc::new(Mutex::new(HashMap::new())),
    };

    let app_router = Router::new()
        .route("/", get(display_form))
        .route("/download", post(handle_unduhan))
        .route("/progress", get(handle_progress))
        .route(
            "/ambil_unduhan/:id_unduhan_str/:nama_file",
            get(ambil_unduhan),
        )
        .with_state(status_aplikasi.clone())
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/downloads", ServeDir::new("downloads"));

    let address = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server berjalan di http://{}", address);

    let listener = TcpListener::bind(address).await.unwrap();
    axum::serve(listener, app_router.into_make_service())
        .await
        .unwrap();
}

async fn display_form() -> impl IntoResponse {
    Html(
        fs::read_to_string("static/index.html")
            .await
            .unwrap_or_else(|_| "Gagal memuat index.html".into()),
    )
}

#[derive(Deserialize)]
struct ParameterProgres {
    id: Uuid,
}

#[derive(Serialize)]
struct ResponseDownloadStart {
    id: String,
    status: String,
}

async fn handle_unduhan(
    State(status_aplikasi): State<StatusAplikasi>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut url_unduhan: Option<String> = None;
    let mut format = "auto".to_string(); // Default: otomatis

    while let Some(field) = multipart.next_field().await.unwrap() {
        match field.name() {
            Some("url") => url_unduhan = Some(field.text().await.unwrap()),
            Some("format") => format = field.text().await.unwrap(),
            _ => {}
        }
    }

    let url_unduhan = match url_unduhan {
        Some(url) => url,
        None => {
            return (StatusCode::BAD_REQUEST, "URL tidak ditemukan.").into_response();
        }
    };

    let id_unduhan = Uuid::new_v4();
    let jalur_keluar_template = format!("downloads/{}.%(ext)s", id_unduhan);

    let (pengirim, _) = broadcast::channel::<String>(16);
    {
        let mut map = status_aplikasi.saluran_progres_unduhan.lock().unwrap();
        map.insert(id_unduhan, pengirim.clone());
    }

    let id_clone = id_unduhan;
    let saluran_clone = status_aplikasi.saluran_progres_unduhan.clone();
    tokio::spawn(async move {
        let mut cmd = Command::new("yt-dlp");

        cmd.arg("-o")
            .arg(&jalur_keluar_template)
            .arg(&url_unduhan)
            .arg("--progress-template")
            .arg("download:%(progress._percent_str)s")
            .args(["--user-agent", "Mozilla/5.0"])
            .arg("--newline");

        match format.as_str() {
            "mp3" => {
                cmd.args(["-x", "--audio-format", "mp3"]);
                println!("[yt-dlp] Format MP3 dipilih");
            }
            "mp4" => {
                cmd.arg("-f")
                    .arg("bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best");
                println!("[yt-dlp] Format MP4 dipilih");
            }
            _ => {
                println!("[yt-dlp] Format AUTO dipilih");
            }
        }

        let mut child = match cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = pengirim.send(format!("ERROR:Gagal menjalankan yt-dlp: {}", e));
                return;
            }
        };

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let mut reader_out = BufReader::new(stdout).lines();
        let mut reader_err = BufReader::new(stderr).lines();

        loop {
            tokio::select! {
                out = reader_out.next_line() => {
                    match out {
                        Ok(Some(line)) => {
                            println!("[stdout] {}", line);
                            if line.contains("download:") || line.contains('%') {
                                let _ = pengirim.send(line.clone());
                            }

                            if line.contains("Merging formats into"){
                                let _ = pengirim.send("INFO: Proses penggabungan format dimulai.".to_string());
                                println!("[Rust] Detected Merge Finish Line");
                            }
                        },
                        Ok(None) => break,
                        Err(e) => {
                            let _ = pengirim.send(format!("ERROR:Stdout error: {}", e));
                            break;
                        }
                    }
                }
                err = reader_err.next_line() => {
                    match err {
                        Ok(Some(line)) => {
                            println!("[stderr] {}", line);
                            if line.to_lowercase().contains("error") {
                                let _ = pengirim.send(format!("ERROR:{}", line));
                            }
                        },
                        Ok(None) => break,
                        Err(e) => {
                            let _ = pengirim.send(format!("ERROR:Stderr error: {}", e));
                            break;
                        }
                    }
                }
            }
        }

        let status = child.wait().await;
        println!("[yt-dlp] Exit status: {:?}", status);

        let mut nama_file: Option<String> = None;
        if let Ok(status) = status {
            if status.success() {
                let entries = fs::read_dir("downloads").await.unwrap();
                tokio::pin!(entries);

                while let Some(entry) = entries.next_entry().await.unwrap() {
                    let fname = entry.file_name().to_string_lossy().to_string();
                    if fname.starts_with(&id_clone.to_string())
                        && (fname.ends_with(".mp4")
                            || fname.ends_with(".mp3")
                            || fname.ends_with(".webm"))
                    {
                        nama_file = Some(fname);
                        break;
                    }
                }
            }
        }

        if let Some(fname) = nama_file {
            match pengirim.send(format!("COMPLETE:{}", fname)) {
                Ok(_) => println!("[Rust] Event COMPLETE terkirim: {}", fname),
                Err(e) => println!("[Rust] ⚠️ Gagal kirim COMPLETE: {}", e),
            }
            println!("[Rust] Unduhan selesai: {}", fname);
        } else {
            match pengirim.send("ERROR:Gagal mengunduh atau file tidak ditemukan.".to_string()) {
                Ok(_) => println!("[Rust] Event ERROR terkirim."),
                Err(e) => println!("[Rust] Gagal kirim ERROR: {}", e),
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        let mut map = saluran_clone.lock().unwrap();
        map.remove(&id_clone);
        println!("[Rust] Channel progress dibersihkan: {}", id_clone);
    });

    let respons_data = ResponseDownloadStart {
        id: id_unduhan.to_string(),
        status: "Download Dimulai.".into(),
    };

    (StatusCode::OK, axum::Json(respons_data)).into_response()
}

async fn ambil_unduhan(Path((_id_unduhan, nama_file)): Path<(String, String)>) -> Response {
    let jalur_file = PathBuf::from(format!("downloads/{}", nama_file));

    if !jalur_file.exists() {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("File tidak ditemukan."))
            .unwrap();
    }

    if let Ok(file) = File::open(&jalur_file).await {
        use tokio_util::io::ReaderStream;
        let progress_reader = ReaderStream::new(file);

        let salinan_jalur_file = jalur_file.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            let _ = fs::remove_file(salinan_jalur_file).await;
        });

        let content_type = mime_guess::from_path(&nama_file)
            .first_or_octet_stream()
            .to_string();

        return Response::builder()
            .header(header::CONTENT_TYPE, content_type)
            .header(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", nama_file),
            )
            .body(Body::from_stream(progress_reader))
            .unwrap();
    }

    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from("Gagal Mengambil file."))
        .unwrap()
}

async fn handle_progress(
    State(status_aplikasi): State<StatusAplikasi>,
    Query(parameter_kueri): Query<ParameterProgres>,
) -> impl IntoResponse {
    let penerima_saluran = {
        let saluran = status_aplikasi.saluran_progres_unduhan.lock().unwrap();
        saluran
            .get(&parameter_kueri.id)
            .map(|pengirim| pengirim.subscribe())
    };

    let Some(mut penerima_saluran) = penerima_saluran else {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("ID unduhan tidak ditemukan atau sudah selesai."))
            .unwrap();
    };

    let progress_berjalan = async_stream::stream! {
        loop {
            match penerima_saluran.recv().await {
                Ok(pesan_progres) => {
                    if pesan_progres.starts_with("COMPLETE:") {
                        let nama_file = pesan_progres.trim_start_matches("COMPLETE:").to_string();
                        yield Ok::<_, std::convert::Infallible>(format!("event: complete\ndata: {}\n\n", nama_file));
                        break; // Akhir
                    } else if pesan_progres.starts_with("ERROR:") {
                        let pesan_error = pesan_progres.trim_start_matches("ERROR:").to_string();
                        yield Ok::<_, std::convert::Infallible>(format!("event: error\ndata: {}\n\n", pesan_error));
                        break; // Akhiri jika ada error
                    } else {
                        // Kirim pesan progres
                        yield Ok::<_, std::convert::Infallible>(format!("data: {}\n\n", pesan_progres));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    eprintln!("Aliran progres tertinggal untuk ID: {}", parameter_kueri.id);
                    yield Ok::<_, std::convert::Infallible>(format!("data: tertinggal\n\n"));
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };

    Response::builder()
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .body(Body::from_stream(progress_berjalan))
        .unwrap()
}
