# Desktop Organizer

Desktop Organizer es una aplicación nativa para Windows que organiza el Escritorio mediante cajones visuales respaldados por carpetas físicas reales y un Dock retráctil de accesos rápidos. La versión `0.2.1` estabiliza el almacenamiento y la actualización del Dock.

## Qué incluye

- Cajones visuales independientes, movibles, redimensionables, bloqueables y configurables.
- Archivos, carpetas, accesos directos y vínculos a aplicaciones con iconos de Windows.
- Arrastrar y soltar desde el Escritorio, restauración al Escritorio y movimiento entre cajones.
- Subcajones físicos, navegación por niveles y orden visual persistente.
- System Tray con una sola instancia, ocultamiento del administrador e inicio opcional con Windows.
- Guardado atómico, cinco backups rotativos, recuperación automática y reconstrucción desde disco.
- Importación y exportación de configuración.
- Dock retráctil configurable con accesos, separadores, shortcut global, reparación de rutas y orden persistente.

## Cómo protege los archivos

La arquitectura central es:

```text
Cajón visual ↔ carpeta física real
```

Los documentos, carpetas y accesos no se guardan dentro de la aplicación. Viven en carpetas reales y claramente separadas dentro de Documentos:

```text
Documentos\Desktop Organizer\
├── Cajones\
└── Dock - Accesos\
```

Windows puede redirigir Documentos o Escritorio a OneDrive u otra ubicación. Desktop Organizer consulta las Known Folders nativas y no presupone una ruta `C:\Users\...`.

Quitar un cajón del organizador elimina solamente su representación visual. Su carpeta y su contenido permanecen en disco. La desinstalación tampoco administra ni elimina `Documentos\Desktop Organizer`.

Consultá [RECOVERY.md](RECOVERY.md) para recuperación manual y reinstalación.

## Instalación

1. Descargá `Desktop-Organizer-v0.2.1-Setup.exe` desde GitHub Releases.
2. Verificá su SHA-256 con el archivo `SHA256SUMS.txt` de la misma Release.
3. Ejecutá el instalador. La instalación es por usuario y no requiere privilegios de administrador.
4. Abrí **Desktop Organizer** desde el menú Inicio.

Windows puede mostrar una advertencia de SmartScreen porque esta primera versión no posee certificado comercial de firma de código. El asset del actualizador sí está firmado criptográficamente por Tauri para impedir reemplazos no autorizados en futuras actualizaciones.

## Uso básico

1. Creá un cajón desde el Administrador.
2. Arrastrá archivos o carpetas del Escritorio hacia el cajón.
3. Usá doble clic para abrirlos y el menú contextual para moverlos, convertir carpetas en subcajones o restaurarlos.
4. Abrí o copiá la ubicación física desde el Administrador o desde las opciones del cajón.
5. Cerrá o minimizá el Administrador para mantener la aplicación en el área de notificación.
6. Usá **Tray → Salir** para finalizar completamente.

## El Dock

El Dock es un lanzador con respaldo físico propio en `Documentos\Desktop Organizer\Dock - Accesos`.

1. Con el Dock activado, en el borde inferior de la pantalla queda un tirador pequeño.
2. **Un clic** en el tirador abre el Dock; otro clic lo cierra. Pasar el mouse por encima sólo lo ilumina: nunca lo abre.
3. Arrastrá programas, accesos directos, carpetas o archivos desde el Explorador hacia el Dock. Si vienen del Escritorio, se mueven a la carpeta del Dock y desaparecen del Escritorio. Si vienen de otra ubicación, el original queda intacto y el Dock guarda una copia del acceso o crea un `.lnk`.
4. **Un clic** sobre un icono lo abre. Si la preferencia «Ocultar Dock después de abrir un elemento» está activada, el Dock se aparta solo.
5. Arrastrá un icono para reordenarlo. El orden se guarda.
6. Clic derecho sobre un icono: Abrir, Abrir ubicación, Renombrar en Dock, Buscar nueva ubicación si está roto o Restaurar al Escritorio. La restauración nunca sobrescribe un nombre existente.
7. Clic derecho sobre el fondo del Dock permite añadir separadores, abrir su carpeta física y actualizarla. Cualquier elemento colocado manualmente en el primer nivel de esa carpeta aparece en el Dock.
8. `Esc` cierra el Dock. El Administrador, el System Tray y el shortcut global opcional también pueden mostrarlo u ocultarlo.
9. La uñita tiene la misma prioridad visual que la barra de tareas: queda sobre ventanas normales, pero una película, juego o aplicación fullscreen del mismo monitor la cubre.

La sección **DOCK** del Administrador permite elegir monitor, ancho automático/manual, tamaño y espaciado de iconos, geometría y posición del tirador, colores, opacidades, bordes, blur, animaciones, modo rendimiento y shortcut.

La diferencia es intencional: cada **Cajón** tiene su carpeta física propia dentro de `Cajones`; el **Dock** usa una única carpeta física diferenciada llamada `Dock - Accesos`.

## Ubicación de datos

Dentro de la carpeta Documentos conocida por Windows:

```text
Desktop Organizer\
├── Cajones\
├── Dock - Accesos\
├── Backups\
└── desktop-organizer-save.json
```

- `Cajones`: contenido real del usuario.
- `Dock - Accesos`: elementos y accesos reales mostrados por el Dock. Incluye una metadata oculta `.dock.json` con nombres, IDs, separadores y orden, por lo que la carpeta completa sirve como copia de seguridad.
- `desktop-organizer-save.json`: apariencia, posiciones, IDs, orden y preferencias generales.
- `Backups`: hasta cinco copias recientes válidas del save y, cuando corresponde, copias preservadas de saves corruptos.
- `.drawer.json`: metadata oculta mínima de identidad dentro de cada cajón o subcajón.

## Tecnologías

- Tauri 2 y Rust.
- React 19 y TypeScript.
- Vite 8.
- APIs nativas de Windows para Known Folders, ventanas, iconos y movimientos atómicos.

## Desarrollo

Requisitos: Windows 10/11, Node.js, npm, Rust estable y las herramientas de compilación MSVC para Tauri.

```powershell
npm install
npm run tauri -- dev
```

Comprobaciones:

```powershell
npm run check
npm run build
cargo test --manifest-path src-tauri\Cargo.toml
cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings
```

Build NSIS:

```powershell
npm run tauri -- build --ci --bundles nsis
```

Los assets firmados requieren las variables de entorno privadas de Tauri. La clave privada no pertenece al repositorio.

## Estado

`v0.1.0`: módulo Cajones.

`v0.2.0`: Dock avanzado.

`v0.2.1`: carpeta física recuperable para el Dock, actualizador integrado y
prioridad visual compatible con aplicaciones fullscreen. Parte 7 pendiente.
