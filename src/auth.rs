use std::fmt;
use std::str::FromStr;

use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::Json;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::models::{ApiError, Claims, LoginRequest, LoginResponse};

/// Secreto por defecto usado únicamente en desarrollo si no se define `JWT_SECRET`.
const SECRETO_DEFAULT: &str = "secreto_didactico_tienda_ropa_jwt_2026";

/// Roles de usuario soportados por la API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rol {
    /// Administrador/vendedor con acceso completo al catálogo, clientes y envíos.
    Vendedor,
    /// Cliente con acceso de consulta y creación de sus propios envíos.
    Cliente,
}

impl Rol {
    /// Devuelve la representación textual en minúsculas usada dentro del JWT.
    pub fn as_str(&self) -> &'static str {
        match self {
            Rol::Vendedor => "vendedor",
            Rol::Cliente => "cliente",
        }
    }
}

impl fmt::Display for Rol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Rol {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "vendedor" => Ok(Rol::Vendedor),
            "cliente" => Ok(Rol::Cliente),
            otro => Err(format!("Rol no reconocido: {otro}")),
        }
    }
}

/// Obtiene la clave secreta para firmar/verificar JWT desde `JWT_SECRET` o usa un valor por defecto.
fn obtener_secreto() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| SECRETO_DEFAULT.to_string())
}

/// Genera y firma un token JWT (algoritmo HS256) con vigencia de 24 horas.
pub fn generar_token(
    correo: &str,
    rol: Rol,
    cliente_id: Option<i32>,
) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = (Utc::now() + Duration::hours(24)).timestamp() as usize;
    let claims = Claims {
        sub: correo.to_string(),
        rol: rol.as_str().to_string(),
        cliente_id,
        exp,
    };

    let secreto = obtener_secreto();
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secreto.as_bytes()),
    )
}

/// Valida la firma y la fecha de expiración de un JWT y retorna sus `Claims`.
#[allow(dead_code)]
pub fn validar_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let secreto = obtener_secreto();
    let datos = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secreto.as_bytes()),
        &Validation::default(),
    )?;
    Ok(datos.claims)
}

/// Inicia sesión validando credenciales de prueba y devolviendo un token JWT.
#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login exitoso", body = LoginResponse),
        (status = 401, description = "Credenciales inválidas", body = ApiError)
    ),
    tag = "Autenticación"
)]
pub async fn login(
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ApiError>)> {
    // Credenciales didácticas fijas para demostrar autorización basada en roles (RBAC).
    let (rol, cliente_id) = match (payload.correo.as_str(), payload.password.as_str()) {
        ("vendedor@tienda.mx", "vendedor123") => (Rol::Vendedor, None),
        ("sofia.mendoza@correo.mx", "cliente123") => (Rol::Cliente, Some(1)),
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ApiError::new("Credenciales inválidas")),
            ));
        }
    };

    let token = generar_token(&payload.correo, rol, cliente_id).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new("Error interno al firmar el token JWT")),
        )
    })?;

    Ok(Json(LoginResponse {
        token,
        rol: rol.as_str().to_string(),
        correo: payload.correo,
        cliente_id,
    }))
}

/// Función auxiliar compartida para extraer y validar el Bearer token desde los headers HTTP.
#[allow(dead_code)]
fn extraer_claims(parts: &Parts) -> Result<Claims, (StatusCode, Json<ApiError>)> {
    let encabezado = parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|valor| valor.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ApiError::new(
                    "Falta el encabezado Authorization: Bearer <token>",
                )),
            )
        })?;

    let token = encabezado.strip_prefix("Bearer ").ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError::new(
                "Encabezado Authorization inválido; use formato 'Bearer <token>'",
            )),
        )
    })?;

    let claims = validar_token(token).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError::new("Token inválido o expirado")),
        )
    })?;

    if claims.rol.parse::<Rol>().is_err() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiError::new("Token con rol desconocido")),
        ));
    }

    Ok(claims)
}

/// Extractor de Axum que exige un JWT válido con rol `"vendedor"`.
///
/// - Si falta el token o es inválido: responde `401 Unauthorized`.
/// - Si el token es válido pero el rol no es `"vendedor"`: responde `403 Forbidden`.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SoloVendedor(pub Claims);

