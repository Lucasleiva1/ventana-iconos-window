# Validación de Desktop Organizer

Estado actual: `v0.2.0` implementada, compilada e instalada para prueba.
Publicación bloqueada por el pendiente conocido del Tray.

Este documento registra pruebas reales y límites del entorno usado para cerrar la versión.

## Pruebas automatizadas

- TypeScript: `npm run check`.
- Frontend de producción: `npm run build`.
- Rust: `cargo test --manifest-path src-tauri\Cargo.toml`.
- Lints Rust: `cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings`.
- Build Tauri de producción y bundle NSIS firmado.

La suite Rust contiene 30 pruebas. Cubre, entre otros casos: conflictos sin
sobrescritura, origen conservado ante copia fallida, guardado atómico, backups,
save corrupto, metadata corrupta, Unicode, 1001 elementos no recursivos,
subcajones, reordenamiento, restauración, caché limitada, migración del Dock de
Parte 5, 300 accesos y límites de ancho.

## Parte 5 — Dock

Verificado con automatización real de mouse (arrastre OLE genuino desde el
Explorador de Windows) contra el binario de release, en 1024x768 al 100 % de
escala:

| Prueba | Resultado |
| --- | --- |
| 1 — Al iniciar sólo se ve el tirador | OK. Tirador visible en (483,724) 58x14; ventana del Dock creada pero oculta. |
| 2 — Hover sobre el tirador | OK. Tras 2 s de hover el Dock sigue cerrado. |
| 3 — Clic abre y segundo clic cierra | OK. |
| 4/5 — Arrastrar un `.lnk` desde el Explorador | OK. Se agrega como acceso; el `.lnk` original permanece en su carpeta. |
| 8 — Un clic sobre el icono | OK. Abre el programa asociado. |
| 9 — Auto-ocultar después de abrir | OK. El Dock se esconde solo. |
| 17 — Persistencia tras reiniciar | OK. El acceso y su icono nativo se conservan. |
| 18 — Cajones intactos | OK. Los cajones se restauran y siguen recibiendo arrastres. |
| Migración de esquema | OK. Un save `schemaVersion` 4 de `v0.1.0` se migró a 5 conservando cajones y preferencias. |

### Hallazgo importante durante la validación

La ventana del Dock **no recibía** los archivos arrastrados desde Windows
mientras se creaba con `focused(false)`. Con esa bandera, Windows no le entrega
los eventos de arrastre a la ventana, aunque el resto del webview funcione con
normalidad. Se comprobó con una prueba de control: el mismo arrastre simulado
entraba en un cajón y no producía ningún evento en el Dock. Quitando
`focused(false)` el arrastre funciona. Queda documentado en
`src-tauri/src/dock_service.rs` para que no se reintroduzca.

Del mismo modo, sin `maximizable(false)`, `minimizable(false)` y
`closable(false)` más un `min_inner_size` explícito, Windows fuerza su tamaño
mínimo de ventana (136x39) y el tirador nace deformado.

### Pendiente de validación por falta de hardware

Escalas DPI de 125 % y 150 %, varios monitores y desconexión en caliente del
monitor del Dock. La lógica cae al monitor principal por diseño y se puede
forzar desde **Administrador → DOCK → Reubicar**.

## Parte 6 — Dock avanzado

| Prueba | Resultado |
| --- | --- |
| TypeScript | OK: `npm run check`. |
| Frontend | OK: build Vite de producción. |
| Rust | OK: 30/30 pruebas. |
| Clippy | OK: todos los targets con `-D warnings`. |
| Build Tauri | OK: perfil release optimizado y bundle NSIS firmado. |
| Arranque normal | OK: Administrador 936×788 visible, dos Cajones restaurados, Dock oculto y tirador 58×14 visible. |
| Single-instance | OK: diez lanzamientos mantienen un proceso. |
| Actualización instalada | OK: instalación 0.1.0 actualizada a 0.2.0, versión de archivo/producto 0.2.0. |
| Datos de Cajones | OK: identidades, cantidad de archivos y bytes físicos sin cambios. |
| Dock existente | OK: prueba automatizada conserva IDs y orden al migrar esquema 5 → 6. |
| Rendimiento en reposo | 0,0000 s de CPU durante una muestra de 5,01 s; working set aproximado 31,49 MiB. |
| Updater | OK: instalador y `.sig`, `latest.json` sin BOM, firma de 432 caracteres y SHA-256. |

La instalación real comenzó con una entrada de Windows 0.1.0, pero el save ya
había sido migrado al esquema 6 por la sesión de desarrollo previa. Por eso la
conservación física se verificó sobre datos reales y la migración pura se
verificó de forma automatizada con fixtures de esquemas 1 y 5.

## Limitaciones de validación

- No se simula un corte eléctrico real durante una copia. Se prueban fallos controlados manteniendo el origen.
- El comportamiento entre volúmenes usa copia verificada por tamaño y SHA-256, pero no muestra una barra de progreso detallada.
- Se probó Windows a 100 % de escala. Windows 10, otros equipos físicos,
  múltiples monitores y escalas DPI 125 %/150 % no estaban disponibles y
  requieren validación adicional real. El cálculo usa `work_area`,
  `scale_factor` y fallback al monitor principal.
- El instalador no posee certificado comercial de firma de código; Windows SmartScreen puede advertirlo.
- El menú por elemento del Dock es en línea dentro de la barra, no un menú nativo del sistema: la ventana del Dock mide pocos píxeles de alto y un menú nativo tomaría el foco.
- La automatización nativa de UI no estuvo disponible durante el cierre, por lo
  que shortcut, menús de reparación, separadores y controles visuales se
  validaron por compilación, lógica y pruebas de persistencia, no mediante una
  secuencia automatizada de clics.
- El shortcut global queda desactivado por defecto y reporta una colisión al
  intentar registrar una combinación ocupada.
- La clave privada disponible no coincidía con la clave pública de v0.1.0. Se
  generó una nueva fuera del repositorio; actualizar desde v0.1.0 requiere una
  instalación manual única.
