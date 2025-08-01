use axum::{
    body::{Body}, 
    extract::{Path, Query, State, Multipart}, 
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
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
    process::Command,
    sync::broadcast,
    net::TcpListener, 
};
use tower_http::services::ServeDir;
use uuid::Uuid;
use mime_guess; 

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
        .route("/ambil_unduhan/:id_unduhan_str/:nama_file", get(ambil_unduhan)) 
        .with_state(status_aplikasi.clone())
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/downloads", ServeDir::new("downloads")); 

    let address = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server berjalan di http://{}", address);

    let listener = TcpListener::bind(address).await.unwrap();
    axum::serve(listener, app_router.into_make_service()).await.unwrap();
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

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        if name == "url" {
            url_unduhan = Some(field.text().await.unwrap());
            break; 
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

    let (pengirim_saluran, _penerima_saluran) = broadcast::channel::<String>(16); // Buffer size 16
    {
        let mut saluran = status_aplikasi.saluran_progres_unduhan.lock().unwrap();
        saluran.insert(id_unduhan, pengirim_saluran.clone()); 
    }

    let pengirim_saluran_kloning = pengirim_saluran.clone();
    let id_unduhan_kloning = id_unduhan;
    let saluran_progres_unduhan_kloning = status_aplikasi.saluran_progres_unduhan.clone(); // Clone untuk task
    tokio::spawn(async move {
        let mut child_proccess = Command::new("yt-dlp")
            .arg("-o")
            .arg(&jalur_keluar_template)
            .arg(&url_unduhan)
            .arg("--progress-template")
            .arg("download:%(progress._percent_str)s")
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Gagal menjalankan yt-dlp");

        let stderr = child_proccess.stderr.take().unwrap();
        let mut pembaca_stderr = BufReader::new(stderr).lines();

        while let Ok(Some(baris)) = pembaca_stderr.next_line().await {
            if baris.contains("download:") {
                let _ = pengirim_saluran_kloning.send(baris); // Kirim update progres
            }
        }

        let status_proses = child_proccess.wait().await;

        let mut nama_asli: Option<String> = None;
        if let Ok(status) = status_proses {
            if status.success() {
                let entries = fs::read_dir("downloads").await.unwrap();
                tokio::pin!(entries);
                let mut dir = entries;
                while let Some(entry_result) = dir.next_entry().await.unwrap() {
                    let fname = entry_result.file_name().to_string_lossy().to_string();
                    if fname.starts_with(&id_unduhan_kloning.to_string()) {
                        nama_asli = Some(fname);
                        break;
                    }
                }
            }
        }

        // Bakal diterima di EventSource di frontend
        if let Some(fname) = nama_asli {
            let _ = pengirim_saluran_kloning.send(format!("COMPLETE:{}", fname));
        } else {
            let _ = pengirim_saluran_kloning.send("ERROR:Gagal mengunduh.".to_string());
        }

        // Hapus Saluran Progress Setelah Mengunduh dan Menghapus Lifetimenya
        {
            let mut saluran = saluran_progres_unduhan_kloning.lock().unwrap();
            saluran.remove(&id_unduhan_kloning);
        }
    });

    let respons_data = ResponseDownloadStart {
        id: id_unduhan.to_string(),
        status: "Download Dimulai.".to_string(),
    };
    (StatusCode::OK, axum::Json(respons_data)).into_response()
}

async fn ambil_unduhan(
    Path((_id_unduhan, nama_file)): Path<(String, String)>, 
) -> Response {
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
        .body(Body::from("Gagal melayani file."))
        .unwrap()
}

async fn handle_progress(
    State(status_aplikasi): State<StatusAplikasi>, 
    Query(parameter_kueri): Query<ParameterProgres>, 
) -> impl IntoResponse {
    let penerima_saluran = {
        let saluran = status_aplikasi.saluran_progres_unduhan.lock().unwrap();
        saluran.get(&parameter_kueri.id).map(|pengirim| pengirim.subscribe())
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