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

PARTE 7 — COMPLETADA
Paneles organizadores adaptativos funcionales

PARTE 8 — COMPLETADA
Paneles avanzados + estabilización + Release v0.3.0

MÓDULO PANELES — COMPLETO

PARTE 9 — PENDIENTE
Configuración global + rendimiento + recuperación integral

PARTE 10 — PENDIENTE
Integración final + QA completo + Release v1.0.0
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

## Qué entró en la Parte 7

- Cada Panel es una ventana Tauri propia (`panel-<id>`), movible con la cabecera
  y redimensionable de verdad por bordes y esquinas. Nunca `always_on_top` y
  fuera de la barra de tareas.
- Escalado automático de iconos: 64 → 56 → 48 → 40 → 32 → 24 px según el tamaño
  del Panel, con `ResizeObserver` y `requestAnimationFrame`. Al llegar al mínimo
  de 24 px los iconos dejan de achicarse y aparece scroll.
- Grilla CSS con columnas calculadas automáticamente y texto adaptativo:
  nombre completo, nombre corto o sólo icono con tooltip.
- Todos los elementos del Panel son **referencias**: arrastrar algo a un Panel
  jamás mueve, copia ni borra el original.
- Máximo real por monitor: área útil menos un margen de seguridad, recalculado
  al cambiar de monitor, resolución o DPI. Un Panel cuyo monitor desaparece
  vuelve al principal.
- Reordenamiento por arrastre con orden persistente, renombrado visual,
  reparación de accesos rotos y bloqueo del Panel.
- Cajones como elemento especial dentro de un Panel: doble clic los trae al frente.
- Sección PANELES en el Administrador y en el System Tray, sin duplicar el icono.
- `schemaVersion` 7 con migración automática desde el esquema 6 de `v0.2.1`.

No se implementaron snapping, layouts guardados, paneles anidados, colocación
libre por coordenadas ni la Release `v0.3.0`; pertenecen a la Parte 8.

## Qué entró en la Parte 8

- Auditoría de la Parte 7 y corrección de tres errores reales: la medición del
  área de la grilla incluía el padding, importar configuración descartaba los
  Paneles, y varios Paneles recuperados de un monitor desconectado quedaban
  apilados en la misma esquina.
- Tamaño de icono automático hasta 72 px y manual con control propio: en manual
  el tamaño no se reduce nunca, se agregan filas y scroll.
- Densidad por Panel: compacta, normal y amplia.
- Imantado a los bordes del área útil y a los demás Paneles, activable por
  Panel y suspendible manteniendo Alt durante el movimiento.
- Bloqueo separado de posición y de contenido.
- Duplicar Panel copiando sólo referencias, con identificadores nuevos.
- Expandir al escritorio y restaurar el tamaño anterior, con resize controlado
  en lugar del maximizado de Windows.
- Selección múltiple con Ctrl y Shift, y quitar la selección completa.
- Estilos de fondo (sólido, transparente, cristal y mínimo), cabecera compacta
  y título ocultable.
- Alinear y distribuir Paneles desde el Administrador.
- Virtualización de la grilla a partir de 240 accesos, conservando intacto el
  arrastre en los Paneles normales.
- `schemaVersion` 8 con migración automática desde el esquema 7.
