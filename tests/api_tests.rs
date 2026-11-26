use axum::body::Body;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use tienda::auth::{generar_token, Rol};
use tienda::models::{ApiError, Cliente, Coleccion, Envio, LoginResponse, Prenda};
use tienda::{crear_app, db};
use tower::ServiceExt;

/// Crea un `PgPool` perezoso (no abre conexión inmediata) para pruebas de autorización
/// donde el extractor rechaza la petición antes de tocar la base de datos.
fn pool_lazy() -> sqlx::PgPool {
    PgPoolOptions::new()
        .connect_lazy("postgres://admin:admin123@localhost:5432/tienda_ropa")
        .expect("Debe construir un pool lazy válido")
}

/// Función auxiliar para extraer y deserializar el cuerpo JSON de una respuesta Axum.
async fn leer_json<T: serde::de::DeserializeOwned>(response: axum::response::Response) -> T {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("Debe leer el cuerpo de la respuesta")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("Debe deserializar el JSON de la respuesta")
}

/// Función auxiliar para iniciar sesión contra `/api/auth/login` y obtener el token JWT.
async fn iniciar_sesion(app: &axum::Router, correo: &str, password: &str) -> String {
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "correo": correo,
                "password": password
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: LoginResponse = leer_json(resp).await;
    body.token
}

#[tokio::test]
async fn prueba_1_cliente_no_puede_crear_prenda_403() {
    let app = crear_app(pool_lazy());
    let token_cliente = generar_token("sofia.mendoza@correo.mx", Rol::Cliente, Some(1))
        .expect("Debe generar token de cliente");

    let req = Request::builder()
        .method("POST")
        .uri("/api/prendas")
        .header(AUTHORIZATION, format!("Bearer {token_cliente}"))
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "nombre": "Sudadera Intento",
                "talla": "M",
                "precio": 599.0,
                "stock": 10,
                "coleccion_id": 1
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn prueba_2_cliente_no_puede_actualizar_envio_403() {
    let app = crear_app(pool_lazy());
    let token_cliente = generar_token("sofia.mendoza@correo.mx", Rol::Cliente, Some(1))
        .expect("Debe generar token de cliente");

    let req = Request::builder()
        .method("PUT")
        .uri("/api/envios/1")
        .header(AUTHORIZATION, format!("Bearer {token_cliente}"))
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "paqueteria": "DHL Express",
                "numero_guia": "DHL-TEST-1",
                "estatus": "Entregado"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn prueba_3_crear_envio_descuenta_stock_y_eliminar_lo_restaura() {
    let pool = db::crear_pool().await;
    let app = crear_app(pool.clone());

    let token_cliente = iniciar_sesion(&app, "sofia.mendoza@correo.mx", "cliente123").await;
    let token_vendedor = iniciar_sesion(&app, "vendedor@tienda.mx", "vendedor123").await;

    let stock_inicial: i32 = sqlx::query_scalar("SELECT stock FROM prendas WHERE id = 1")
        .fetch_one(&pool)
        .await
        .expect("La prenda 1 debe existir en la base de datos");
    assert!(
        stock_inicial >= 2,
        "La prenda 1 requiere al menos 2 unidades de stock para la prueba"
    );

    // El cliente envía cliente_id = 999 en el body, pero el backend debe forzar cliente_id = 1 del JWT.
    let req_crear = Request::builder()
        .method("POST")
        .uri("/api/envios")
        .header(AUTHORIZATION, format!("Bearer {token_cliente}"))
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "paqueteria": "DHL Express",
                "numero_guia": "TEST-GUIA-STOCK-01",
                "cantidad": 2,
                "cliente_id": 999,
                "prenda_id": 1
            })
            .to_string(),
        ))
        .unwrap();

    let resp_crear = app.clone().oneshot(req_crear).await.unwrap();
    assert_eq!(resp_crear.status(), StatusCode::CREATED);

    let envio_creado: Envio = leer_json(resp_crear).await;
    assert_eq!(envio_creado.cantidad, 2);
    assert_eq!(envio_creado.cliente_id, 1);
    assert_eq!(envio_creado.prenda_id, 1);
    assert_eq!(envio_creado.estatus, "Preparando");

    let stock_despues: i32 = sqlx::query_scalar("SELECT stock FROM prendas WHERE id = 1")
        .fetch_one(&pool)
        .await
        .expect("Debe consultar el stock actualizado");
    assert_eq!(stock_despues, stock_inicial - 2);

    // Limpieza y verificación de restauración transaccional de stock mediante DELETE /api/envios/:id
    let req_eliminar = Request::builder()
        .method("DELETE")
        .uri(format!("/api/envios/{}", envio_creado.id))
        .header(AUTHORIZATION, format!("Bearer {token_vendedor}"))
        .body(Body::empty())
        .unwrap();

    let resp_eliminar = app.oneshot(req_eliminar).await.unwrap();
    assert_eq!(resp_eliminar.status(), StatusCode::OK);

    let stock_restaurado: i32 = sqlx::query_scalar("SELECT stock FROM prendas WHERE id = 1")
        .fetch_one(&pool)
        .await
        .expect("Debe consultar el stock restaurado");
    assert_eq!(stock_restaurado, stock_inicial);
}