#[axum::async_trait]
impl<S> FromRequestParts<S> for SoloVendedor
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ApiError>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = extraer_claims(parts)?;

        if claims.rol != Rol::Vendedor.as_str() {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ApiError::new("Acceso denegado: se requiere rol de vendedor")),
            ));
        }

        Ok(SoloVendedor(claims))
    }
}

/// Extractor de Axum que acepta cualquier usuario autenticado (`"vendedor"` o `"cliente"`).
///
/// - Si falta el token o es inválido: responde `401 Unauthorized`.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct UsuarioAutenticado(pub Claims);

#[axum::async_trait]
impl<S> FromRequestParts<S> for UsuarioAutenticado
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ApiError>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = extraer_claims(parts)?;
        Ok(UsuarioAutenticado(claims))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header::CONTENT_TYPE, Request, StatusCode};
    use axum::routing::{get, post};
    use axum::Router;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[test]
    fn test_generar_y_validar_token_vendedor() {
        let token = generar_token("vendedor@tienda.mx", Rol::Vendedor, None)
            .expect("Debe firmar el token de vendedor");

        let claims = validar_token(&token).expect("Debe validar el token recién generado");
        assert_eq!(claims.sub, "vendedor@tienda.mx");
        assert_eq!(claims.rol, "vendedor");
        assert_eq!(claims.cliente_id, None);
    }

    #[test]
    fn test_token_invalido() {
        let resultado = validar_token("este.token.no.es.valido");
        assert!(resultado.is_err(), "Un token malformado debe fallar");
    }

    #[test]
    fn test_rol_cliente() {
        let token = generar_token("sofia.mendoza@correo.mx", Rol::Cliente, Some(1))
            .expect("Debe firmar el token de cliente");

        let claims = validar_token(&token).expect("Debe validar el token de cliente");
        assert_eq!(claims.sub, "sofia.mendoza@correo.mx");
        assert_eq!(claims.rol, "cliente");
        assert_eq!(claims.cliente_id, Some(1));
        assert_eq!(claims.rol.parse::<Rol>(), Ok(Rol::Cliente));
    }

    #[tokio::test]
    async fn test_extractores_y_login() {
        async fn ruta_vendedor(SoloVendedor(claims): SoloVendedor) -> String {
            claims.sub
        }

        async fn ruta_autenticado(UsuarioAutenticado(claims): UsuarioAutenticado) -> String {
            claims.rol
        }

        let app = Router::new()
            .route("/api/auth/login", post(login))
            .route("/solo-vendedor", get(ruta_vendedor))
            .route("/cualquiera", get(ruta_autenticado));

        // Login exitoso vendedor
        let resp_login_vendedor = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"correo":"vendedor@tienda.mx","password":"vendedor123"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp_login_vendedor.status(), StatusCode::OK);
        let body_bytes = resp_login_vendedor
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let login_vendedor: LoginResponse = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(login_vendedor.rol, "vendedor");
        assert_eq!(login_vendedor.cliente_id, None);

        // Login exitoso cliente
        let resp_login_cliente = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"correo":"sofia.mendoza@correo.mx","password":"cliente123"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp_login_cliente.status(), StatusCode::OK);
        let body_bytes = resp_login_cliente
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let login_cliente: LoginResponse = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(login_cliente.rol, "cliente");
        assert_eq!(login_cliente.cliente_id, Some(1));

        // Login inválido -> 401
        let resp_login_invalido = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"correo":"vendedor@tienda.mx","password":"mal"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp_login_invalido.status(), StatusCode::UNAUTHORIZED);

        // Sin token -> 401
        let resp_sin_token = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/solo-vendedor")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp_sin_token.status(), StatusCode::UNAUTHORIZED);

        // Cliente intentando acceder a ruta de SoloVendedor -> 403
        let resp_forbidden = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/solo-vendedor")
                    .header(AUTHORIZATION, format!("Bearer {}", login_cliente.token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp_forbidden.status(), StatusCode::FORBIDDEN);

        // Cliente accediendo a UsuarioAutenticado -> 200
        let resp_cliente_ok = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/cualquiera")
                    .header(AUTHORIZATION, format!("Bearer {}", login_cliente.token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp_cliente_ok.status(), StatusCode::OK);

        // Vendedor accediendo a SoloVendedor -> 200
        let resp_vendedor_ok = app
            .oneshot(
                Request::builder()
                    .uri("/solo-vendedor")
                    .header(AUTHORIZATION, format!("Bearer {}", login_vendedor.token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp_vendedor_ok.status(), StatusCode::OK);
    }
}
