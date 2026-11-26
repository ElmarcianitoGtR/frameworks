use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use sqlx::PgPool;

use crate::auth::SoloVendedor;
use crate::models::{ApiError, Coleccion, CrearColeccionDto, MensajeResponse};
use crate::routes::{api_error, es_codigo_pg, mapear_error_db, ApiResult};

/// Registra las rutas del recurso `/api/colecciones`.
pub fn rutas(pool: PgPool) -> Router {
    Router::new()
        .route("/", get(listar_colecciones).post(crear_coleccion))
        .route(
            "/:id",
            get(obtener_coleccion)
                .put(actualizar_coleccion)
                .delete(eliminar_coleccion),
        )
        .with_state(pool)
}

/// Valida que los campos obligatorios de una colección no vengan vacíos.
fn validar_coleccion_dto(dto: &CrearColeccionDto) -> ApiResult<()> {
    if dto.nombre.trim().is_empty() || dto.descripcion.trim().is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "El nombre y la descripción de la colección son obligatorios",
        ));
    }
    Ok(())
}

/// Lista todas las colecciones ordenadas por identificador ascendente.
#[utoipa::path(
    get,
    path = "/api/colecciones",
    tag = "Colecciones",
    responses(
        (status = 200, description = "Lista de colecciones disponibles", body = [Coleccion]),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn listar_colecciones(State(pool): State<PgPool>) -> ApiResult<Json<Vec<Coleccion>>> {
    let colecciones = sqlx::query_as::<_, Coleccion>(
        "SELECT id, nombre, anio, descripcion FROM colecciones ORDER BY id ASC",
    )
    .fetch_all(&pool)
    .await
    .map_err(mapear_error_db)?;

    Ok(Json(colecciones))
}

/// Obtiene el detalle de una colección específica por su ID.
#[utoipa::path(
    get,
    path = "/api/colecciones/{id}",
    tag = "Colecciones",
    params(
        ("id" = i32, Path, description = "Identificador único de la colección")
    ),
    responses(
        (status = 200, description = "Colección encontrada", body = Coleccion),
        (status = 404, description = "Colección no encontrada", body = ApiError),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn obtener_coleccion(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> ApiResult<Json<Coleccion>> {
    let coleccion = sqlx::query_as::<_, Coleccion>(
        "SELECT id, nombre, anio, descripcion FROM colecciones WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(mapear_error_db)?;

    match coleccion {
        Some(item) => Ok(Json(item)),
        None => Err(api_error(StatusCode::NOT_FOUND, "Colección no encontrada")),
    }
}

/// Crea una nueva colección de ropa (requiere rol vendedor).
#[utoipa::path(
    post,
    path = "/api/colecciones",
    tag = "Colecciones",
    request_body = CrearColeccionDto,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Colección creada exitosamente", body = Coleccion),
        (status = 400, description = "Datos inválidos", body = ApiError),
        (status = 401, description = "No autenticado", body = ApiError),
        (status = 403, description = "Acceso denegado: requiere rol vendedor", body = ApiError),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn crear_coleccion(
    SoloVendedor(_claims): SoloVendedor,
    State(pool): State<PgPool>,
    Json(dto): Json<CrearColeccionDto>,
) -> ApiResult<(StatusCode, Json<Coleccion>)> {
    validar_coleccion_dto(&dto)?;

    let coleccion = sqlx::query_as::<_, Coleccion>(
        "INSERT INTO colecciones (nombre, anio, descripcion)
         VALUES ($1, $2, $3)
         RETURNING id, nombre, anio, descripcion",
    )
    .bind(dto.nombre.trim())
    .bind(dto.anio)
    .bind(dto.descripcion.trim())
    .fetch_one(&pool)
    .await
    .map_err(mapear_error_db)?;

    Ok((StatusCode::CREATED, Json(coleccion)))
}

/// Actualiza todos los campos de una colección existente (requiere rol vendedor).
#[utoipa::path(
    put,
    path = "/api/colecciones/{id}",
    tag = "Colecciones",
    params(
        ("id" = i32, Path, description = "Identificador único de la colección")
    ),
    request_body = CrearColeccionDto,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Colección actualizada", body = Coleccion),
        (status = 400, description = "Datos inválidos", body = ApiError),
        (status = 401, description = "No autenticado", body = ApiError),
        (status = 403, description = "Acceso denegado: requiere rol vendedor", body = ApiError),
        (status = 404, description = "Colección no encontrada", body = ApiError),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn actualizar_coleccion(
    SoloVendedor(_claims): SoloVendedor,
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Json(dto): Json<CrearColeccionDto>,
) -> ApiResult<Json<Coleccion>> {
    validar_coleccion_dto(&dto)?;

    let coleccion = sqlx::query_as::<_, Coleccion>(
        "UPDATE colecciones
         SET nombre = $1, anio = $2, descripcion = $3
         WHERE id = $4
         RETURNING id, nombre, anio, descripcion",
    )
    .bind(dto.nombre.trim())
    .bind(dto.anio)
    .bind(dto.descripcion.trim())
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(mapear_error_db)?;

    match coleccion {
        Some(item) => Ok(Json(item)),
        None => Err(api_error(StatusCode::NOT_FOUND, "Colección no encontrada")),
    }
}

/// Elimina una colección por su ID (requiere rol vendedor).
#[utoipa::path(
    delete,
    path = "/api/colecciones/{id}",
    tag = "Colecciones",
    params(
        ("id" = i32, Path, description = "Identificador único de la colección")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Colección eliminada correctamente", body = MensajeResponse),
        (status = 401, description = "No autenticado", body = ApiError),
        (status = 403, description = "Acceso denegado: requiere rol vendedor", body = ApiError),
        (status = 404, description = "Colección no encontrada", body = ApiError),
        (status = 409, description = "La colección tiene prendas asociadas", body = ApiError),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn eliminar_coleccion(
    SoloVendedor(_claims): SoloVendedor,
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> ApiResult<Json<MensajeResponse>> {
    let resultado = sqlx::query("DELETE FROM colecciones WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|err| {
            if es_codigo_pg(&err, "23503") {
                api_error(
                    StatusCode::CONFLICT,
                    "No se puede eliminar la colección porque tiene prendas asociadas",
                )
            } else {
                mapear_error_db(err)
            }
        })?;

    if resultado.rows_affected() == 0 {
        return Err(api_error(StatusCode::NOT_FOUND, "Colección no encontrada"));
    }

    Ok(Json(MensajeResponse {
        mensaje: format!("Colección {id} eliminada correctamente"),
    }))
}
