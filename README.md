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

## Los tres sistemas

Desktop Organizer tiene tres formas de ordenar, y hacen cosas distintas a propósito.

| | **Cajón** | **Dock** | **Panel** |
| --- | --- | --- | --- |
| Para qué sirve | Guardar cosas fuera del Escritorio | Lanzar rápido lo de todos los días | Tener accesos siempre a la vista |
| Qué hace con los archivos | **Los mueve de verdad** a una carpeta real | Guarda accesos en su propia carpeta | **Sólo guarda una referencia** |
| Cómo se ve | Ventana que se puede contraer u ocultar | Barra que aparece con el tirador | Ventana fija sobre el escritorio |
| Cómo se abre algo | Doble clic | Un clic | Doble clic |

## El Panel

Un Panel es un organizador visual: una ventana propia que queda sobre el escritorio con los accesos que le pongas.

**Nunca mueve tus archivos.** Si arrastrás `Escritorio\popes` a un Panel, `popes` sigue estando en el Escritorio. El Panel guarda sólo la ruta. Si querés sacar algo del Escritorio de verdad, eso lo hacen los Cajones.

### Los iconos se adaptan solos

Es lo que distingue al Panel. Al agrandarlo los iconos crecen; al achicarlo se reducen; y cuando llegan al mínimo de 24 px dejan de achicarse y aparece scroll. Nunca vas a terminar con iconos microscópicos. El máximo es 72 px.

También podés fijar el tamaño a mano: en modo manual el tamaño elegido se respeta siempre y, si no entra, se agregan filas y scroll.

### Qué se le puede poner

Programas, accesos directos, carpetas, archivos de cualquier tipo y hasta tus propios Cajones. Se arrastran desde el Explorador de Windows. Doble clic abre cada cosa con su aplicación de siempre; un Cajón se trae al frente.

### Qué se puede configurar

Densidad (compacta, normal o amplia), color, opacidad, estilo de fondo, cabecera normal o compacta, y si se ve el título. Se puede duplicar un Panel, expandirlo al escritorio y volver al tamaño anterior.

**Dos bloqueos distintos.** *Bloquear posición* impide moverlo y redimensionarlo. *Bloquear contenido* impide reordenar o quitar accesos, pero los sigue abriendo. Se pueden usar por separado.

**Imantado.** Al acercar un Panel a un borde de la pantalla o a otro Panel, se alinea solo. Mantené **Alt** mientras lo movés para colocarlo libremente.

### Seguridad

Quitar un acceso, duplicar un Panel, importar una configuración o eliminar un Panel entero **nunca** borra, mueve ni copia un archivo real. Lo único que se pierde al eliminar un Panel es su disposición.

A diferencia de los Cajones, un Panel no tiene una carpeta física que permita reconstruirlo: su disposición vive en el guardado maestro y sus copias de seguridad. Los archivos referenciados no se pierden nunca.

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
