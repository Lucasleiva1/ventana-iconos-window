# Desktop Organizer v0.4.0

Parte 9 completa: configuración global, recuperación integral, actualizaciones,
diagnóstico y optimización.

## Novedades

- Centro de configuración global para inicio, comportamiento, rendimiento,
  animaciones y valores predeterminados de Paneles.
- Centro de backups con ocho copias rotativas, creación manual, restauración,
  exportación y eliminación segura.
- Importación integral de Cajones, Dock, Paneles y preferencias, siempre con
  backup previo.
- Actualizador integrado con consulta automática cada 24 horas, notas de
  versión, progreso y backup obligatorio antes de instalar.
- Diagnóstico copiable sin datos personales, health check y logs locales
  rotativos.
- Migraciones encadenadas desde todos los schemas anteriores hasta el schema 9
  y recuperación parcial cuando un módulo aislado está dañado.
- Modo rendimiento y control global de animaciones para todas las ventanas.
- Guardado final de la geometría pendiente de Paneles al salir desde el Tray.

## Instalación

Descargá `Desktop-Organizer-v0.4.0-Setup.exe` desde esta Release. La instalación
manual conserva Cajones, Dock, Paneles, preferencias y backups existentes.

Esta instalación actualiza la clave pública del updater. A partir de v0.4.0,
las futuras versiones firmadas con el mismo par podrán instalarse desde la
propia aplicación.

## Verificación

La Release incluye `SHA256SUMS-v0.4.0.txt`, la firma Tauri del instalador y
`latest.json`. La suite local pasó 46 pruebas Rust, TypeScript, build frontend y
Clippy estricto sin advertencias.

No contiene telemetría ni sube archivos o diagnósticos a Internet.
