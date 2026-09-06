# RESUELTO — El icono de la bandeja se duplicaba

**Estado: corregido y publicado en la v1.0.0.** Este documento conserva el
diagnóstico original porque explica *por qué* el icono se registra como se
registra; el código quedó implementado en `src-tauri/src/tray_service.rs`.

- Fecha del diagnóstico: 5 de septiembre de 2026.
- Fecha de la corrección: 6 de septiembre de 2026 (Parte 10).

## Qué se implementó, respecto del plan de abajo

Se aplicó la solución acordada de la sección 4, con una diferencia que bajó el
riesgo: no hizo falta reconstruir la lógica del menú. Las doce acciones son
exactamente las mismas funciones de antes; sólo cambió quién dibuja el menú
(`TrackPopupMenu` en lugar de la librería de Tauri).

Además del plan original se agregó:

- volver a registrar el icono si se reinicia el Explorador de Windows
  (mensaje de difusión `TaskbarCreated`), que antes tampoco estaba cubierto;
- un camino de respaldo: si Windows rechaza el GUID —pasa cuando ese GUID quedó
  asociado a un ejecutable en otra ruta, típico al alternar entre la compilación
  de desarrollo y la instalada— el icono se registra igual del modo clásico en
  lugar de desaparecer.

Como el icono ya no lo crea Tauri, se quitó también la feature `tray-icon` de la
dependencia, que quedó sin uso.

---

## 1. El síntoma

En el área de notificación de Windows (el panel de "Mostrar iconos ocultos")
aparecen **varios iconos idénticos** de Desktop Organizer, uno al lado del otro.
Se van acumulando con el uso.

No es un problema estético menor: el usuario espera que la aplicación se
comporte como cualquier otra de la barra (Claude Code, MEGAsync, NVIDIA App),
que siempre muestran un icono y nunca se duplican.

## 2. Verificación hecha

| Comprobación | Resultado |
| --- | --- |
| Iconos visibles en el panel | 10 iconos idénticos |
| Procesos de Desktop Organizer corriendo | **1** |
| Al pasar el mouse por encima de cada uno | 9 desaparecen; queda 1 |
| Guarda contra duplicados en el código | Existe y funciona (`tray_service.rs`) |
| Single instance | Activo, no puede haber dos procesos |

Conclusión de la verificación: la aplicación **no crea** iconos de más. Los
repetidos son restos ("fantasmas") de ejecuciones anteriores que Windows sigue
dibujando.

## 3. La causa real

Un icono de bandeja no tiene nombre propio. Windows lo identifica con dos datos
que le entrega el programa:

```text
ventana invisible (HWND)  +  número (uID)
```

La librería que usa Tauri (`tray-icon` 0.24.2) **crea una ventana invisible
nueva en cada arranque** de la aplicación. Verificado en su código fuente:

```text
tray-icon-0.24.2/src/platform_impl/windows/mod.rs
  - línea ~595: Shell_NotifyIconW(NIM_ADD, ...) con hWnd + uID
  - línea ~612: remove_tray_icon(hwnd, id) usa los mismos dos datos
```

Como el identificador cambia en cada ejecución, **Windows no tiene forma de
saber que el icono nuevo es el mismo de antes**. Lo trata como el icono de otro
programa distinto y lo dibuja al lado del anterior en vez de reemplazarlo.

Es el equivalente a que cada vez que abrís un programa te creara un acceso
directo con un nombre distinto en el Escritorio: nunca se pisan, se acumulan.

### Por qué queda el resto de la ejecución anterior

Windows borra un icono de bandeja únicamente cuando el programa se lo pide al
cerrarse (`Shell_NotifyIcon` con `NIM_DELETE`). Si el proceso muere sin poder
despedirse —se cuelga, lo matan desde el Administrador de tareas, o se lo mata
para recompilar durante el desarrollo—, el dibujo queda ahí hasta que alguien
pasa el mouse por encima del panel.

Nota: el cierre normal por **Tray → Salir** sí borra el icono. Está verificado
leyendo el código de Tauri 2.11.5: `AppHandle::exit()` llama a
`cleanup_before_exit()`, que vacía la lista de iconos, y el `Drop` de
`tray-icon` ejecuta `Shell_NotifyIconW(NIM_DELETE, ...)` de forma inmediata.

### Es un problema conocido de Tauri

Está reportado en el foro oficial con estas palabras: al abrir y cerrar la
aplicación varias veces, quedan varios iconos en la bandeja.

## 4. La solución acordada

**Darle al icono un identificador fijo y permanente, propio de Desktop
Organizer.**

