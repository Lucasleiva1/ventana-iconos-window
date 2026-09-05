# Estado de desarrollo

```text
PARTE 1 — COMPLETADA
Base nativa y sistema de ventanas de cajones

PARTE 2 — COMPLETADA
Contenido real y almacenamiento físico

PARTE 3 — COMPLETADA
Subcajones, System Tray, single instance y configuración

PARTE 4 — COMPLETADA
Estabilización, seguridad, instalador y publicación v0.1.0

MÓDULO CAJONES — COMPLETO

PARTE 5 — COMPLETADA
Dock retráctil funcional

PARTE 6 — COMPLETADA
Dock avanzado + estabilización

MÓDULO DOCK — COMPLETO

PARTE 7 — PENDIENTE
Paneles organizadores adaptativos
```

La versión `0.2.1` incorpora el actualizador integrado, almacenamiento físico
del Dock y prioridad visual compatible con fullscreen.

## Qué entró en la Parte 5

- Ventana `dock` y ventana `dock-handle` (tirador), creadas una sola vez y
  gestionadas con `show()` / `hide()`.
- Apertura y cierre exclusivamente intencionales: clic en el tirador, Esc,
  System Tray, Administrador y auto-ocultamiento tras abrir un elemento.
- Posicionamiento sobre el área útil real del monitor, correcto a cualquier DPI
  y con la barra de tareas en cualquier borde.
- Accesos a programas, `.lnk`, carpetas y archivos arrastrados desde Windows.
  El Dock usa `Documentos\Desktop Organizer\Dock - Accesos`: mueve allí lo
  que viene del Escritorio y crea o copia accesos para orígenes externos.
- Reordenamiento por arrastre con orden persistente, tamaños de icono,
  opacidad, selección de monitor y scroll horizontal.
- `schemaVersion` 5 con migración automática desde el esquema 4 de `v0.1.0`.

## Qué entró en la Parte 6

- Geometría y posición configurables del tirador con límites de pantalla.
- Ancho automático/manual, espaciado y overflow con rueda y flechas.
- Separadores, nombres visuales y reparación de accesos rotos.
- Shortcut global configurable con detección de colisión.
- Color, opacidad, bordes, blur, animaciones y modo rendimiento.
- Esquema 6 compatible con todos los accesos creados durante Parte 5.

No se implementaron Paneles, múltiples docks ni orientación vertical/superior; pertenecen a etapas posteriores.
