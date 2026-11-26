use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use sqlx::PgPool;

use crate::auth::{Rol, SoloVendedor, UsuarioAutenticado};
use crate::models::{ActualizarEnvioDto, ApiError, CrearEnvioDto, Envio, MensajeResponse};
use crate::routes::{api_error, es_codigo_pg, mapear_error_db, ApiResult};

/// Registra las rutas del recurso `/api/envios`.
pub fn rutas(pool: PgPool) -> Router {
    Router::new()
        .route("/", get(listar_envios).post(crear_envio))
        .route(
            "/:id",
            get(obtener_envio)
                .put(actualizar_envio)
                .delete(eliminar_envio),
        )
        .with_state(pool)
}

/// Lista los envíos visibles para el usuario autenticado:
/// - Vendedor: todos los envíos.
/// - Cliente: únicamente los envíos asociados a su `cliente_id`.
#[utoipa::path(
    get,
    path = "/api/envios",
    tag = "Envíos",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Lista de envíos según el rol del usuario", body = [Envio]),
        (status = 401, description = "No autenticado", body = ApiError),
        (status = 403, description = "Token de cliente sin cliente_id asociado", body = ApiError),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn listar_envios(
    UsuarioAutenticado(claims): UsuarioAutenticado,
    State(pool): State<PgPool>,
) -> ApiResult<Json<Vec<Envio>>> {
    let envios = if claims.rol == Rol::Vendedor.as_str() {
        sqlx::query_as::<_, Envio>(
            "SELECT id, fecha_despacho, paqueteria, numero_guia, estatus, cantidad, cliente_id, prenda_id
             FROM envios
             ORDER BY id ASC",
        )
        .fetch_all(&pool)
        .await
        .map_err(mapear_error_db)?
    } else {
        let cliente_id = claims.cliente_id.ok_or_else(|| {
            api_error(
                StatusCode::FORBIDDEN,
                "El token de cliente no contiene un cliente_id válido",
            )
        })?;

        sqlx::query_as::<_, Envio>(
            "SELECT id, fecha_despacho, paqueteria, numero_guia, estatus, cantidad, cliente_id, prenda_id
             FROM envios
             WHERE cliente_id = $1
             ORDER BY id ASC",
        )
        .bind(cliente_id)
        .fetch_all(&pool)
        .await
        .map_err(mapear_error_db)?
    };

    Ok(Json(envios))
}

/// Obtiene un envío específico por ID.
/// Si el usuario es cliente, valida que el envío pertenezca a su `cliente_id`.
#[utoipa::path(
    get,
    path = "/api/envios/{id}",
    tag = "Envíos",
    params(
        ("id" = i32, Path, description = "Identificador único del envío")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Envío encontrado", body = Envio),
        (status = 401, description = "No autenticado", body = ApiError),
        (status = 403, description = "Acceso denegado a envíos de otro cliente", body = ApiError),
        (status = 404, description = "Envío no encontrado", body = ApiError),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn obtener_envio(
    UsuarioAutenticado(claims): UsuarioAutenticado,
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> ApiResult<Json<Envio>> {
    let envio = sqlx::query_as::<_, Envio>(
        "SELECT id, fecha_despacho, paqueteria, numero_guia, estatus, cantidad, cliente_id, prenda_id
         FROM envios
         WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(mapear_error_db)?;

    let envio = match envio {
        Some(item) => item,
        None => return Err(api_error(StatusCode::NOT_FOUND, "Envío no encontrado")),
    };

    if claims.rol == Rol::Cliente.as_str() && claims.cliente_id != Some(envio.cliente_id) {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "Acceso denegado: solo puedes consultar tus propios envíos",
        ));
    }

    Ok(Json(envio))
}

/// Crea un nuevo envío dentro de una transacción descontando el stock de la prenda solicitada.
///
/// - Si el usuario tiene rol `"cliente"`, fuerza `cliente_id` desde los claims del JWT.
/// - Si el usuario tiene rol `"vendedor"`, utiliza el `cliente_id` enviado en el cuerpo.
#[utoipa::path(
    post,
    path = "/api/envios",
    tag = "Envíos",
    request_body = CrearEnvioDto,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Envío creado y stock descontado", body = Envio),
        (status = 400, description = "Cantidad inválida, stock insuficiente o cliente inexistente", body = ApiError),
        (status = 401, description = "No autenticado", body = ApiError),
        (status = 403, description = "Token de cliente sin cliente_id asociado", body = ApiError),
        (status = 404, description = "Prenda no encontrada", body = ApiError),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn crear_envio(
    UsuarioAutenticado(claims): UsuarioAutenticado,
    State(pool): State<PgPool>,
    Json(dto): Json<CrearEnvioDto>,
) -> ApiResult<(StatusCode, Json<Envio>)> {
    if dto.cantidad <= 0 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "La cantidad debe ser mayor a cero",
        ));
    }
    if dto.paqueteria.trim().is_empty() || dto.numero_guia.trim().is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "La paquetería y el número de guía son obligatorios",
        ));
    }

    // 1. Determinar el cliente destinatario según el rol autenticado.
    let cliente_id = if claims.rol == Rol::Cliente.as_str() {
        claims.cliente_id.ok_or_else(|| {
            api_error(
                StatusCode::FORBIDDEN,
                "El token de cliente no contiene un cliente_id válido",
            )
        })?
    } else {
        dto.cliente_id.ok_or_else(|| {
            api_error(
                StatusCode::BAD_REQUEST,
                "El campo cliente_id es obligatorio cuando un vendedor registra un envío",
            )
        })?
    };

    // Iniciamos transacción para garantizar consistencia entre stock y registro de envío.
    let mut tx = pool.begin().await.map_err(mapear_error_db)?;

    // 2. Consultar y bloquear la fila de la prenda para evitar condiciones de carrera sobre el stock.
    let stock_actual: Option<i32> =
        sqlx::query_scalar("SELECT stock FROM prendas WHERE id = $1 FOR UPDATE")
            .bind(dto.prenda_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(mapear_error_db)?;

    let stock = match stock_actual {
        Some(valor) => valor,
        None => return Err(api_error(StatusCode::NOT_FOUND, "Prenda no encontrada")),
    };

    if stock < dto.cantidad {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Stock insuficiente para la prenda solicitada",
        ));
    }

    // 3. Descontar el stock de la prenda dentro de la misma transacción.
    sqlx::query("UPDATE prendas SET stock = stock - $1 WHERE id = $2")
        .bind(dto.cantidad)
        .bind(dto.prenda_id)
        .execute(&mut *tx)
        .await
        .map_err(mapear_error_db)?;

    // 4. Insertar el envío con la fecha actual y estatus inicial "Preparando".
    let fecha_hoy = Utc::now().date_naive();
    let envio = sqlx::query_as::<_, Envio>(
        "INSERT INTO envios (fecha_despacho, paqueteria, numero_guia, estatus, cantidad, cliente_id, prenda_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, fecha_despacho, paqueteria, numero_guia, estatus, cantidad, cliente_id, prenda_id",
    )
    .bind(fecha_hoy)
    .bind(dto.paqueteria.trim())
    .bind(dto.numero_guia.trim())
    .bind("Preparando")
    .bind(dto.cantidad)
    .bind(cliente_id)
    .bind(dto.prenda_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| {
        if es_codigo_pg(&err, "23503") {
            api_error(StatusCode::BAD_REQUEST, "El cliente especificado no existe")
        } else {
            mapear_error_db(err)
        }
    })?;

    // 5. Confirmar la transacción.
    tx.commit().await.map_err(mapear_error_db)?;

    // 6. Responder 201 Created con el envío recién creado.
    Ok((StatusCode::CREATED, Json(envio)))
}