Windows tiene exactamente ese mecanismo: el campo `guidItem` con la bandera
`NIF_GUID` de la estructura `NOTIFYICONDATAW`. La documentación de Microsoft lo
describe como *"el método recomendado para identificar el icono"*: reemplaza al
par ventana+número por un identificador único que no cambia nunca.

**La librería de Tauri no lo implementa.** Se verificó: no hay ninguna
referencia a `NIF_GUID` ni a `guidItem` en todo el código de `tray-icon` 0.24.2.

### Qué hay que hacer, en concreto

1. Dejar de crear el icono con `TrayIconBuilder` de Tauri.
2. Crearlo directamente con la API de Windows (`Shell_NotifyIconW`), usando un
   GUID fijo generado una sola vez y escrito como constante en el código.
3. **Al arrancar**, antes de registrar el icono, ejecutar `NIM_DELETE` con ese
   GUID. Eso borra cualquier resto de una ejecución anterior, incluso si la
   aplicación murió de golpe. Recién después, `NIM_ADD`.
4. Rehacer el menú del tray con el menú nativo de Windows (`TrackPopupMenu`),
   porque va atado al icono. Debe conservar **todas** las entradas actuales:

   ```text
   Nuevo cajón
   Abrir administrador
   ---------------------------
   Mostrar todos los cajones
   Ocultar todos los cajones
   Abrir carpeta Cajones
   ---------------------------
   Mostrar Dock
   Ocultar Dock
   Configuración
   ---------------------------
   Salir
   ```

5. Conservar el comportamiento actual del clic izquierdo sobre el icono: abre el
   Administrador.
6. Agregado chico que va de paso: borrar el icono también cuando Windows cierra
   sesión o se apaga (`WM_QUERYENDSESSION` / `WM_ENDSESSION`), que hoy no está
   cubierto.

### Resultado esperado

Es **imposible** que existan dos iconos de Desktop Organizer. No se "limpian":
directamente no pueden coexistir, porque comparten identificador y el segundo
reemplaza al primero.

### Costo y riesgo

- Alrededor de 250 líneas de código nativo de Windows.
- Riesgo medio: toca el System Tray, que es una función de la Parte 3 que hoy
  funciona bien. Hay que probarlo con cuidado antes de darlo por hecho.

### Archivos que se tocarían

- `src-tauri/src/tray_service.rs` — reescritura del icono y su menú.
- `src-tauri/src/lib.rs` — línea ~102, donde se llama a `tray_service::setup`.
- `src-tauri/Cargo.toml` — puede necesitar la feature `Win32_UI_Shell` para
  `Shell_NotifyIconW` (ya está habilitada) y `Win32_System_LibraryLoader`.

## 5. Criterio de aceptación

La solución está bien solamente si se cumple todo esto:

1. Abrir y cerrar Desktop Organizer cinco veces seguidas con **Salir**: en la
   bandeja hay siempre exactamente un icono.
2. Matar el proceso desde el Administrador de tareas y volver a abrir la app:
   sigue habiendo exactamente un icono.
3. Todas las entradas del menú del tray funcionan igual que antes.
4. El clic izquierdo sobre el icono sigue abriendo el Administrador.
5. Los Cajones y el Dock siguen funcionando sin cambios.

## 6. Alternativas descartadas

| Alternativa | Por qué se descartó |
| --- | --- |
| Barrer el área de notificación al iniciar para que Windows borre los fantasmas | Es un parche: no ataca la causa, solo esconde el síntoma. Además depende de detalles internos del Explorador que cambian entre versiones de Windows. |
| Solo asegurar el borrado en todos los cierres controlados | Necesario pero insuficiente: no cubre el caso de que la aplicación muera de golpe, que es justamente cuando aparece el fantasma. Queda incluido dentro de la solución elegida, en el punto 6. |
| Esperar a que Tauri lo arregle upstream | No hay ninguna señal de que vaya a implementar `NIF_GUID`. No se puede depender de eso. |

## 7. Fuentes

- [Tauri — How to properly remove system tray icon before exit? (Discussion #4668)](https://github.com/tauri-apps/tauri/discussions/4668)
- [Tauri — v2 configuration tray icon appears with multiple tray icons (Issue #8982)](https://github.com/tauri-apps/tauri/issues/8982)
- [Microsoft — NOTIFYICONDATAW: campo `guidItem` y bandera `NIF_GUID`](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-notifyicondataw)
- [tauri-apps/tray-icon (librería usada, sin soporte de GUID)](https://github.com/tauri-apps/tray-icon)
