# Validación de Desktop Organizer

Estado actual: Parte 9 preparada como `0.4.0`. La Release `v1.0.0` permanece reservada para la Parte 10.

Este documento registra pruebas reales y límites del entorno usado para cerrar la versión.

## Pruebas automatizadas

- TypeScript: `npm run check`.
- Frontend de producción: `npm run build`.
- Rust: `cargo test --manifest-path src-tauri\Cargo.toml`.
- Lints Rust: `cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings`.
- Build Tauri de producción sin bundle. El bundle NSIS firmado se reserva para
  un entorno de Release autorizado que posea la clave privada fuera del repositorio.

La suite Rust contiene 46 pruebas. Cubre, entre otros casos: conflictos sin
sobrescritura, origen conservado ante copia fallida, guardado atómico, backups,
save corrupto, metadata corrupta, Unicode, 1001 elementos no recursivos,
subcajones, reordenamiento, restauración, caché limitada, migración del Dock de
Parte 5, migraciones encadenadas de todos los schemas, recuperación parcial de
módulos corruptos, logs rotativos, 300 accesos y límites de ancho.

## Parte 10 — QA final y cierre v1.0.0

Equipo de prueba: **Windows 10 Pro 22H2, build 19045, 64 bits. Un solo monitor
de 1024x768 a escala 100%. 8 núcleos.**

### Verificación técnica

| Comprobación | Resultado |
| --- | --- |
| TypeScript (`npm run check`) | Pasa |
| Build de frontend | Pasa |
| `cargo check --all-targets` | Pasa |
| `cargo test` | **51 pruebas, 0 fallos** |
| `cargo clippy -- -D warnings` | Pasa, sin advertencias |
| `npm audit` | 0 vulnerabilidades |
| Dependencias sin uso | Ninguna; se quitó la feature `tray-icon` que quedó libre |
| `unwrap()`/`expect()` en producción | **0** — las 148 apariciones están dentro de `#[cfg(test)]` |
| `console.log` de desarrollo | 0; sólo cuatro `console.error` de manejo de errores |
| TODO/FIXME/HACK pendientes | Ninguno |

### Seguridad

| Comprobación | Resultado |
| --- | --- |
| Claves privadas, tokens o contraseñas en archivos versionados | **Ninguna** |
| Lo mismo en todo el historial de git | **Ninguna**; nunca se commiteó un `.key`, `.pem` ni `.env` |
| Rutas personales o nombres propios en el código | Ninguna |
| Llamadas de red en el frontend | **Ninguna** |
| Llamadas de red en Rust | **Ninguna** fuera del plugin oficial del updater |
| Telemetría o analytics | No existe |
| CSP | `default-src 'self'` con `connect-src` limitado a IPC: la interfaz no puede salir a Internet |
| Permisos de Tauri | `core:default` para todas las ventanas; el updater sólo para el Administrador |
| Path traversal | `safe_backup_path` rechaza separadores; la importación valida `is_within` contra la raíz de Cajones |

### Movimiento de archivos, con archivos reales

| Escenario | Resultado |
| --- | --- |
| Mismo volumen | `fs::rename` directo, sin copiar |
| **Entre volúmenes distintos (C: ↔ D:)** | Copia, verifica y **recién entonces** borra el origen. Verificado con una carpeta anidada de 512 KiB |
| Fallo a mitad de la copia | El origen queda intacto y el error lo dice explícitamente |
| Conflicto de nombre, mismo volumen | No sobrescribe ni fusiona; origen y destino quedan como estaban |
| Conflicto de nombre, entre volúmenes | Igual: ambos lados intactos |
| Restos de staging tras una copia | Ninguno |
| Nombres `Diseño`, `Música 2026`, `Ñandú`, `Proyecto 日本語`, espacios múltiples | Ciclo completo Escritorio → Cajón → Escritorio sin pérdida ni duplicación |

### Icono del área de notificación — el bug que bloqueaba la Release

Medición hecha leyendo la barra del Explorador e identificando **qué proceso
puso cada icono**, que es la única forma de distinguir un icono vivo de un
fantasma.

| Momento | Iconos de Desktop Organizer | Fantasmas |
| --- | --- | --- |
| Tras instalar la v1.0.0, 20 ejecuciones seguidas y **13 muertes bruscas del proceso** | **1** | **0** |
| Tras reiniciar el Explorador de Windows | **1** | **0** |

El proceso sobrevivió al reinicio del Explorador con el mismo PID y volvió a
registrar su icono solo. Trece cierres a la fuerza —el escenario exacto que
generaba los fantasmas— no dejaron ninguno.

### Rendimiento medido

| Medición | Valor real |
| --- | --- |
| Arranque hasta ventana operativa | 565 ms en frío; 101 y 106 ms después. **Promedio 257 ms** |
| CPU en reposo, aplicación quieta | **0,013 %** sobre 8 núcleos (31 ms de CPU en 30 s) |
| CPU de WebView2 en reposo | 0 % |
| **RAM privada real** | **169,4 MB** en 11 procesos |
| RAM en `WorkingSet` | 635,6 MB — cifra inflada, cuenta la memoria compartida una vez por proceso |
| Fuga de memoria, 8 ciclos de matar y reabrir | 24,6 → 24,5 MB. **Delta −0,1 MB: no hay fuga** |