/// Actualiza paquetería, número de guía y estatus de un envío (requiere rol vendedor).
#[utoipa::path(
    put,
    path = "/api/envios/{id}",
    tag = "Envíos",
    params(
        ("id" = i32, Path, description = "Identificador único del envío")
    ),
    request_body = ActualizarEnvioDto,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Envío actualizado", body = Envio),
        (status = 400, description = "Datos inválidos", body = ApiError),
        (status = 401, description = "No autenticado", body = ApiError),
        (status = 403, description = "Acceso denegado: requiere rol vendedor", body = ApiError),
        (status = 404, description = "Envío no encontrado", body = ApiError),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn actualizar_envio(
    SoloVendedor(_claims): SoloVendedor,
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Json(dto): Json<ActualizarEnvioDto>,
) -> ApiResult<Json<Envio>> {
    if dto.paqueteria.trim().is_empty()
        || dto.numero_guia.trim().is_empty()
        || dto.estatus.trim().is_empty()
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Paquetería, número de guía y estatus son obligatorios",
        ));
    }

    let envio = sqlx::query_as::<_, Envio>(
        "UPDATE envios
         SET paqueteria = $1, numero_guia = $2, estatus = $3
         WHERE id = $4
         RETURNING id, fecha_despacho, paqueteria, numero_guia, estatus, cantidad, cliente_id, prenda_id",
    )
    .bind(dto.paqueteria.trim())
    .bind(dto.numero_guia.trim())
    .bind(dto.estatus.trim())
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(mapear_error_db)?;

    match envio {
        Some(item) => Ok(Json(item)),
        None => Err(api_error(StatusCode::NOT_FOUND, "Envío no encontrado")),
    }
}

/// Elimina un envío dentro de una transacción y restaura el stock de la prenda asociada
/// (requiere rol vendedor).
#[utoipa::path(
    delete,
    path = "/api/envios/{id}",
    tag = "Envíos",
    params(
        ("id" = i32, Path, description = "Identificador único del envío")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Envío eliminado y stock restaurado", body = MensajeResponse),
        (status = 401, description = "No autenticado", body = ApiError),
        (status = 403, description = "Acceso denegado: requiere rol vendedor", body = ApiError),
        (status = 404, description = "Envío no encontrado", body = ApiError),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn eliminar_envio(
    SoloVendedor(_claims): SoloVendedor,
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> ApiResult<Json<MensajeResponse>> {
    let mut tx = pool.begin().await.map_err(mapear_error_db)?;

    // Eliminamos el envío y recuperamos prenda_id + cantidad en un solo paso atómico.
    let envio_eliminado = sqlx::query_as::<_, Envio>(
        "DELETE FROM envios
         WHERE id = $1
         RETURNING id, fecha_despacho, paqueteria, numero_guia, estatus, cantidad, cliente_id, prenda_id",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(mapear_error_db)?;

    let envio = match envio_eliminado {
        Some(item) => item,
        None => return Err(api_error(StatusCode::NOT_FOUND, "Envío no encontrado")),
    };

    // Restauramos el inventario de la prenda afectada.
    sqlx::query("UPDATE prendas SET stock = stock + $1 WHERE id = $2")
        .bind(envio.cantidad)
        .bind(envio.prenda_id)
        .execute(&mut *tx)
        .await
        .map_err(mapear_error_db)?;

    tx.commit().await.map_err(mapear_error_db)?;

    Ok(Json(MensajeResponse {
        mensaje: format!(
            "Envío {} eliminado y {} unidad(es) restauradas al stock de la prenda {}",
            envio.id, envio.cantidad, envio.prenda_id
        ),
    }))
}
