use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse, Result};
use futures_util::TryStreamExt;
use std::io::Write;
use std::path::PathBuf;
use tracing::{error, info};

pub async fn upload_handler(
    req: HttpRequest,
    mut payload: Multipart,
    config: web::Data<crate::Spis>,
) -> Result<HttpResponse> {
    // Проверка включена ли функция загрузки
    if !config.api_upload_enable {
        return Ok(HttpResponse::NotFound().body("Upload not enabled"));
    }
    
    // Проверка токена
    if let Some(expected_token) = &config.api_upload_token {
        let token = req.headers()
            .get("X-UPLOAD-TOKEN")
            .and_then(|v| v.to_str().ok());
            
        if token != Some(expected_token.as_str()) {
            error!("Invalid upload token");
            return Ok(HttpResponse::Unauthorized().body("Invalid token"));
        }
    }
    
    // Директория для загрузки
    let upload_dir = config.api_upload_folder
        .as_ref()
        .unwrap_or(&config.media_dir);
    
    // Создаем директорию если не существует
    std::fs::create_dir_all(upload_dir)?;
    
    let mut uploaded_count = 0;
    
    // Обработка multipart данных
    while let Some(mut field) = payload.try_next().await? {
        let filename = field.content_disposition()
            .get_filename()
            .ok_or_else(|| actix_web::error::ErrorBadRequest("No filename"))?;
        
        // Безопасное имя файла (убираем путь)
        let safe_filename = PathBuf::from(filename)
            .file_name()
            .ok_or_else(|| actix_web::error::ErrorBadRequest("Invalid filename"))?
            .to_owned();
        
        let filepath = upload_dir.join(&safe_filename);
        
        // Пишем файл
        let mut f = web::block(move || std::fs::File::create(filepath))
            .await??;
            
        while let Some(chunk) = field.try_next().await? {
            f = web::block(move || f.write_all(&chunk).map(|_| f))
                .await??;
        }
        
        info!("Uploaded file: {:?}", safe_filename);
        uploaded_count += 1;
    }
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": format!("Uploaded {} file(s)", uploaded_count)
    })))
}