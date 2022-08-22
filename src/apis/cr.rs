#[derive(serde::Deserialize, serde::Serialize, std::fmt::Debug)]
pub struct CreateCRRequest {
    pub title: Option<String>,
    pub description: Option<String>,
}

pub async fn create_cr(
    req: actix_web::web::Json<CreateCRRequest>,
) -> actix_web::Result<actix_web::HttpResponse> {
    match create_cr_worker(req.0).await {
        Ok(cr_number) => {
            #[derive(serde::Serialize)]
            struct CreateCRResponse {
                url: String,
            }
            let url = format!("-/{}/-/about/", cr_number);
            fpm::apis::success(CreateCRResponse { url })
        }
        Err(err) => fpm::apis::error(
            err.to_string(),
            actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        ),
    }
}

async fn create_cr_worker(cr_request: CreateCRRequest) -> fpm::Result<usize> {
    let config = fpm::Config::read(None, false).await?;
    let cr_number = config.extract_cr_number().await?;
    let default_title = format!("CR#{cr_number}");
    let cr_about = fpm::cr::CRAbout {
        title: cr_request.title.unwrap_or_else(|| default_title),
        description: cr_request.description,
        cr_number: cr_number as usize,
        open: true,
    };
    fpm::commands::create_cr::add_cr_to_workspace(&config, &cr_about).await?;
    Ok(cr_number as usize)
}

pub async fn create_cr_page() -> actix_web::Result<actix_web::HttpResponse> {
    match create_cr_page_worker().await {
        Ok(body) => Ok(actix_web::HttpResponse::Ok().body(body)),
        Err(err) => fpm::apis::error(
            err.to_string(),
            actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        ),
    }
}

async fn create_cr_page_worker() -> fpm::Result<Vec<u8>> {
    let mut config = fpm::Config::read(None, false).await?;
    let create_cr_ftd = if config.root.join("ccp.ftd").exists() {
        tokio::fs::read_to_string(config.root.join("ccp.ftd")).await?
    } else {
        fpm::create_cr_ftd().to_string()
    };

    let main_document = fpm::Document {
        id: "create-cr.ftd".to_string(),
        content: create_cr_ftd,
        parent_path: config.root.as_str().to_string(),
        package_name: config.package.name.clone(),
    };

    fpm::package_doc::read_ftd(&mut config, &main_document, "/", false).await
}
