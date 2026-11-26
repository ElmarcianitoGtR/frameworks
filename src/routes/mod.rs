use axum::http::StatusCode;
use axum::{Json, Router};
use sqlx::PgPool;

use crate::models::ApiError;

pub mod clientes;
pub mod colecciones;
pub mod envios;
pub mod prendas;

/// Alias de resultado estándar para los handlers HTTP de la API.
pub type ApiResult<T> = Result<T, (StatusCode, Json<ApiError>)>;

/// Construye una respuesta de error HTTP con código de estado y cuerpo JSON `ApiError`.
pub fn api_error(status: StatusCode, mensaje: impl Into<String>) -> (StatusCode, Json<ApiError>) {
    (status, Json(ApiError::new(mensaje)))
}

/// Verifica si un error de SQLx corresponde a un código específico de PostgreSQL (SQLSTATE).
pub fn es_codigo_pg(err: &sqlx::Error, codigo: &str) -> bool {
    matches!(err, sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some(codigo))
}

/// Convierte un error de SQLx/PostgreSQL en una respuesta HTTP adecuada.
pub fn mapear_error_db(err: sqlx::Error) -> (StatusCode, Json<ApiError>) {
    if es_codigo_pg(&err, "23503") {
        return api_error(
            StatusCode::CONFLICT,
            "No se puede completar la operación por una restricción de integridad referencial",
        );
    }

    if es_codigo_pg(&err, "23505") {
        return api_error(
            StatusCode::CONFLICT,
            "Ya existe un registro con ese valor único",
        );
    }

    eprintln!("Error en base de datos: {err}");
    api_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Error interno en la base de datos",
    )
}

/// Construye el router con los endpoints del negocio:
/// colecciones, prendas, clientes y envíos.
pub fn crear_rutas(pool: PgPool) -> Router {
    Router::new()
        .nest("/api/colecciones", colecciones::rutas(pool.clone()))
        .nest("/api/prendas", prendas::rutas(pool.clone()))
        .nest("/api/clientes", clientes::rutas(pool.clone()))
        .nest("/api/envios", envios::rutas(pool))
}
