# ==============================================================================
# Prueba RBAC (Control de Acceso Basado en Roles) - API Tienda de Ropa
# ==============================================================================
# Verifica los permisos diferenciados entre los roles "cliente" y "vendedor"
# utilizando tokens JWT y los extractores de Axum (SoloVendedor / público).
#
# Ejecución:
#   hurl --test tests/hurl/01_rbac_permisos.hurl
# ==============================================================================

# ------------------------------------------------------------------------------
# 1. Login como cliente (Sofía Mendoza) y captura de su JWT
# ------------------------------------------------------------------------------
POST http://localhost:3000/api/auth/login
Content-Type: application/json
{
  "correo": "sofia.mendoza@correo.mx",
  "password": "cliente123"
}
HTTP 200
[Captures]
cliente_token: jsonpath "$.token"
[Asserts]
jsonpath "$.token" isString
jsonpath "$.rol" == "cliente"
jsonpath "$.correo" == "sofia.mendoza@correo.mx"
jsonpath "$.cliente_id" == 1

# ------------------------------------------------------------------------------
# 2. Cliente consulta el catálogo de prendas (Endpoint público -> 200 OK)
# ------------------------------------------------------------------------------
GET http://localhost:3000/api/prendas
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[0].id" isInteger

# ------------------------------------------------------------------------------
# 3. Cliente intenta crear una prenda (Requiere SoloVendedor -> 403 Forbidden)
# ------------------------------------------------------------------------------
POST http://localhost:3000/api/prendas
Authorization: Bearer {{cliente_token}}
Content-Type: application/json
{
  "nombre": "Sudadera No Autorizada",
  "talla": "M",
  "precio": 799.00,
  "stock": 10,
  "coleccion_id": 1
}
HTTP 403
[Asserts]
jsonpath "$.error" isString

# ------------------------------------------------------------------------------
# 4. Cliente intenta actualizar el estado de un envío (Requiere SoloVendedor -> 403 Forbidden)
# ------------------------------------------------------------------------------
PUT http://localhost:3000/api/envios/1
Authorization: Bearer {{cliente_token}}
Content-Type: application/json
{
  "paqueteria": "DHL Express",
  "numero_guia": "DHL-MX-900101",
  "estatus": "Entregado"
}
HTTP 403
[Asserts]
jsonpath "$.error" isString

# ------------------------------------------------------------------------------
# 5. Login como vendedor y captura de su JWT con privilegios completos
# ------------------------------------------------------------------------------
POST http://localhost:3000/api/auth/login
Content-Type: application/json
{
  "correo": "vendedor@tienda.mx",
  "password": "vendedor123"
}
HTTP 200
[Captures]
vendedor_token: jsonpath "$.token"
[Asserts]
jsonpath "$.token" isString
jsonpath "$.rol" == "vendedor"
jsonpath "$.correo" == "vendedor@tienda.mx"

# ------------------------------------------------------------------------------
# 6. Vendedor crea una nueva prenda exitosamente (-> 201 Created)
# ------------------------------------------------------------------------------
POST http://localhost:3000/api/prendas
Authorization: Bearer {{vendedor_token}}
Content-Type: application/json
{
  "nombre": "Chamarra Cortavientos RBAC",
  "talla": "L",
  "precio": 1590.00,
  "stock": 15,
  "coleccion_id": 1
}
HTTP 201
[Asserts]
jsonpath "$.id" isInteger
jsonpath "$.nombre" == "Chamarra Cortavientos RBAC"
jsonpath "$.talla" == "L"
jsonpath "$.stock" == 15
jsonpath "$.coleccion_id" == 1
