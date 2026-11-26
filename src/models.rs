use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

/// Estructura estándar para responder errores HTTP en formato JSON.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    /// Mensaje descriptivo del error ocurrido.
    #[schema(example = "Credenciales inválidas")]
    pub error: String,
}

impl ApiError {
    /// Construye un `ApiError` a partir de cualquier cadena o `&str`.
    pub fn new(mensaje: impl Into<String>) -> Self {
        Self {
            error: mensaje.into(),
        }
    }
}

// ============================================================================
// Modelos de Autenticación (JWT)
// ============================================================================

/// Cuerpo de la petición para iniciar sesión en `/api/auth/login`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// Correo electrónico registrado del usuario.
    #[schema(example = "vendedor@tienda.mx")]
    pub correo: String,
    /// Contraseña de acceso.
    #[schema(example = "vendedor123")]
    pub password: String,
}

/// Respuesta devuelta al autenticarse exitosamente con un token JWT.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginResponse {
    /// Token JWT firmado que debe enviarse en `Authorization: Bearer <token>`.
    pub token: String,
    /// Rol asignado al usuario (`"vendedor"` o `"cliente"`).
    #[schema(example = "vendedor")]
    pub rol: String,
    /// Correo electrónico del usuario autenticado.
    #[schema(example = "vendedor@tienda.mx")]
    pub correo: String,
    /// Identificador del cliente en la base de datos (si el rol es `"cliente"`).
    #[schema(example = 1)]
    pub cliente_id: Option<i32>,
}

/// Carga útil (claims) almacenada dentro del token JWT.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Claims {
    /// Sujeto del token: correo electrónico del usuario.
    pub sub: String,
    /// Rol del usuario (`"vendedor"` o `"cliente"`).
    pub rol: String,
    /// ID de cliente asociado cuando el usuario tiene rol `"cliente"`.
    pub cliente_id: Option<i32>,
    /// Marca de tiempo UNIX (en segundos) de expiración del token.
    pub exp: usize,
}

// ============================================================================
// Modelos del Dominio (Base de Datos y DTOs)
// ============================================================================

/// Representa una colección de ropa (por temporada y año).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Coleccion {
    /// Identificador único de la colección.
    pub id: i32,
    /// Nombre de la colección (ej. "Otoño-Invierno").
    #[schema(example = "Otoño-Invierno")]
    pub nombre: String,
    /// Año de lanzamiento de la colección.
    #[schema(example = 2026)]
    pub anio: i32,
    /// Descripción breve de la temática o prendas incluidas.
    #[schema(example = "Abrigos y tejidos cálidos de temporada")]
    pub descripcion: String,
}

/// Datos requeridos para crear o actualizar una colección.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CrearColeccionDto {
    /// Nombre de la colección.
    #[schema(example = "Primavera-Verano")]
    pub nombre: String,
    /// Año de la colección.
    #[schema(example = 2026)]
    pub anio: i32,
    /// Descripción general de la colección.
    #[schema(example = "Prendas ligeras de algodón y lino")]
    pub descripcion: String,
}

/// Representa una prenda disponible en el catálogo de la tienda.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Prenda {
    /// Identificador único de la prenda.
    pub id: i32,
    /// Nombre comercial de la prenda.
    #[schema(example = "Chamarra de Mezclilla")]
    pub nombre: String,
    /// Talla de la prenda (XS, S, M, L, XL, etc.).
    #[schema(example = "M")]
    pub talla: String,
    /// Precio unitario de venta.
    ///
    /// Nota SQLx + PostgreSQL: en la tabla `prendas`, la columna `precio` suele ser
    /// `NUMERIC`; en las consultas SQL se recomienda castear `precio::float8 AS precio`
    /// para mapearla limpiamente a `f64`.
    #[schema(example = 899.50)]
    pub precio: f64,
    /// Cantidad de unidades disponibles en inventario.
    #[schema(example = 15)]
    pub stock: i32,
    /// Identificador de la colección a la que pertenece la prenda.
    #[schema(example = 1)]
    pub coleccion_id: i32,
}

