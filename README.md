# Desktop Organizer

Desktop Organizer es una aplicación nativa para Windows que organiza el Escritorio mediante cajones visuales respaldados por carpetas físicas reales y un Dock retráctil de accesos rápidos. La versión `0.2.0` cierra los módulos Cajones y Dock.

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

Los documentos y carpetas no se guardan dentro de la aplicación ni dentro del JSON. Viven en la carpeta Documentos real informada por Windows:

```text
Documentos\Desktop Organizer\Cajones
```

Windows puede redirigir Documentos o Escritorio a OneDrive u otra ubicación. Desktop Organizer consulta las Known Folders nativas y no presupone una ruta `C:\Users\...`.

Quitar un cajón del organizador elimina solamente su representación visual. Su carpeta y su contenido permanecen en disco. La desinstalación tampoco administra ni elimina `Documentos\Desktop Organizer`.

Consultá [RECOVERY.md](RECOVERY.md) para recuperación manual y reinstalación.

## Instalación

1. Cuando se publique v0.2.0, descargá `Desktop-Organizer-v0.2.0-Setup.exe` desde GitHub Releases.
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

El Dock es un lanzador, no un almacén: todo lo que le arrastres queda como acceso y el archivo, la carpeta o el programa original se quedan exactamente donde estaban.

1. Con el Dock activado, en el borde inferior de la pantalla queda un tirador pequeño.
2. **Un clic** en el tirador abre el Dock; otro clic lo cierra. Pasar el mouse por encima sólo lo ilumina: nunca lo abre.
3. Arrastrá programas, accesos directos, carpetas o archivos desde el Explorador hacia el Dock.
4. **Un clic** sobre un icono lo abre. Si la preferencia «Ocultar Dock después de abrir un elemento» está activada, el Dock se aparta solo.
5. Arrastrá un icono para reordenarlo. El orden se guarda.
6. Clic derecho sobre un icono: Abrir, Abrir ubicación, Renombrar en Dock, Buscar nueva ubicación si está roto o Quitar del Dock. Quitar borra sólo el acceso.
7. Clic derecho sobre el fondo del Dock permite añadir separadores y actualizar la disponibilidad. Los separadores también se reordenan.
8. `Esc` cierra el Dock. El Administrador, el System Tray y el shortcut global opcional también pueden mostrarlo u ocultarlo.

La sección **DOCK** del Administrador permite elegir monitor, ancho automático/manual, tamaño y espaciado de iconos, geometría y posición del tirador, colores, opacidades, bordes, blur, animaciones, modo rendimiento y shortcut.

La diferencia es intencional: un **Cajón** puede mover físicamente archivos a su carpeta administrada; el **Dock** sólo conserva referencias y nunca mueve ni elimina el original.

## Ubicación de datos

Dentro de la carpeta Documentos conocida por Windows:

```text
Desktop Organizer\
├── Cajones\
├── Backups\
└── desktop-organizer-save.json
```

- `Cajones`: contenido real del usuario.
- `desktop-organizer-save.json`: apariencia, posiciones, IDs, orden, preferencias y accesos del Dock.
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

`v0.2.0`: Dock avanzado implementado y build firmado validado localmente. La
publicación está bloqueada por la validación pendiente del icono del Tray tras
cierres forzados. Parte 7 (Paneles organizadores) pendiente.
