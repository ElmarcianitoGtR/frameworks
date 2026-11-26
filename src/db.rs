use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Crea y configura el pool de conexiones a PostgreSQL utilizando SQLx.
///
/// Lee la variable de entorno `DATABASE_URL` (cargando previamente el archivo `.env`
/// si existe en el directorio del proyecto) y abre un pool con un máximo de 5 conexiones.
pub async fn crear_pool() -> PgPool {
    // Cargar variables desde `.env` si el archivo existe; no falla si ya están en el entorno.
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect(
        "La variable de entorno DATABASE_URL debe estar definida (ej. postgres://usuario:clave@localhost:5432/tienda)",
    );

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Error al conectar a PostgreSQL")
}