/// Datos requeridos para registrar o actualizar una prenda.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CrearPrendaDto {
    /// Nombre comercial de la prenda.
    #[schema(example = "Playera Básica")]
    pub nombre: String,
    /// Talla disponible.
    #[schema(example = "M")]
    pub talla: String,
    /// Precio unitario en moneda local.
    #[schema(example = 349.90)]
    pub precio: f64,
    /// Existencias iniciales o actuales en inventario.
    #[schema(example = 25)]
    pub stock: i32,
    /// ID de la colección asociada.
    #[schema(example = 1)]
    pub coleccion_id: i32,
}

/// Representa a un cliente registrado en la tienda.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Cliente {
    /// Identificador único del cliente.
    pub id: i32,
    /// Nombre completo del cliente.
    #[schema(example = "Sofía Mendoza")]
    pub nombre: String,
    /// Dirección física donde se entregan los pedidos.
    #[schema(example = "Av. Reforma 123, Col. Centro, CDMX")]
    pub direccion_envio: String,
    /// Correo electrónico de contacto.
    #[schema(example = "sofia.mendoza@correo.mx")]
    pub correo: String,
}

/// Datos requeridos para registrar o actualizar un cliente.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CrearClienteDto {
    /// Nombre completo del cliente.
    #[schema(example = "Sofía Mendoza")]
    pub nombre: String,
    /// Dirección de entrega.
    #[schema(example = "Av. Reforma 123, Col. Centro, CDMX")]
    pub direccion_envio: String,
    /// Correo electrónico del cliente.
    #[schema(example = "sofia.mendoza@correo.mx")]
    pub correo: String,
}

/// Representa un envío de prendas hacia un cliente.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Envio {
    /// Identificador único del envío.
    pub id: i32,
    /// Fecha en que se despachó o registró el envío (`YYYY-MM-DD`).
    #[schema(example = "2026-11-25")]
    pub fecha_despacho: NaiveDate,
    /// Empresa de mensajería o paquetería encargada.
    #[schema(example = "Estafeta")]
    pub paqueteria: String,
    /// Código de rastreo asignado por la paquetería.
    #[schema(example = "EST-99887766")]
    pub numero_guia: String,
    /// Estado actual del envío (ej. "Preparando", "En tránsito", "Entregado").
    #[schema(example = "En tránsito")]
    pub estatus: String,
    /// Cantidad de unidades de la prenda incluidas en el envío.
    #[schema(example = 2)]
    pub cantidad: i32,
    /// ID del cliente destinatario.
    #[schema(example = 1)]
    pub cliente_id: i32,
    /// ID de la prenda enviada.
    #[schema(example = 1)]
    pub prenda_id: i32,
}

/// Datos requeridos para generar un nuevo envío.
///
/// Cuando quien realiza la petición tiene rol `"cliente"`, el `cliente_id`
/// se obtiene directamente de los claims del token JWT (ignorando o validando
/// el campo opcional).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CrearEnvioDto {
    /// Nombre de la paquetería.
    #[schema(example = "DHL")]
    pub paqueteria: String,
    /// Número de guía de seguimiento.
    #[schema(example = "DHL-12345678")]
    pub numero_guia: String,
    /// Cantidad de prendas a enviar (debe ser mayor a 0).
    #[schema(example = 1)]
    pub cantidad: i32,
    /// ID del cliente destinatario (requerido para vendedor; tomado del JWT si es cliente).
    #[schema(example = 1)]
    pub cliente_id: Option<i32>,
    /// ID de la prenda solicitada.
    #[schema(example = 1)]
    pub prenda_id: i32,
}

/// Datos permitidos para actualizar el estado y seguimiento de un envío.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ActualizarEnvioDto {
    /// Paquetería asignada.
    #[schema(example = "DHL")]
    pub paqueteria: String,
    /// Número de guía actualizado.
    #[schema(example = "DHL-12345678")]
    pub numero_guia: String,
    /// Nuevo estatus del envío (ej. "Preparando", "En tránsito", "Entregado").
    #[schema(example = "Entregado")]
    pub estatus: String,
}

/// Respuesta estándar con un mensaje de confirmación.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MensajeResponse {
    /// Mensaje descriptivo del resultado de la operación.
    #[schema(example = "Recurso eliminado correctamente")]
    pub mensaje: String,
}
