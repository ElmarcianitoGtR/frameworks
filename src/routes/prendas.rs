use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use sqlx::PgPool;

use crate::auth::SoloVendedor;
use crate::models::{ApiError, CrearPrendaDto, MensajeResponse, Prenda};
use crate::routes::{api_error, es_codigo_pg, mapear_error_db, ApiResult};

/// Registra las rutas del recurso `/api/prendas`.
pub fn rutas(pool: PgPool) -> Router {
    Router::new()
        .route("/", get(listar_prendas).post(crear_prenda))
        .route(
            "/:id",
            get(obtener_prenda)
                .put(actualizar_prenda)
                .delete(eliminar_prenda),
        )
        .with_state(pool)
}

/// Valida los campos de negocio de una prenda (textos no vacíos, precio y stock no negativos).
fn validar_prenda_dto(dto: &CrearPrendaDto) -> ApiResult<()> {
    if dto.nombre.trim().is_empty() || dto.talla.trim().is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "El nombre y la talla de la prenda son obligatorios",
        ));
    }
    if !dto.precio.is_finite() || dto.precio < 0.0 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "El precio debe ser un número válido mayor o igual a cero",
        ));
    }
    if dto.stock < 0 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "El stock no puede ser negativo",
        ));
    }
    Ok(())
}

/// Lista todas las prendas del catálogo ordenadas por ID ascendente.
#[utoipa::path(
    get,
    path = "/api/prendas",
    tag = "Prendas",
    responses(
        (status = 200, description = "Catálogo completo de prendas", body = [Prenda]),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn listar_prendas(State(pool): State<PgPool>) -> ApiResult<Json<Vec<Prenda>>> {
    // PostgreSQL almacena `precio` como NUMERIC(10,2); se convierte a float8 para `f64`.
    let prendas = sqlx::query_as::<_, Prenda>(
        "SELECT id, nombre, talla, precio::float8 AS precio, stock, coleccion_id
         FROM prendas
         ORDER BY id ASC",
    )
    .fetch_all(&pool)
    .await
    .map_err(mapear_error_db)?;

    Ok(Json(prendas))
}

/// Obtiene una prenda específica por su ID.
#[utoipa::path(
    get,
    path = "/api/prendas/{id}",
    tag = "Prendas",
    params(
        ("id" = i32, Path, description = "Identificador único de la prenda")
    ),
    responses(
        (status = 200, description = "Prenda encontrada", body = Prenda),
        (status = 404, description = "Prenda no encontrada", body = ApiError),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn obtener_prenda(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> ApiResult<Json<Prenda>> {
    let prenda = sqlx::query_as::<_, Prenda>(
        "SELECT id, nombre, talla, precio::float8 AS precio, stock, coleccion_id
         FROM prendas
         WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(mapear_error_db)?;

    match prenda {
        Some(item) => Ok(Json(item)),
        None => Err(api_error(StatusCode::NOT_FOUND, "Prenda no encontrada")),
    }
}

/// Registra una nueva prenda en el catálogo (requiere rol vendedor).
#[utoipa::path(
    post,
    path = "/api/prendas",
    tag = "Prendas",
    request_body = CrearPrendaDto,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Prenda registrada exitosamente", body = Prenda),
        (status = 400, description = "Datos inválidos o colección inexistente", body = ApiError),
        (status = 401, description = "No autenticado", body = ApiError),
        (status = 403, description = "Acceso denegado: requiere rol vendedor", body = ApiError),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn crear_prenda(
    SoloVendedor(_claims): SoloVendedor,
    State(pool): State<PgPool>,
    Json(dto): Json<CrearPrendaDto>,
) -> ApiResult<(StatusCode, Json<Prenda>)> {
    validar_prenda_dto(&dto)?;

    let prenda = sqlx::query_as::<_, Prenda>(
        "INSERT INTO prendas (nombre, talla, precio, stock, coleccion_id)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, nombre, talla, precio::float8 AS precio, stock, coleccion_id",
    )
    .bind(dto.nombre.trim())
    .bind(dto.talla.trim())
    .bind(dto.precio)
    .bind(dto.stock)
    .bind(dto.coleccion_id)
    .fetch_one(&pool)
    .await
    .map_err(|err| {
        if es_codigo_pg(&err, "23503") {
            api_error(StatusCode::BAD_REQUEST, "La colección especificada no existe")
        } else {
            mapear_error_db(err)
        }
    })?;

    Ok((StatusCode::CREATED, Json(prenda)))
}

/// Actualiza los datos e inventario de una prenda existente (requiere rol vendedor).
#[utoipa::path(
    put,
    path = "/api/prendas/{id}",
    tag = "Prendas",
    params(
        ("id" = i32, Path, description = "Identificador único de la prenda")
    ),
    request_body = CrearPrendaDto,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Prenda actualizada", body = Prenda),
        (status = 400, description = "Datos inválidos o colección inexistente", body = ApiError),
        (status = 401, description = "No autenticado", body = ApiError),
        (status = 403, description = "Acceso denegado: requiere rol vendedor", body = ApiError),
        (status = 404, description = "Prenda no encontrada", body = ApiError),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn actualizar_prenda(
    SoloVendedor(_claims): SoloVendedor,
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Json(dto): Json<CrearPrendaDto>,
) -> ApiResult<Json<Prenda>> {
    validar_prenda_dto(&dto)?;

    let prenda = sqlx::query_as::<_, Prenda>(
        "UPDATE prendas
         SET nombre = $1, talla = $2, precio = $3, stock = $4, coleccion_id = $5
         WHERE id = $6
         RETURNING id, nombre, talla, precio::float8 AS precio, stock, coleccion_id",
    )
    .bind(dto.nombre.trim())
    .bind(dto.talla.trim())
    .bind(dto.precio)
    .bind(dto.stock)
    .bind(dto.coleccion_id)
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(|err| {
        if es_codigo_pg(&err, "23503") {
            api_error(StatusCode::BAD_REQUEST, "La colección especificada no existe")
        } else {
            mapear_error_db(err)
        }
    })?;

    match prenda {
        Some(item) => Ok(Json(item)),
        None => Err(api_error(StatusCode::NOT_FOUND, "Prenda no encontrada")),
    }
}

/// Elimina una prenda del catálogo por su ID (requiere rol vendedor).
#[utoipa::path(
    delete,
    path = "/api/prendas/{id}",
    tag = "Prendas",
    params(
        ("id" = i32, Path, description = "Identificador único de la prenda")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Prenda eliminada correctamente", body = MensajeResponse),
        (status = 401, description = "No autenticado", body = ApiError),
        (status = 403, description = "Acceso denegado: requiere rol vendedor", body = ApiError),
        (status = 404, description = "Prenda no encontrada", body = ApiError),
        (status = 409, description = "La prenda tiene envíos asociados", body = ApiError),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn eliminar_prenda(
    SoloVendedor(_claims): SoloVendedor,
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> ApiResult<Json<MensajeResponse>> {
    let resultado = sqlx::query("DELETE FROM prendas WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|err| {
            if es_codigo_pg(&err, "23503") {
                api_error(
                    StatusCode::CONFLICT,
                    "No se puede eliminar la prenda porque tiene envíos asociados",
                )
            } else {
                mapear_error_db(err)
            }
        })?;

    if resultado.rows_affected() == 0 {
        return Err(api_error(StatusCode::NOT_FOUND, "Prenda no encontrada"));
    }

    Ok(Json(MensajeResponse {
        mensaje: format!("Prenda {id} eliminada correctamente"),
    }))
}
