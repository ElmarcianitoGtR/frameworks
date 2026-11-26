(() => {
  const STORAGE_KEY = "tienda_ropa_session";

  const state = {
    token: null,
    role: null,
    email: null,
    colecciones: [],
    prendas: [],
    envios: [],
  };

  const el = {
    statusBanner: document.getElementById("status-banner"),
    currentRole: document.getElementById("current-role"),
    currentEmail: document.getElementById("current-email"),
    currentToken: document.getElementById("current-token"),
    tbodyColecciones: document.getElementById("tbody-colecciones"),
    tbodyPrendas: document.getElementById("tbody-prendas"),
    tbodyEnvios: document.getElementById("tbody-envios"),
    enviosScope: document.getElementById("envios-scope"),
    selectPrenda: document.getElementById("input-prenda-id"),
    inputFecha: document.getElementById("input-fecha"),
    inputGuia: document.getElementById("input-guia"),
    inputClienteId: document.getElementById("input-cliente-id"),
    formEnvio: document.getElementById("form-nuevo-envio"),
    btnVendedor: document.getElementById("btn-login-vendedor"),
    btnCliente: document.getElementById("btn-login-cliente"),
    btnLogout: document.getElementById("btn-logout"),
    btnRefreshCatalogo: document.getElementById("btn-refresh-catalogo"),
    btnRefreshEnvios: document.getElementById("btn-refresh-envios"),
  };

  function showStatus(message, isError = false) {
    el.statusBanner.textContent = message;
    el.statusBanner.classList.toggle("error", isError);
    el.statusBanner.hidden = false;
  }

  function clearStatus() {
    el.statusBanner.hidden = true;
    el.statusBanner.textContent = "";
  }

  function parseJwtPayload(token) {
    try {
      const parts = token.split(".");
      if (parts.length < 2) return null;
      const base64 = parts[1].replace(/-/g, "+").replace(/_/g, "/");
      const json = decodeURIComponent(
        atob(base64)
          .split("")
          .map((c) => "%" + ("00" + c.charCodeAt(0).toString(16)).slice(-2))
          .join("")
      );
      return JSON.parse(json);
    } catch {
      return null;
    }
  }

  function saveSession() {
    if (state.token) {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({ token: state.token, role: state.role, email: state.email })
      );
    } else {
      localStorage.removeItem(STORAGE_KEY);
    }
  }

  function loadSession() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return;
      const parsed = JSON.parse(raw);
      if (parsed && parsed.token) {
        state.token = parsed.token;
        state.role = parsed.role || null;
        state.email = parsed.email || null;
      }
    } catch {
      localStorage.removeItem(STORAGE_KEY);
    }
  }

  function renderSession() {
    el.currentRole.textContent = state.role || "Sin sesión";
    el.currentEmail.textContent = state.email ? `(${state.email})` : "";
    el.currentToken.textContent = state.token || "(Sin token)";

    if (!state.token) {
      el.enviosScope.textContent = "Inicia sesión para consultar los envíos.";
    } else if ((state.role || "").toLowerCase().includes("vendedor")) {
      el.enviosScope.textContent = "Vista Vendedor: mostrando todos los envíos.";
    } else {
      el.enviosScope.textContent = "Vista Cliente: mostrando únicamente tus envíos.";
    }
  }

  async function apiFetch(url, options = {}) {
    const headers = new Headers(options.headers || {});
    if (state.token && !headers.has("Authorization")) {
      headers.set("Authorization", `Bearer ${state.token}`);
    }
    if (options.body && !headers.has("Content-Type")) {
      headers.set("Content-Type", "application/json");
    }

    const response = await fetch(url, { ...options, headers });
    const text = await response.text();
    let data = null;
    try {
      data = text ? JSON.parse(text) : null;
    } catch {
      data = text;
    }

    if (!response.ok) {
      const msg =
        (data && typeof data === "object" && (data.error || data.message || data.detail)) ||
        (typeof data === "string" && data) ||
        `Error HTTP ${response.status}`;
      const error = new Error(msg);
      error.status = response.status;
      throw error;
    }

    return data;
  }

  function clearElement(node) {
    while (node.firstChild) {
      node.removeChild(node.firstChild);
    }
  }

  function createEmptyRow(colspan, text) {
    const tr = document.createElement("tr");
    const td = document.createElement("td");
    td.colSpan = colspan;
    td.textContent = text;
    tr.appendChild(td);
    return tr;
  }

  function appendCell(tr, value) {
    const td = document.createElement("td");
    td.textContent = value === null || value === undefined ? "" : String(value);
    tr.appendChild(td);
  }

  function renderColecciones() {
    clearElement(el.tbodyColecciones);
    if (!state.colecciones.length) {
      el.tbodyColecciones.appendChild(createEmptyRow(4, "No hay colecciones disponibles."));
      return;
    }
    for (const c of state.colecciones) {
      const tr = document.createElement("tr");
      appendCell(tr, c.id);
      appendCell(tr, c.nombre);
      appendCell(tr, c.anio);
      appendCell(tr, c.descripcion);
      el.tbodyColecciones.appendChild(tr);
    }
  }

  function renderPrendas() {
    clearElement(el.tbodyPrendas);
    const colMap = new Map(state.colecciones.map((c) => [c.id, c.nombre]));

    const currentSelected = el.selectPrenda.value;
    clearElement(el.selectPrenda);
    const defaultOpt = document.createElement("option");
    defaultOpt.value = "";
    defaultOpt.textContent = "Selecciona una prenda...";
    el.selectPrenda.appendChild(defaultOpt);

    if (!state.prendas.length) {
      el.tbodyPrendas.appendChild(createEmptyRow(6, "No hay prendas disponibles."));
      return;
    }

    for (const p of state.prendas) {
      const tr = document.createElement("tr");
      appendCell(tr, p.id);
      appendCell(tr, p.nombre);
      appendCell(tr, p.talla);
      appendCell(tr, `$${Number(p.precio).toFixed(2)}`);
      appendCell(tr, p.stock);
      const colLabel = colMap.get(p.coleccion_id)
        ? `${colMap.get(p.coleccion_id)} (#${p.coleccion_id})`
        : p.coleccion_id;
      appendCell(tr, colLabel);
      el.tbodyPrendas.appendChild(tr);

      const opt = document.createElement("option");
      opt.value = String(p.id);
      opt.textContent = `#${p.id} - ${p.nombre} (${p.talla}) - $${Number(p.precio).toFixed(2)} [stock: ${p.stock}]`;
      el.selectPrenda.appendChild(opt);
    }

    if (currentSelected) {
      el.selectPrenda.value = currentSelected;
    } else if (state.prendas.length > 0) {
      el.selectPrenda.value = String(state.prendas[0].id);
    }
  }

  function renderEnvios() {
    clearElement(el.tbodyEnvios);
    if (!state.token) {
      el.tbodyEnvios.appendChild(
        createEmptyRow(8, "Inicia sesión como Vendedor o Cliente para ver los envíos.")
      );
      return;
    }
    if (!state.envios.length) {
      el.tbodyEnvios.appendChild(createEmptyRow(8, "No se encontraron envíos."));
      return;
    }
    for (const e of state.envios) {
      const tr = document.createElement("tr");
      appendCell(tr, e.id);
      appendCell(tr, e.fecha_despacho);
      appendCell(tr, e.paqueteria);
      appendCell(tr, e.numero_guia);
      appendCell(tr, e.estatus);
      appendCell(tr, e.cantidad);
      appendCell(tr, e.cliente_id);
      appendCell(tr, e.prenda_id);
      el.tbodyEnvios.appendChild(tr);
    }
  }

  async function loadCatalogo() {
    try {
      const [colecciones, prendas] = await Promise.all([
        apiFetch("/api/colecciones"),
        apiFetch("/api/prendas"),
      ]);
      state.colecciones = Array.isArray(colecciones) ? colecciones : [];
      state.prendas = Array.isArray(prendas) ? prendas : [];
      renderColecciones();
      renderPrendas();
    } catch (err) {
      showStatus(`Error al cargar catálogo: ${err.message}`, true);
    }
  }

  async function loadEnvios() {
    if (!state.token) {
      state.envios = [];
      renderEnvios();
      return;
    }
    try {
      const envios = await apiFetch("/api/envios");
      state.envios = Array.isArray(envios) ? envios : [];
      renderEnvios();
    } catch (err) {
      state.envios = [];
      renderEnvios();
      showStatus(`Error al cargar envíos: ${err.message}`, true);
    }
  }

  async function loginAs(correo, fallbackRole) {
    clearStatus();
    const password = correo.includes("vendedor") ? "vendedor123" : "cliente123";
    const endpoints = ["/api/auth/login", "/api/login"];
    const bodies = [
      { correo, password },
      { email: correo, password }
    ];
    let lastError = null;

    for (const endpoint of endpoints) {
      for (const body of bodies) {
        try {
          const res = await fetch(endpoint, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(body),
          });
          if (res.status === 404 || res.status === 405) {
            lastError = new Error(`Endpoint ${endpoint} no disponible (${res.status})`);
            break;
          }
          const data = await res.json().catch(() => null);
          if (!res.ok) {
            lastError = new Error(
              (data && (data.error || data.message)) || `Error HTTP ${res.status}`
            );
            continue;
          }

          const token = data && (data.token || data.access_token || data.jwt);
          if (!token) {
            throw new Error("La respuesta de login no incluyó un token JWT.");
          }

          const payload = parseJwtPayload(token);
          const role =
            (data && (data.rol || data.role)) ||
            (payload && (payload.rol || payload.role)) ||
            fallbackRole;

          state.token = token;
          state.role = role;
          state.email = correo;
          if (fallbackRole === "Cliente") {
            el.inputClienteId.value = String((payload && (payload.cliente_id || payload.sub)) || 1);
          }

          saveSession();
          renderSession();
          showStatus(`Sesión iniciada como ${role} (${correo}).`);
          await Promise.all([loadCatalogo(), loadEnvios()]);
          return;
        } catch (err) {
          lastError = err;
        }
      }
    }

    showStatus(
      `No se pudo iniciar sesión: ${lastError ? lastError.message : "Error desconocido"}`,
      true
    );
  }

  function logout() {
    state.token = null;
    state.role = null;
    state.email = null;
    state.envios = [];
    saveSession();
    renderSession();
    renderEnvios();
    showStatus("Sesión cerrada.");
  }

  function setDefaultFormValues() {
    const today = new Date().toISOString().slice(0, 10);
    el.inputFecha.value = today;
    el.inputGuia.value = `GUIA-${Math.floor(100000 + Math.random() * 900000)}`;
  }

  async function handleCreateEnvio(event) {
    event.preventDefault();
    clearStatus();

    if (!state.token) {
      showStatus("Debes iniciar sesión antes de crear un envío.", true);
      return;
    }

    const formData = new FormData(el.formEnvio);
    const payload = {
      fecha_despacho: String(formData.get("fecha_despacho") || ""),
      paqueteria: String(formData.get("paqueteria") || "").trim(),
      numero_guia: String(formData.get("numero_guia") || "").trim(),
      estatus: String(formData.get("estatus") || "").trim(),
      cantidad: Number(formData.get("cantidad")),
      cliente_id: Number(formData.get("cliente_id")),
      prenda_id: Number(formData.get("prenda_id")),
    };

    try {
      await apiFetch("/api/envios", {
        method: "POST",
        body: JSON.stringify(payload),
      });
      showStatus("Envío creado correctamente.");
      el.inputGuia.value = `GUIA-${Math.floor(100000 + Math.random() * 900000)}`;
      await Promise.all([loadEnvios(), loadCatalogo()]);
    } catch (err) {
      showStatus(`Error al crear envío: ${err.message}`, true);
    }
  }

  el.btnVendedor.addEventListener("click", () => loginAs("vendedor@tienda.mx", "Vendedor"));
  el.btnCliente.addEventListener("click", () => loginAs("sofia.mendoza@correo.mx", "Cliente"));
  el.btnLogout.addEventListener("click", logout);
  el.btnRefreshCatalogo.addEventListener("click", loadCatalogo);
  el.btnRefreshEnvios.addEventListener("click", loadEnvios);
  el.formEnvio.addEventListener("submit", handleCreateEnvio);

  loadSession();
  setDefaultFormValues();
  renderSession();
  loadCatalogo();
  loadEnvios();
})();