La huella de memoria está dominada por WebView2: hay una ventana web por cada
Administrador, Cajón, Panel, Dock y tirador. Es el costo de la arquitectura
elegida, no una fuga.

### Instalación y datos

| Comprobación | Resultado |
| --- | --- |
| Single instance: 20 ejecuciones seguidas | **1 proceso** |
| Actualizar 0.4.0 → 1.0.0 | Correcta |
| Desinstalar: ¿borra `Documentos\Desktop Organizer`? | **No.** SHA-256 del save idéntico antes y después |
| Desinstalar + reinstalar | Cajones, Dock y los 9 backups intactos; save idéntico |
| Accesos directos del instalador | 1 en el Menú Inicio, sin duplicados |
| Metadata de Windows | ProductName, versión, editor y copyright completos, sin placeholders |
| Entradas huérfanas de inicio automático | Ninguna |

### Cómo reproducir exactamente este build

```text
Versión          : 1.0.0
Fecha del build  : 2026-09-06 (UTC)
Sistema          : Windows 10 Pro 22H2, build 19045, x64
rustc            : 1.96.0 (ac68faa20 2026-05-25)
cargo            : 1.96.0 (30a34c682 2026-05-25)
node             : v24.17.0
npm              : 11.13.0
tauri-cli        : 2.11.4
Comando          : npm run tauri -- build --ci --bundles nsis
                   (precedido de cargo clean --release, con la clave de firma
                   del updater en TAURI_SIGNING_PRIVATE_KEY)
Instalador       : Desktop-Organizer-v1.0.0-Setup.exe
Tamaño           : 3.758.299 bytes
SHA-256          : 125f916ab79fc244cdec2d7d3effdcb0af7494ab23f4d843a20b4759e6b23782
```

El commit exacto es el que lleva el tag `app-v1.0.0`.

### Pendiente de validación por falta de hardware

El equipo tiene un solo monitor a 100% con Windows 10. **No se probaron
físicamente**: multimonitor, monitor que se desconecta, monitores con DPI
distinto, escalado a 125%/150% y Windows 11. Están detallados en
`KNOWN_ISSUES.md`.

Tampoco se automatizaron las acciones que exigen arrastrar con el mouse dentro
de una ventana web —soltar un archivo del Escritorio en un Cajón, en el Dock o
en un Panel—. La lógica que ejecutan esos gestos sí está cubierta por las
pruebas de `item_repository`, `dock_repository` y `storage_service` con archivos
reales en disco.

## Parte 9 — configuración y recuperación integral

Resultados ejecutados el 2026-09-06 sobre el repositorio limpio recibido en `main` más los cambios locales de Parte 9:

| Prueba | Resultado |
| --- | --- |
| TypeScript | OK: `npm run check`. |
| Frontend de producción | OK: 49 módulos; build Vite en 366 ms y repetición dentro del build nativo en 213 ms. |
| Rust | OK: 46/46 pruebas, 0 fallos. |
| Migración encadenada | OK: fixtures de todos los schemas 1–8 llegan directamente al schema 9. |
| Configuración parcialmente corrupta | OK: un Dock inválido no descarta Cajones ni Paneles válidos. |
| Backup/save corrupto | OK: master atómico, copia válida recuperable y archivo corrupto preservado. |
| Logs | OK: rotación limitada y archivo ajeno intacto. |
| Filesystem Cajones/Dock | OK por regresión automatizada: movimientos verificados, conflictos sin sobrescritura y recuperación no recursiva. |
| Paneles | OK por regresión automatizada y TypeScript: referencias puras, persistencia y layout existentes sin fallos. |
| Clippy | OK: todos los targets con `-D warnings`. |
| Tauri release | OK: binario e instalador NSIS `0.4.0` compilados; firma updater de 432 caracteres y metadatos internos 0.4.0 verificados. |
| Assets de Release | OK: instalador `Desktop-Organizer-v0.4.0-Setup.exe`, `.sig`, `latest.json`, `latest-v0.4.0.json` y `SHA256SUMS-v0.4.0.txt`, todos versionados o acompañados por su alias técnico requerido. |
| Single-instance | Verificación parcial real: lanzar el binario de desarrollo con la instalación 0.3.0 abierta activó la instancia existente en vez de crear un segundo proceso funcional. |
| Revisión visual 0.4.0 | Pendiente de la instalación manual solicitada por el usuario. |
| CPU/RAM/disco y arranque 0.4.0 | No medidos todavía; no se inventan valores. El único muestreo nativo periódico, necesario para prioridad fullscreen del Dock, bajó de 4 Hz a 1 Hz. |

