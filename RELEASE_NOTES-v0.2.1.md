# Desktop Organizer v0.2.1

Esta actualización estabiliza el Dock y deja preparado el flujo de futuras
actualizaciones desde la propia aplicación.

## Novedades

- Actualizador integrado en el Administrador, sin ventanas externas: busca,
  informa el estado, muestra el progreso y permite instalar la nueva versión.
- Carpeta física propia `Documentos\Desktop Organizer\Dock - Accesos`.
- Lo arrastrado desde el Escritorio se mueve a esa carpeta; los orígenes externos
  se conservan y se representan mediante un acceso directo.
- El Dock se reconstruye desde esa carpeta y su metadata oculta `.dock.json`.
- Restaurar al Escritorio protege contra sobrescrituras.
- La uñita se mantiene sobre ventanas normales, pero queda detrás de películas,
  juegos y aplicaciones fullscreen del mismo monitor, igual que la barra de tareas.

## Datos conservados

La instalación y la desinstalación no eliminan `Documentos\Desktop Organizer`.
Los Cajones permanecen en `Cajones` y los accesos del Dock en
`Dock - Accesos`.

## Compatibilidad

Actualización directa desde v0.2.0. Incluye instalador NSIS firmado para Windows
x64 y manifest `latest.json` para las próximas actualizaciones.
