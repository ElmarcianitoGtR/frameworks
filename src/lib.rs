pub mod auth;
pub mod db;
pub mod models;
pub mod routes;

use axum::routing::post;
use axum::Router;
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use crate::models::{
    ActualizarEnvioDto, ApiError, Cliente, Coleccion, CrearClienteDto, CrearColeccionDto,
    CrearEnvioDto, CrearPrendaDto, Envio, LoginRequest, LoginResponse, MensajeResponse, Prenda,
};

/// Agrega el esquema de seguridad Bearer JWT a la especificación OpenAPI
/// para habilitar el botón "Authorize" en Swagger UI.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

/// Especificación OpenAPI 3 de la API Tienda de Ropa.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "API Tienda de Ropa",
        version = "0.1.0",
        description = "API REST didáctica construida con Axum, SQLx, JWT y Swagger UI."
    ),
    paths(
        auth::login,
        routes::colecciones::listar_colecciones,
        routes::colecciones::obtener_coleccion,
        routes::colecciones::crear_coleccion,
        routes::colecciones::actualizar_coleccion,
        routes::colecciones::eliminar_coleccion,
        routes::prendas::listar_prendas,
        routes::prendas::obtener_prenda,
        routes::prendas::crear_prenda,
        routes::prendas::actualizar_prenda,
        routes::prendas::eliminar_prenda,
        routes::clientes::registrar_cliente,
        routes::clientes::listar_clientes,
        routes::clientes::obtener_cliente,
        routes::clientes::actualizar_cliente,
        routes::clientes::eliminar_cliente,
        routes::envios::listar_envios,
        routes::envios::obtener_envio,
        routes::envios::crear_envio,
        routes::envios::actualizar_envio,
        routes::envios::eliminar_envio,
    ),
    components(
        schemas(
            ApiError,
            MensajeResponse,
            LoginRequest,
            LoginResponse,
            Coleccion,
            CrearColeccionDto,
            Prenda,
            CrearPrendaDto,
            Cliente,
            CrearClienteDto,
            Envio,
            CrearEnvioDto,
            ActualizarEnvioDto,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Autenticación", description = "Inicio de sesión y emisión de tokens JWT"),
        (name = "Colecciones", description = "Gestión de colecciones de temporada"),
        (name = "Prendas", description = "Catálogo e inventario de prendas"),
        (name = "Clientes", description = "Registro y consulta de clientes"),
        (name = "Envíos", description = "Creación y seguimiento de envíos")
    )
)]
pub struct ApiDoc;

/// Construye el `Router` principal combinando autenticación, rutas del negocio,
/// Swagger UI, archivos estáticos en `public/` y CORS permisivo.
pub fn crear_app(pool: PgPool) -> Router {
    Router::new()
        .route("/api/auth/login", post(auth::login))
        .merge(routes::crear_rutas(pool))
        .merge(SwaggerUi::new("/api-docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .fallback_service(ServeDir::new("public"))
        .layer(CorsLayer::permissive())
}
