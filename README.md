# Desktop Organizer

Desktop Organizer es una aplicación nativa y local-first para Windows. Integra Cajones con almacenamiento físico, un Dock retráctil y Paneles de referencias. `v1.0.0` es la primera versión estable y cierra el plan completo de desarrollo.

## Qué incluye

- Cajones visuales independientes, movibles, redimensionables, bloqueables y configurables.
- Archivos, carpetas, accesos directos y vínculos a aplicaciones con iconos de Windows.
- Arrastrar y soltar desde el Escritorio, restauración al Escritorio y movimiento entre cajones.
- Subcajones físicos, navegación por niveles y orden visual persistente.
- System Tray con una sola instancia, ocultamiento del administrador e inicio opcional con Windows.
- Guardado maestro versionado y atómico, ocho backups rotativos, recuperación automática y reconstrucción desde disco.
- Importación y exportación de configuración.
- Dock retráctil configurable con accesos, separadores, shortcut global, reparación de rutas y orden persistente.
- Centro de Configuración con preferencias globales, rendimiento, actualizaciones, backups y diagnóstico.
- Logs locales rotativos, health check y reporte de diagnóstico sin secretos ni contenido personal.

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

1. Descargá `Desktop-Organizer-v1.0.0-Setup.exe` desde GitHub Releases.
2. Verificá su SHA-256 con `SHA256SUMS-v1.0.0.txt` de la misma Release.
3. Ejecutá el instalador. La instalación es por usuario y no requiere privilegios de administrador.
4. Abrí **Desktop Organizer** desde el menú Inicio.

### Dos firmas distintas, y sólo una existe

Son dos cosas diferentes y conviene no confundirlas:

| | Estado | Para qué sirve |
| --- | --- | --- |
| **Firma del updater** (minisign/Tauri) | **Activa** | Impide que una actualización sea reemplazada por un paquete ajeno. Cada Release incluye la firma del instalador y la aplicación la verifica con la clave pública que lleva incrustada. |
| **Code signing de Windows** (certificado comercial) | **No existe** | Identificaría al editor ante Windows. No se contrató ningún certificado. |

Como consecuencia, **Windows va a mostrar una advertencia de SmartScreen** la
primera vez que ejecutes el instalador («Windows protegió su PC»). Para
continuar: *Más información* → *Ejecutar de todas formas*. Verificar el SHA-256
publicado es la forma de comprobar que el archivo es el original.

## Requisitos

- Windows 10 o Windows 11, 64 bits.
- **WebView2**: la aplicación lo necesita para dibujar su interfaz. Viene
  incluido en Windows 11 y en las versiones actualizadas de Windows 10. Si
  faltara, el instalador lo descarga e instala solo, sin preguntar.
- No hace falta instalar Node, npm, Rust ni ninguna herramienta de desarrollo:
  el instalador trae todo lo necesario.
- La instalación es por usuario y no pide privilegios de administrador.

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

## Configuración, backups y diagnóstico

El **Centro de configuración** reúne el inicio con Windows, inicio silencioso, cierre del Administrador, rendimiento global, animaciones y valores predeterminados de Paneles. Las opciones específicas de cada Cajón, Dock o Panel continúan junto a ese módulo y se guardan en el mismo estado maestro.

**Crear backup ahora** guarda configuración, metadata, referencias, posiciones y orden. No copia los archivos físicos de Cajones ni el contenido de `Dock - Accesos`. Desde el mismo centro se puede listar, validar, restaurar, exportar o eliminar cada copia. Antes de importar, restaurar, resetear la apariencia o instalar una actualización se preserva el estado actual.

**Restablecer configuración visual** devuelve apariencia y posiciones a valores seguros, pero conserva archivos, accesos y referencias. **Comprobar estado** revisa rápidamente save, carpetas, backups y updater. El informe copiable reemplaza el perfil de usuario por `%USERPROFILE%` y omite tokens, secretos y nombres de documentos.

Los logs se escriben sólo para eventos útiles, rotan al llegar a 1 MiB y mantienen como máximo cinco archivos. No hay telemetría ni envío automático.

## Actualizaciones

Configuración → **Actualizaciones** consulta `latest.json` de las Releases del repositorio oficial. Tauri compara versiones SemVer y verifica el paquete con la clave pública incorporada; la clave privada permanece fuera del repositorio. La búsqueda automática ocurre como máximo una vez cada 24 horas y un error de red no afecta el funcionamiento local.

La instalación siempre requiere una acción del usuario y crea primero un backup de configuración. Actualizar o reinstalar no elimina `Documentos\Desktop Organizer`. No se implementa rollback binario improvisado: ante una descarga o instalación fallida se conserva la versión operativa cuando el instalador lo permite y, en todos los casos, los datos permanecen fuera de la carpeta de instalación.

## Ubicación de datos

Dentro de la carpeta Documentos conocida por Windows:

```text
Desktop Organizer\
├── Cajones\
├── Dock - Accesos\
├── Backups\
├── Logs\
└── desktop-organizer-save.json
```

- `Cajones`: contenido real del usuario.
- `Dock - Accesos`: elementos y accesos reales mostrados por el Dock. Incluye una metadata oculta `.dock.json` con nombres, IDs, separadores y orden, por lo que la carpeta completa sirve como copia de seguridad.
- `desktop-organizer-save.json`: configuración y organización; nunca contiene los archivos del usuario.
- `Backups`: hasta ocho copias recientes válidas del save y, cuando corresponde, copias preservadas de saves corruptos.
- `Logs`: hasta cinco archivos locales rotativos de diagnóstico.
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

Build NSIS **firmado** para el updater (recomendado para publicar):

```powershell
npm run build:firmado
```

`scripts/build-firmado.ps1` toma el par de claves vigente de
`%APPDATA%\Desktop Organizer\updater`, comprueba que su clave pública sea la
misma que lleva incrustada `tauri.conf.json` y recién entonces compila. Si no
coinciden, aborta antes de generar nada: firmar con un par distinto dejaría a
las instalaciones existentes sin poder actualizarse.

El par vigente desde `v0.4.0` es `tauri-updater.key` (ID `AD66AFEE118F4C`). El
par anterior, `tauri-updater-v0.2.0.key` (ID `CE2E9BE59BB0436B`), quedó retirado
y ya no debe usarse: las versiones hasta `v0.3.0` inclusive están firmadas con
él y no pueden actualizarse automáticamente a `v0.4.0`, esa transición requiere
instalación manual.

La clave privada no pertenece al repositorio y `.gitignore` bloquea `*.key`.

## Estado

`v0.1.0`: módulo Cajones.

`v0.2.0`: Dock avanzado.

`v0.2.1`: carpeta física recuperable para el Dock, actualizador integrado y
prioridad visual compatible con aplicaciones fullscreen.

`v0.3.0`: Paneles avanzados completos.

`v0.4.0`: Parte 9 completa: configuración global, recuperación, updater, diagnóstico y optimización.

`v1.0.0`: primera versión estable. QA final, corrección del icono duplicado en
el área de notificación y cierre del plan de diez partes.
