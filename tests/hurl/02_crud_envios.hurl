# ==============================================================================
# Prueba CRUD Completo sobre Envíos (Tabla principal de 8 columnas)
# ==============================================================================
# Demuestra el ciclo completo Read -> Create -> Update -> Delete sobre /api/envios
# autenticándose como vendedor para operar con permisos completos.
#
# Ejecución:
#   hurl --test tests/hurl/02_crud_envios.hurl
# ==============================================================================

# ------------------------------------------------------------------------------
# Autenticación previa como vendedor para obtener el Bearer token
# ------------------------------------------------------------------------------
POST http://localhost:3000/api/auth/login
Content-Type: application/json
{
  "correo": "vendedor@tienda.mx",
  "password": "vendedor123"
}
HTTP 200
[Captures]
token: jsonpath "$.token"
[Asserts]
jsonpath "$.rol" == "vendedor"

# ------------------------------------------------------------------------------
# Paso 1 (READ): Listar todos los envíos existentes
# ------------------------------------------------------------------------------
GET http://localhost:3000/api/envios
Authorization: Bearer {{token}}
HTTP 200
[Asserts]
jsonpath "$" isCollection
jsonpath "$[0].id" isInteger
jsonpath "$[0].paqueteria" isString
jsonpath "$[0].numero_guia" isString
jsonpath "$[0].estatus" isString

# ------------------------------------------------------------------------------
# Paso 2 (CREATE): Crear un nuevo envío y capturar su ID generado
# (Descuenta stock de la prenda dentro de una transacción SQL)
# ------------------------------------------------------------------------------
POST http://localhost:3000/api/envios
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "paqueteria": "DHL Express",
  "numero_guia": "DHL-DEMO-999",
  "cantidad": 1,
  "cliente_id": 1,
  "prenda_id": 1
}
HTTP 201
[Captures]
nuevo_id: jsonpath "$.id"
[Asserts]
jsonpath "$.id" isInteger
jsonpath "$.paqueteria" == "DHL Express"
jsonpath "$.numero_guia" == "DHL-DEMO-999"
jsonpath "$.cantidad" == 1
jsonpath "$.cliente_id" == 1
jsonpath "$.prenda_id" == 1

# ------------------------------------------------------------------------------
# Paso 3 (UPDATE): Actualizar estatus y datos de seguimiento del envío creado
# ------------------------------------------------------------------------------
PUT http://localhost:3000/api/envios/{{nuevo_id}}
Authorization: Bearer {{token}}
Content-Type: application/json
{
  "paqueteria": "DHL Express",
  "numero_guia": "DHL-DEMO-999",
  "estatus": "En tránsito"
}
HTTP 200
[Asserts]
jsonpath "$.id" == {{nuevo_id}}
jsonpath "$.paqueteria" == "DHL Express"
jsonpath "$.numero_guia" == "DHL-DEMO-999"
jsonpath "$.estatus" == "En tránsito"

# ------------------------------------------------------------------------------
# Paso 4 (DELETE): Eliminar el envío de demostración
# (Restaura el stock de la prenda en la base de datos)
# ------------------------------------------------------------------------------
DELETE http://localhost:3000/api/envios/{{nuevo_id}}
Authorization: Bearer {{token}}
HTTP 200