La clave privada del updater se mantiene fuera del árbol. La instalación manual de `0.4.0` realiza la transición al nuevo par de firma; desde esa versión, la verificación criptográfica de futuras actualizaciones queda a cargo del plugin oficial de Tauri con la clave pública configurada.

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
| Rust | OK: 40/40 pruebas. |
| Carpeta física del Dock | OK por pruebas automatizadas: reconstrucción desde disco, metadata y separadores persistentes. Validación visual/manual pendiente. |
| Prioridad tipo barra de tareas | OK por pruebas automatizadas: diferencia fullscreen de una ventana maximizada que conserva la barra de tareas. Validación visual/manual pendiente. |
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

## Parte 7 — Paneles organizadores

Verificado con automatización real de mouse contra el binario de release, en
1024x768 al 100 % de escala:

| Prueba | Resultado |
| --- | --- |
| 88 — Crear panel desde el Administrador | OK. Aparece como ventana independiente de 480x320. |
| 89 — Movimiento y persistencia de posición | OK. Tras reiniciar vuelve a la misma posición y tamaño. |
| 90 — Resize grande | OK. Con 6 accesos en 620x460 los iconos llegan a 64 px y muestran el nombre completo. |
| 91 — Resize medio | OK. En 360x260 bajan a tamaño intermedio y reorganizan columnas. |
| 92 — Resize pequeño | OK. En 210x170 llegan al mínimo de 24 px y quedan sólo iconos. |
| 93 — Scroll después del mínimo | OK. Con 61 accesos en 230x190 aparece scroll vertical y los iconos no siguen achicándose. |
| 94 — Muchos elementos | OK con 61 accesos: grilla de 11 columnas, resize fluido y scroll. |
| 95 — Drag desde Windows | OK. Archivos y carpetas se agregan como referencias. |
| 96 — Archivo del Escritorio | **OK.** El archivo se agregó al Panel y siguió existiendo en el Escritorio. |
| 97 — Abrir con doble clic | OK. Abre con la aplicación predeterminada de Windows. |
| 99 — Bloquear panel | OK. Windows quita el borde redimensionable y la cabecera deja de moverlo. |
| 102 — Single instance | OK. Ejecutar el binario otra vez sólo trae el Administrador al frente. |
| 103 — Cajones | OK. Se restauran y siguen funcionando. |
| 104 — Dock | OK. El tirador sigue abriendo y cerrando el Dock. |
| Migración | OK. Un save `schemaVersion` 6 de `v0.2.1` pasó a 7 conservando cajones, Dock y preferencias. |

### Pendiente de validación por falta de hardware

Escalas DPI de 125 % y 150 %, varios monitores y desconexión en caliente del
monitor de un Panel. La lógica recalcula límites y devuelve el Panel al monitor
principal por diseño.

## Parte 8 — Paneles avanzados y cierre v0.3.0

Verificado con automatización real contra el binario de release, en 1024x768 al
100 % de escala, sobre Windows 10 Pro 19045:

| Prueba | Resultado |
| --- | --- |
| 71 — Imantado a bordes | OK. Soltado a 9 px del borde izquierdo, se alineó en 12 (margen de seguridad). |
| 72 — Imantado entre Paneles | OK. Soltado a 4 px del borde de otro Panel, alineó ambos bordes izquierdos. |
| 74 — Auto vs manual | OK. En automático el tamaño de icono sigue al Panel; en manual se respeta y aparece scroll. |
| 76 — Bloqueo de posición | OK. Windows quita el borde redimensionable y la cabecera deja de mover el Panel. |
| 77 — Bloqueo de contenido | OK por diseño y cubierto en el backend: reordenar, renombrar y quitar devuelven error; abrir sigue funcionando. |
| 82 — Persistencia | OK. Tras reiniciar, cada Panel vuelve a su posición, tamaño, modo y densidad. |
| 83 — Export/import | OK. Cubierto por prueba automatizada: importar reconstruye los Paneles y no toca los Cajones ni ningún archivo. |
| 87 — Single instance | OK. Ejecutar el binario otra vez deja un solo proceso. |
| 96 — Actualizar v0.2.1 a v0.3.0 | **OK.** Instalación silenciosa sobre la v0.2.1: registro actualizado a 0.3.0, cajones, Dock, preferencias, carpetas físicas y backups intactos. |
| 107 — Instalador | OK. Instalado y ejecutado desde `%LOCALAPPDATA%\Desktop Organizer`. |
| Dock en pantalla completa | **OK.** Con una aplicación a pantalla completa el tirador se oculta como la barra de tareas, y vuelve al salir. Verificado con el reproductor real del usuario. |

### Bugs de la Parte 7 encontrados y corregidos

1. La grilla medía su área con `clientWidth`, que incluye el padding, así que
   calculaba 20 px más de los reales en cada eje. Ahora usa la caja de contenido
   que entrega `ResizeObserver`.
2. Importar configuración descartaba los Paneles por completo.
3. Varios Paneles recuperados de un monitor desconectado quedaban apilados
   exactamente en la misma esquina.

### Pendiente de validación por falta de hardware

Escalas DPI de 125 % y 150 %, varios monitores y desconexión en caliente. La
lógica recalcula límites y devuelve los Paneles al monitor principal por diseño,
escalonándolos para que ninguno quede tapado.

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
