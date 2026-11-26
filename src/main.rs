use tienda::{crear_app, db};

#[tokio::main]
async fn main() {
    // Cargamos variables de entorno desde `.env` si existe
    dotenvy::dotenv().ok();

    // Creamos el pool de conexiones a PostgreSQL
    let pool = db::crear_pool().await;

    // Construimos la aplicación Axum completa
    let app = crear_app(pool);

    let puerto = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let direccion = format!("0.0.0.0:{puerto}");

    let listener = tokio::net::TcpListener::bind(&direccion)
        .await
        .expect("No se pudo iniciar el servidor TCP");

    println!("Servidor Tienda escuchando en http://localhost:{puerto}");
    println!("Documentación Swagger UI en http://localhost:{puerto}/api-docs");

    axum::serve(listener, app)
        .await
        .expect("Error al ejecutar el servidor HTTP");
}

#[cfg(test)]
mod tests {
    use tienda::ApiDoc;
    use utoipa::OpenApi;

    #[test]
    fn test_openapi_contiene_esquemas_y_seguridad() {
        let doc = ApiDoc::openapi();
        let json = serde_json::to_value(&doc).expect("Debe serializar OpenAPI a JSON");

        // Verificar que existe el esquema de seguridad bearer_auth
        assert!(
            json["components"]["securitySchemes"]["bearer_auth"].is_object(),
            "Debe incluir securityScheme bearer_auth"
        );

        // Verificar que existen todos los esquemas del dominio y de errores/auth
        let schemas = &json["components"]["schemas"];
        for nombre in [
            "ApiError",
            "MensajeResponse",
            "LoginRequest",
            "LoginResponse",
            "Coleccion",
            "CrearColeccionDto",
            "Prenda",
            "CrearPrendaDto",
            "Cliente",
            "CrearClienteDto",
            "Envio",
            "CrearEnvioDto",
            "ActualizarEnvioDto",
        ] {
            assert!(schemas[nombre].is_object(), "Falta el esquema {nombre}");
        }

        // Verificar todas las rutas principales registradas en OpenAPI
        for (ruta, metodo) in [
            ("/api/auth/login", "post"),
            ("/api/colecciones", "get"),
            ("/api/colecciones", "post"),
            ("/api/colecciones/{id}", "get"),
            ("/api/colecciones/{id}", "put"),
            ("/api/colecciones/{id}", "delete"),
            ("/api/prendas", "get"),
            ("/api/prendas", "post"),
            ("/api/prendas/{id}", "get"),
            ("/api/prendas/{id}", "put"),
            ("/api/prendas/{id}", "delete"),
            ("/api/clientes", "get"),
            ("/api/clientes", "post"),
            ("/api/clientes/{id}", "get"),
            ("/api/clientes/{id}", "put"),
            ("/api/clientes/{id}", "delete"),
            ("/api/envios", "get"),
            ("/api/envios", "post"),
            ("/api/envios/{id}", "get"),
            ("/api/envios/{id}", "put"),
            ("/api/envios/{id}", "delete"),
        ] {
            assert!(
                json["paths"][ruta][metodo].is_object(),
                "Debe registrar {metodo} {ruta}"
            );
        }
    }
}
