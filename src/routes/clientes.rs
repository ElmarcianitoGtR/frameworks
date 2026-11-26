use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use sqlx::PgPool;

use crate::auth::{Rol, SoloVendedor, UsuarioAutenticado};
use crate::models::{ApiError, Claims, Cliente, CrearClienteDto, MensajeResponse};
use crate::routes::{api_error, es_codigo_pg, mapear_error_db, ApiResult};

/// Registra las rutas del recurso `/api/clientes`.
pub fn rutas(pool: PgPool) -> Router {
    Router::new()
        .route("/", post(registrar_cliente).get(listar_clientes))
        .route(
            "/:id",
            get(obtener_cliente)
                .put(actualizar_cliente)
                .delete(eliminar_cliente),
        )
        .with_state(pool)
}

/// Valida que los campos obligatorios de un cliente estén presentes.
fn validar_cliente_dto(dto: &CrearClienteDto) -> ApiResult<()> {
    if dto.nombre.trim().is_empty()
        || dto.direccion_envio.trim().is_empty()
        || dto.correo.trim().is_empty()
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Nombre, dirección de envío y correo son obligatorios",
        ));
    }
    Ok(())
}

/// Verifica que un cliente autenticado únicamente acceda o modifique su propio `cliente_id`.
/// El rol vendedor tiene permiso sobre cualquier identificador.
fn verificar_permiso_cliente(
    claims: &Claims,
    id_solicitado: i32,
    accion: &str,
) -> ApiResult<()> {
    if claims.rol == Rol::Cliente.as_str() && claims.cliente_id != Some(id_solicitado) {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            format!("Acceso denegado: solo puedes {accion} tu propio perfil"),
        ));
    }
    Ok(())
}

