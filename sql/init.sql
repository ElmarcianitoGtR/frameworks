-- Esquema de base de datos para la tienda de ropa

CREATE TABLE IF NOT EXISTS colecciones (
    id SERIAL PRIMARY KEY,
    nombre VARCHAR(100) NOT NULL,
    anio INT NOT NULL,
    descripcion TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS prendas (
    id SERIAL PRIMARY KEY,
    nombre VARCHAR(150) NOT NULL,
    talla VARCHAR(10) NOT NULL,
    precio NUMERIC(10,2) NOT NULL CHECK (precio >= 0),
    stock INT NOT NULL CHECK (stock >= 0),
    coleccion_id INT NOT NULL REFERENCES colecciones(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS clientes (
    id SERIAL PRIMARY KEY,
    nombre VARCHAR(150) NOT NULL,
    direccion_envio TEXT NOT NULL,
    correo VARCHAR(150) UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS envios (
    id SERIAL PRIMARY KEY,
    fecha_despacho DATE NOT NULL,
    paqueteria VARCHAR(100) NOT NULL,
    numero_guia VARCHAR(100) NOT NULL,
    estatus VARCHAR(50) NOT NULL,
    cantidad INT NOT NULL CHECK (cantidad > 0),
    cliente_id INT NOT NULL REFERENCES clientes(id) ON DELETE RESTRICT,
    prenda_id INT NOT NULL REFERENCES prendas(id) ON DELETE RESTRICT
);

-- 10 registros en colecciones
INSERT INTO colecciones (nombre, anio, descripcion) VALUES
('Primavera Urbana', 2026, 'Prendas ligeras y cortes relajados inspirados en la arquitectura contemporánea.'),
('Esenciales Minimal', 2026, 'Básicos atemporales en algodón orgánico y paleta neutra.'),
('Otoño Terracota', 2025, 'Tejidos cálidos en tonos tierra, ocres y borgoña.'),
('Invierno Nórdico', 2025, 'Abrigos estructurados, lana merino y capas térmicas.'),
('Verano Costero', 2026, 'Lino transpirable y siluetas frescas para clima cálido.'),
('Denim Artesanal', 2025, 'Mezclilla selvedge de alto gramaje con acabados lavados.'),
('Cápsula Nocturna', 2026, 'Sastrería moderna y telas satinadas para ocasiones especiales.'),
('Movimiento Activo', 2026, 'Ropa deportiva técnica con tejidos elásticos y secado rápido.'),
('Herencia Mexicana', 2025, 'Bordados contemporáneos y textiles naturales hechos en taller.'),
('Edición Cáñamo', 2026, 'Fibras sostenibles de cáñamo y tencel con tintes vegetales.');

-- 10 registros en prendas (stock > 10 en todas para pruebas)
INSERT INTO prendas (nombre, talla, precio, stock, coleccion_id) VALUES
('Chaqueta Bomber Ligera', 'M', 1490.00, 25, 1),
('Camiseta Oversize Algodón', 'L', 490.00, 60, 2),
('Suéter Cuello Alto Terracota', 'S', 1150.00, 30, 3),
('Abrigo Lana Cruzado', 'M', 2890.00, 18, 4),
('Camisa Lino Manga Corta', 'L', 890.00, 42, 5),
('Jeans Rectos Selvedge', '32', 1650.00, 35, 6),
('Blazer Sastre Negro', 'M', 2250.00, 22, 7),
('Sudadera Técnica Zip', 'S', 1290.00, 40, 8),
('Blusa Bordada Artesanal', 'M', 1380.00, 20, 9),
('Pantalón Chino Cáñamo', '30', 1190.00, 50, 10);

-- 10 registros en clientes (id 1 = Sofía Mendoza)
INSERT INTO clientes (nombre, direccion_envio, correo) VALUES
('Sofía Mendoza', 'Av. Álvaro Obregón 145, Col. Roma Norte, CDMX', 'sofia.mendoza@correo.mx'),
('Mateo Hernández', 'Calle Hidalgo 412, Col. Centro, Guadalajara, JAL', 'mateo.hernandez@correo.mx'),
('Valentina Cruz', 'Calz. del Valle 220, San Pedro Garza García, NL', 'valentina.cruz@correo.mx'),
('Diego Ramírez', 'Av. Juárez 88, Col. Centro, Puebla, PUE', 'diego.ramirez@correo.mx'),
('Camila Torres', 'Calle Montejo 305, Col. México, Mérida, YUC', 'camila.torres@correo.mx'),
('Sebastián Morales', 'Blvd. Agua Caliente 910, Col. Aviación, Tijuana, BC', 'sebastian.morales@correo.mx'),
('Lucía Navarro', 'Av. Universidad 514, Col. Narvarte, CDMX', 'lucia.navarro@correo.mx'),
('Alejandro Vega', 'Calle Madero 120, Col. Centro, Querétaro, QRO', 'alejandro.vega@correo.mx'),
('Mariana Castillo', 'Av. Revolución 670, Col. San Ángel, CDMX', 'mariana.castillo@correo.mx'),
('Emiliano Rojas', 'Calle Morelos 255, Col. Reforma, Oaxaca, OAX', 'emiliano.rojas@correo.mx');

-- 10 registros en envios
INSERT INTO envios (fecha_despacho, paqueteria, numero_guia, estatus, cantidad, cliente_id, prenda_id) VALUES
('2026-11-10', 'DHL Express', 'DHL-MX-900101', 'Entregado', 2, 1, 2),
('2026-11-12', 'Estafeta', 'EST-MX-882310', 'En tránsito', 1, 1, 7),
('2026-11-13', 'FedEx', 'FDX-MX-771204', 'Entregado', 1, 2, 6),
('2026-11-14', 'DHL Express', 'DHL-MX-900245', 'En tránsito', 3, 3, 5),
('2026-11-15', 'Paquetexpress', 'PQX-MX-551908', 'En preparación', 1, 4, 1),
('2026-11-16', 'UPS', 'UPS-MX-339012', 'Entregado', 2, 5, 9),
('2026-11-17', 'Estafeta', 'EST-MX-882499', 'En tránsito', 1, 6, 4),
('2026-11-18', 'FedEx', 'FDX-MX-771589', 'En preparación', 2, 7, 8),
('2026-11-19', 'DHL Express', 'DHL-MX-900412', 'Enviado', 1, 8, 3),
('2026-11-20', 'UPS', 'UPS-MX-339455', 'En preparación', 2, 9, 10);