#[tokio::test]
async fn prueba_4_crear_envio_stock_insuficiente_400() {
    let pool = db::crear_pool().await;
    let app = crear_app(pool.clone());
    let token_vendedor = iniciar_sesion(&app, "vendedor@tienda.mx", "vendedor123").await;

    let stock_antes: i32 = sqlx::query_scalar("SELECT stock FROM prendas WHERE id = 2")
        .fetch_one(&pool)
        .await
        .expect("La prenda 2 debe existir");

    let req = Request::builder()
        .method("POST")
        .uri("/api/envios")
        .header(AUTHORIZATION, format!("Bearer {token_vendedor}"))
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "paqueteria": "Estafeta",
                "numero_guia": "TEST-SIN-STOCK",
                "cantidad": stock_antes + 50,
                "cliente_id": 1,
                "prenda_id": 2
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let error: ApiError = leer_json(resp).await;
    assert_eq!(error.error, "Stock insuficiente para la prenda solicitada");

    let stock_despues: i32 = sqlx::query_scalar("SELECT stock FROM prendas WHERE id = 2")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stock_despues, stock_antes);
}

#[tokio::test]
async fn prueba_5_cliente_solo_puede_consultar_su_propio_perfil() {
    let pool = db::crear_pool().await;
    let app = crear_app(pool);

    let token_cliente = iniciar_sesion(&app, "sofia.mendoza@correo.mx", "cliente123").await;
    let token_vendedor = iniciar_sesion(&app, "vendedor@tienda.mx", "vendedor123").await;

    // Cliente consulta su propio perfil (id 1) -> 200 OK
    let resp_propio = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/clientes/1")
                .header(AUTHORIZATION, format!("Bearer {token_cliente}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_propio.status(), StatusCode::OK);
    let cliente: Cliente = leer_json(resp_propio).await;
    assert_eq!(cliente.id, 1);

    // Cliente intenta consultar el perfil de otro cliente (id 2) -> 403 Forbidden
    let resp_ajeno = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/clientes/2")
                .header(AUTHORIZATION, format!("Bearer {token_cliente}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_ajeno.status(), StatusCode::FORBIDDEN);

    // Vendedor consulta cualquier perfil (id 2) -> 200 OK
    let resp_vendedor = app
        .oneshot(
            Request::builder()
                .uri("/api/clientes/2")
                .header(AUTHORIZATION, format!("Bearer {token_vendedor}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_vendedor.status(), StatusCode::OK);
    let cliente2: Cliente = leer_json(resp_vendedor).await;
    assert_eq!(cliente2.id, 2);
}

#[tokio::test]
async fn prueba_6_catalogo_publico_deserializa_colecciones_y_prendas() {
    let pool = db::crear_pool().await;
    let app = crear_app(pool);

    let resp_col = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/colecciones")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_col.status(), StatusCode::OK);
    let colecciones: Vec<Coleccion> = leer_json(resp_col).await;
    assert!(!colecciones.is_empty());

    let resp_prendas = app
        .oneshot(
            Request::builder()
                .uri("/api/prendas")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_prendas.status(), StatusCode::OK);
    let prendas: Vec<Prenda> = leer_json(resp_prendas).await;
    assert!(!prendas.is_empty());
    assert!(prendas.iter().all(|p| p.precio >= 0.0));
}

#[tokio::test]
async fn prueba_7_crud_completo_y_permisos_envios() {
    let pool = db::crear_pool().await;
    let app = crear_app(pool);

    let token_vendedor = iniciar_sesion(&app, "vendedor@tienda.mx", "vendedor123").await;
    let token_cliente = iniciar_sesion(&app, "sofia.mendoza@correo.mx", "cliente123").await;

    // 1. CRUD Colección (POST -> GET -> PUT -> DELETE como vendedor)
    let resp_col = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/colecciones")
                .header(AUTHORIZATION, format!("Bearer {token_vendedor}"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "nombre": "Colección Prueba",
                        "anio": 2026,
                        "descripcion": "Temporal para pruebas"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_col.status(), StatusCode::CREATED);
    let col: Coleccion = leer_json(resp_col).await;

    let resp_col_put = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/colecciones/{}", col.id))
                .header(AUTHORIZATION, format!("Bearer {token_vendedor}"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "nombre": "Colección Actualizada",
                        "anio": 2027,
                        "descripcion": "Descripción editada"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_col_put.status(), StatusCode::OK);
    let col_act: Coleccion = leer_json(resp_col_put).await;
    assert_eq!(col_act.nombre, "Colección Actualizada");
    assert_eq!(col_act.anio, 2027);

    // 2. CRUD Prenda asociada a la colección temporal (valida NUMERIC <-> f64)
    let resp_prenda = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/prendas")
                .header(AUTHORIZATION, format!("Bearer {token_vendedor}"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "nombre": "Prenda Prueba",
                        "talla": "M",
                        "precio": 799.50,
                        "stock": 15,
                        "coleccion_id": col.id
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_prenda.status(), StatusCode::CREATED);
    let prenda: Prenda = leer_json(resp_prenda).await;
    assert!((prenda.precio - 799.50).abs() < 0.01);

    let resp_prenda_put = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/prendas/{}", prenda.id))
                .header(AUTHORIZATION, format!("Bearer {token_vendedor}"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "nombre": "Prenda Editada",
                        "talla": "L",
                        "precio": 850.25,
                        "stock": 20,
                        "coleccion_id": col.id
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_prenda_put.status(), StatusCode::OK);
    let prenda_act: Prenda = leer_json(resp_prenda_put).await;
    assert_eq!(prenda_act.nombre, "Prenda Editada");
    assert!((prenda_act.precio - 850.25).abs() < 0.01);

    // 3. Registro público de cliente, consulta por vendedor, actualización y eliminación
    let resp_cli = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/clientes")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "nombre": "Cliente Temporal",
                        "direccion_envio": "Calle Falsa 123",
                        "correo": "temporal.prueba@correo.mx"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_cli.status(), StatusCode::CREATED);
    let cli: Cliente = leer_json(resp_cli).await;

    let resp_cli_list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/clientes")
                .header(AUTHORIZATION, format!("Bearer {token_vendedor}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_cli_list.status(), StatusCode::OK);

    let resp_cli_put_403 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/clientes/{}", cli.id))
                .header(AUTHORIZATION, format!("Bearer {token_cliente}"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "nombre": "Intento",
                        "direccion_envio": "Otro lado",
                        "correo": "temporal.prueba@correo.mx"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_cli_put_403.status(), StatusCode::FORBIDDEN);

    let resp_cli_put_ok = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/clientes/{}", cli.id))
                .header(AUTHORIZATION, format!("Bearer {token_vendedor}"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "nombre": "Cliente Temporal Editado",
                        "direccion_envio": "Calle Verdadera 456",
                        "correo": "temporal.prueba@correo.mx"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_cli_put_ok.status(), StatusCode::OK);
    let cli_act: Cliente = leer_json(resp_cli_put_ok).await;
    assert_eq!(cli_act.nombre, "Cliente Temporal Editado");

    // 4. Envío listado por cliente (solo cliente_id=1) vs vendedor, GET /:id, PUT /:id (vendedor)
    let resp_envios_cli = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/envios")
                .header(AUTHORIZATION, format!("Bearer {token_cliente}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_envios_cli.status(), StatusCode::OK);
    let envios_cli: Vec<Envio> = leer_json(resp_envios_cli).await;
    assert!(envios_cli.iter().all(|e| e.cliente_id == 1));

    let resp_envio_propio = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/envios/1")
                .header(AUTHORIZATION, format!("Bearer {token_cliente}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_envio_propio.status(), StatusCode::OK);

    let resp_envio_ajeno = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/envios/3")
                .header(AUTHORIZATION, format!("Bearer {token_cliente}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_envio_ajeno.status(), StatusCode::FORBIDDEN);

    // Crear un envío temporal sobre la prenda de prueba y actualizar su estatus con PUT /api/envios/:id
    let resp_envio_tmp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/envios")
                .header(AUTHORIZATION, format!("Bearer {token_vendedor}"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "paqueteria": "FedEx",
                        "numero_guia": "FDX-TMP-001",
                        "cantidad": 1,
                        "cliente_id": cli.id,
                        "prenda_id": prenda.id
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_envio_tmp.status(), StatusCode::CREATED);
    let envio_tmp: Envio = leer_json(resp_envio_tmp).await;

    let resp_envio_put = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/envios/{}", envio_tmp.id))
                .header(AUTHORIZATION, format!("Bearer {token_vendedor}"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "paqueteria": "DHL Express",
                        "numero_guia": "DHL-TMP-002",
                        "estatus": "En tránsito"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_envio_put.status(), StatusCode::OK);
    let envio_act: Envio = leer_json(resp_envio_put).await;
    assert_eq!(envio_act.estatus, "En tránsito");

    // 5. OpenAPI JSON servido correctamente en /api-docs/openapi.json
    let resp_docs = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_docs.status(), StatusCode::OK);

    // 6. Limpieza de registros creados en orden inverso de dependencias FK
    let del_envio = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/envios/{}", envio_tmp.id))
                .header(AUTHORIZATION, format!("Bearer {token_vendedor}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_envio.status(), StatusCode::OK);
    let del_cli = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/clientes/{}", cli.id))
                .header(AUTHORIZATION, format!("Bearer {token_vendedor}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_cli.status(), StatusCode::OK);

    let del_prenda = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/prendas/{}", prenda.id))
                .header(AUTHORIZATION, format!("Bearer {token_vendedor}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_prenda.status(), StatusCode::OK);

    let del_col = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/colecciones/{}", col.id))
                .header(AUTHORIZATION, format!("Bearer {token_vendedor}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_col.status(), StatusCode::OK);
}