/// Registra un nuevo cliente en la tienda (endpoint público).
#[utoipa::path(
    post,
    path = "/api/clientes",
    tag = "Clientes",
    request_body = CrearClienteDto,
    responses(
        (status = 201, description = "Cliente registrado exitosamente", body = Cliente),
        (status = 400, description = "Datos inválidos", body = ApiError),
        (status = 409, description = "El correo ya está registrado", body = ApiError),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn registrar_cliente(
    State(pool): State<PgPool>,
    Json(dto): Json<CrearClienteDto>,
) -> ApiResult<(StatusCode, Json<Cliente>)> {
    validar_cliente_dto(&dto)?;

    let cliente = sqlx::query_as::<_, Cliente>(
        "INSERT INTO clientes (nombre, direccion_envio, correo)
         VALUES ($1, $2, $3)
         RETURNING id, nombre, direccion_envio, correo",
    )
    .bind(dto.nombre.trim())
    .bind(dto.direccion_envio.trim())
    .bind(dto.correo.trim())
    .fetch_one(&pool)
    .await
    .map_err(|err| {
        if es_codigo_pg(&err, "23505") {
            api_error(
                StatusCode::CONFLICT,
                "Ya existe un cliente registrado con ese correo electrónico",
            )
        } else {
            mapear_error_db(err)
        }
    })?;

    Ok((StatusCode::CREATED, Json(cliente)))
}

/// Lista todos los clientes registrados (requiere rol vendedor).
#[utoipa::path(
    get,
    path = "/api/clientes",
    tag = "Clientes",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Lista completa de clientes", body = [Cliente]),
        (status = 401, description = "No autenticado", body = ApiError),
        (status = 403, description = "Acceso denegado: requiere rol vendedor", body = ApiError),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn listar_clientes(
    SoloVendedor(_claims): SoloVendedor,
    State(pool): State<PgPool>,
) -> ApiResult<Json<Vec<Cliente>>> {
    let clientes = sqlx::query_as::<_, Cliente>(
        "SELECT id, nombre, direccion_envio, correo FROM clientes ORDER BY id ASC",
    )
    .fetch_all(&pool)
    .await
    .map_err(mapear_error_db)?;

    Ok(Json(clientes))
}

/// Consulta el perfil de un cliente por ID.
/// Un cliente solo puede consultar su propio ID; un vendedor puede consultar cualquiera.
#[utoipa::path(
    get,
    path = "/api/clientes/{id}",
    tag = "Clientes",
    params(
        ("id" = i32, Path, description = "Identificador único del cliente")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Perfil del cliente", body = Cliente),
        (status = 401, description = "No autenticado", body = ApiError),
        (status = 403, description = "Acceso denegado al perfil de otro cliente", body = ApiError),
        (status = 404, description = "Cliente no encontrado", body = ApiError),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn obtener_cliente(
    UsuarioAutenticado(claims): UsuarioAutenticado,
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> ApiResult<Json<Cliente>> {
    verificar_permiso_cliente(&claims, id, "consultar")?;

    let cliente = sqlx::query_as::<_, Cliente>(
        "SELECT id, nombre, direccion_envio, correo FROM clientes WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(mapear_error_db)?;

    match cliente {
        Some(item) => Ok(Json(item)),
        None => Err(api_error(StatusCode::NOT_FOUND, "Cliente no encontrado")),
    }
}

/// Actualiza los datos de un cliente por ID.
/// Un cliente solo puede editar su propio perfil; un vendedor puede editar cualquiera.
#[utoipa::path(
    put,
    path = "/api/clientes/{id}",
    tag = "Clientes",
    params(
        ("id" = i32, Path, description = "Identificador único del cliente")
    ),
    request_body = CrearClienteDto,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Cliente actualizado", body = Cliente),
        (status = 400, description = "Datos inválidos", body = ApiError),
        (status = 401, description = "No autenticado", body = ApiError),
        (status = 403, description = "Acceso denegado al perfil de otro cliente", body = ApiError),
        (status = 404, description = "Cliente no encontrado", body = ApiError),
        (status = 409, description = "El correo ya está registrado por otro cliente", body = ApiError),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn actualizar_cliente(
    UsuarioAutenticado(claims): UsuarioAutenticado,
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Json(dto): Json<CrearClienteDto>,
) -> ApiResult<Json<Cliente>> {
    verificar_permiso_cliente(&claims, id, "actualizar")?;
    validar_cliente_dto(&dto)?;

    let cliente = sqlx::query_as::<_, Cliente>(
        "UPDATE clientes
         SET nombre = $1, direccion_envio = $2, correo = $3
         WHERE id = $4
         RETURNING id, nombre, direccion_envio, correo",
    )
    .bind(dto.nombre.trim())
    .bind(dto.direccion_envio.trim())
    .bind(dto.correo.trim())
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(|err| {
        if es_codigo_pg(&err, "23505") {
            api_error(
                StatusCode::CONFLICT,
                "Ya existe otro cliente registrado con ese correo electrónico",
            )
        } else {
            mapear_error_db(err)
        }
    })?;

    match cliente {
        Some(item) => Ok(Json(item)),
        None => Err(api_error(StatusCode::NOT_FOUND, "Cliente no encontrado")),
    }
}

/// Elimina un cliente por su ID (requiere rol vendedor).
#[utoipa::path(
    delete,
    path = "/api/clientes/{id}",
    tag = "Clientes",
    params(
        ("id" = i32, Path, description = "Identificador único del cliente")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Cliente eliminado correctamente", body = MensajeResponse),
        (status = 401, description = "No autenticado", body = ApiError),
        (status = 403, description = "Acceso denegado: requiere rol vendedor", body = ApiError),
        (status = 404, description = "Cliente no encontrado", body = ApiError),
        (status = 409, description = "El cliente tiene envíos asociados", body = ApiError),
        (status = 500, description = "Error interno del servidor", body = ApiError)
    )
)]
pub async fn eliminar_cliente(
    SoloVendedor(_claims): SoloVendedor,
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> ApiResult<Json<MensajeResponse>> {
    let resultado = sqlx::query("DELETE FROM clientes WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|err| {
            if es_codigo_pg(&err, "23503") {
                api_error(
                    StatusCode::CONFLICT,
                    "No se puede eliminar el cliente porque tiene envíos asociados",
                )
            } else {
                mapear_error_db(err)
            }
        })?;

    if resultado.rows_affected() == 0 {
        return Err(api_error(StatusCode::NOT_FOUND, "Cliente no encontrado"));
    }

    Ok(Json(MensajeResponse {
        mensaje: format!("Cliente {id} eliminado correctamente"),
    }))
}
