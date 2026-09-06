# Changelog

## 1.1.0 — 2026-09-06

### Cambiado

- **Fuera la etiqueta de modo de guardado** (“GUARDADO” / “VINCULADO”) que
  aparecía fija debajo de cada elemento: ocupaba lugar y no cambiaba ninguna
  decisión de uso. El nombre del archivo sigue visible.
- Al guardar algo, el aviso de abajo dice qué entró —con el nombre cuando es
  un solo elemento— y se borra solo. Si entran varias cosas seguidas, el
  aviso anterior se reemplaza por el nuevo.
- Se ocultó el texto explicativo del Cajón vacío (“Arrastrá elementos acá…”).
  El rombo sigue marcando que está vacío.
- Se controlan desde `src/features.ts` (`DRAWER_STORAGE_BADGES_VISIBLE`,
  `DRAWER_EMPTY_HINT_VISIBLE`): nada se borró, alcanza con ponerlos en `true`.

### Cambiado

- **Todo el mando del Cajón vive en su cabecera.** Se eliminó la barra que
  estaba dentro del cuerpo y repetía el nombre del cajón. El botón de
  actualizar pasó arriba, junto a contraer y opciones, y al entrar en un nivel
  la cabecera muestra su nombre con un botón para volver.
- **Los subcajones quedaron fuera de la vista** (`SUBDRAWERS_ENABLED` en
  `src/features.ts`). No eran un cajón dentro de otro sino un nivel más de la
  misma ventana, y para eso conviene crear otro Cajón al lado. El código sigue
  completo: alcanza con poner esa constante en `true` para recuperarlos, y los
  subcajones que existan en el disco se siguen abriendo con normalidad.

### Agregado

- **Diálogos propios del Cajón.** Renombrar o quitar un cajón ya no abren el
  cuadro gris de Windows con la dirección del servidor: usan un cuadro con el
  estilo de la aplicación, que se cierra con Escape o haciendo clic afuera.

### Cambiado

- **Una sola barra de opacidad por ventana, de fondo invisible a sólido.** La
  barra de Opacidad de cada Cajón, del Dock y de cada Panel ahora llega hasta
  cero: al fondo la ventana es sólida, al principio el fondo desaparece y se ve
  el escritorio. Ya no hay interruptor global ni un segundo nivel que se
  multiplique con el de cada ventana.
- Los Paneles dejaron de recortar la opacidad según el estilo de fondo. El
  estilo describe el aspecto; el valor lo decide la barra.
- El Administrador no tiene barra de opacidad, así que nunca se vuelve
  transparente. Los menús de opciones y los ajustes también quedan sólidos.

### Corregido

- **Las puntas en las esquinas de Cajones y Paneles.** Windows acompaña la
  sombra del sistema con un marco recto de 1 píxel que asomaba por fuera del
  borde redondeado y dibujaba una esquina cuadrada. Esas dos ventanas se
  crean ahora sin sombra del sistema, igual que el Dock, que ya no la tenía.

- **El fondo del documento tapaba el escritorio.** El lienzo de todas las
  ventanas se pintaba con el color oscuro de la aplicación, así que ninguna
  opacidad podía dejar ver lo que había detrás: el Cajón se veía gris hiciera
  lo que hiciera la barra. Ahora el fondo sólido lo pinta únicamente el
  Administrador.
- Cada ventana se identifica desde el arranque (`data-window` en el documento)
  en lugar de deducirse del contenido ya renderizado, que era frágil y dejaba
  ventanas con el fondo equivocado.

### Agregado

- La cabecera de los Paneles conserva su propio fondo: con la opacidad al
  mínimo la ventana sigue teniendo de dónde agarrarse para moverla.
- Los nombres de los elementos llevan sombra para leerse sobre el escritorio
  cuando el fondo está en cero.
- El Administrador se reorganizó en secciones desplegables: Cajones, Dock,
  Iconos fijos, Configuración general, Actualizaciones, Backups y Diagnóstico.

### Pruebas

- 52 pruebas Rust, verificación de tipos TypeScript y compilación de producción.

## 1.0.1 — 2026-09-06

### Corregido

- El inicio con Windows registra el ejecutable entre comillas y funciona con
  rutas que contienen espacios.
- La aplicación valida que la entrada apunte al ejecutable actual y la repara
  automáticamente cuando la preferencia está activada.
- Windows vuelve a marcar explícitamente Desktop Organizer como habilitado en
  Aplicaciones de inicio al activar o reparar la opción.

### Pruebas

- Arranque local mediante `--autostart`: una sola instancia, proceso
  respondiendo y ventanas restauradas.
- 52 pruebas Rust, TypeScript, compilación de producción y verificación del
  guardado maestro.

## 1.0.0 — 2026-09-06

Primera versión estable. Cierra el plan de diez partes: Cajones, Dock y Paneles
funcionando en conjunto sobre una base de persistencia, recuperación y
actualización ya probada.

### Corregido

- **El icono del área de notificación ya no puede duplicarse.** Se registraba
  con el par ventana+número que Windows asigna en cada arranque, así que los
  restos de una ejecución anterior quedaban dibujados al lado del icono nuevo.
  Ahora el icono tiene un identificador propio y permanente (`NIF_GUID`) y, al
  iniciar, se borra cualquier resto previo antes de registrarse. El menú pasó a
  la API nativa de Windows conservando sus doce acciones intactas.
- El icono se vuelve a registrar solo si se reinicia el Explorador de Windows, y
  se retira al cerrar sesión o apagar el equipo.

### Cambiado

- Versión estable alineada como `1.0.0` en aplicación, paquete, instalador y
  metadata de Windows.
- Metadata del instalador completa: editor, copyright, categoría y descripciones.
- `npm run build:firmado` deja el instalador firmado verificando primero que el
  par de claves coincida con el que lleva incrustada la aplicación.

### Quitado

- La dependencia de la funcionalidad de bandeja de Tauri (`tray-icon`), que
  quedó sin uso al pasar el icono a la API nativa.

### Pruebas

- 51 pruebas Rust, TypeScript, build de frontend y Clippy estricto sin
  advertencias.
- Movimiento entre volúmenes distintos verificado con archivos reales: copia,
  verificación y recién entonces borrado del origen.
- Nombres con acentos, eñes, kanji y espacios múltiples verificados en un ciclo
  real de mover al Cajón y restaurar al Escritorio.

## 0.4.0 — 2026-09-06

### Agregado

- Centro de configuración global para inicio, comportamiento del Administrador, rendimiento, animaciones y valores predeterminados de Paneles.
- Centro de backup con estado del save, ocho copias rotativas, creación manual, listado, restauración, exportación y eliminación segura.
- Diagnóstico copiable sin secretos, health check, carpeta de datos accesible y logs locales rotativos limitados a cinco archivos de 1 MiB.
- Búsqueda automática de actualizaciones como máximo cada 24 horas, notas/fecha de Release y backup obligatorio antes de instalar.

### Cambiado

- `schemaVersion` pasa de 8 a 9 mediante migraciones encadenadas directas desde todos los schemas anteriores admitidos.
- Importar configuración crea un backup previo e integra Cajones, Dock, Paneles y preferencias globales antes de reconstruir sus ventanas.
- El modo rendimiento y las animaciones se aplican coherentemente a todas las ventanas y respetan `prefers-reduced-motion`.
- Tray → Salir captura también la geometría pendiente de Paneles antes del último guardado.
- Versión estable alineada como `0.4.0`, preparada para distribución manual y futuras actualizaciones firmadas.

### Seguridad

- La validación recupera módulos sanos aunque Dock, un Cajón o un Panel aislado sean inválidos; rutas rotas quedan no disponibles sin provocar un crash.
- Restaurar, importar, resetear visualmente o instalar una actualización conserva un backup del estado anterior y nunca elimina contenido físico de Cajones.
- `.gitignore` bloquea formatos habituales de claves privadas y certificados de firma.

## 0.3.0

Tercer módulo completo: **Paneles organizadores adaptativos**. Cajones, Dock y
Paneles funcionan simultáneamente.

### Agregado

- Paneles organizadores: ventanas propias del escritorio, movibles y
  redimensionables libremente por bordes y esquinas.
- Escalado automático del tamaño de icono entre 24 y 72 px según el espacio
  real del Panel, con scroll cuando el contenido ya no entra.
- Modo manual de tamaño de icono: el tamaño elegido no se reduce nunca.
- Densidad por Panel: compacta, normal y amplia.
- Grilla responsive con columnas automáticas y texto adaptativo: nombre
  completo, nombre corto o sólo icono con tooltip.
- Arrastrar programas, accesos directos, carpetas y archivos desde Windows.
  Todo queda como referencia: el original nunca se mueve.
- Reordenamiento por arrastre con orden persistente y selección múltiple con
  Ctrl y Shift.
- Imantado a los bordes del monitor y a otros Paneles, con Alt para mover libre.
- Bloqueo de posición y bloqueo de contenido, independientes entre sí.
- Duplicar Panel, expandir al escritorio y restaurar el tamaño anterior.
- Estilos de fondo, cabecera compacta, título ocultable, color y opacidad.
- Cajones colocables dentro de un Panel; doble clic los trae al frente.
- Alinear y distribuir Paneles desde el Administrador.
- Sección PANELES en el Administrador y en el System Tray.
- Virtualización de la grilla a partir de 240 accesos.

### Cambiado

- `schemaVersion` pasa de 6 a 8. Los saves de `v0.2.x` se migran solos y
  conservan cajones, Dock, preferencias y archivos.
- Exportar e importar configuración ahora incluye los Paneles. Importar nunca
  mueve ni copia archivos.
- La caché de iconos conserva también los iconos usados por los Paneles.

### Seguridad

- Los Paneles no administran almacenamiento físico: agregar, quitar, duplicar,
  importar o eliminar un Panel jamás mueve, copia ni borra un archivo real.


## 0.2.1 — 2026-09-05

### Agregado

- Actualizador integrado en el Administrador con búsqueda, progreso, confirmación
  e instalación dentro de la propia interfaz.
- Carpeta física diferenciada `Documentos\Desktop Organizer\Dock - Accesos`,
  reconstrucción automática desde su contenido y metadata oculta portable con
  identidad, nombres, separadores y orden.
- Prioridad visual equivalente a la barra de tareas: la uñita permanece sobre
  ventanas normales y queda detrás de aplicaciones fullscreen del mismo monitor.

### Cambiado

- Los elementos arrastrados desde el Escritorio al Dock se mueven a su carpeta
  física. Para orígenes externos se copia el acceso o se crea un `.lnk`, sin
  mover el original.
- Quitar un elemento del Dock ahora lo restaura al Escritorio y nunca
  sobrescribe un nombre existente.

## 0.2.0 — 2026-09-05

### Agregado

- Dock retráctil en el borde inferior del monitor, con tirador propio.
- El Dock se abre y se cierra sólo con acciones intencionales: clic en el
  tirador, `Esc`, System Tray o Administrador. El hover nunca lo abre; no hay
  hot edge ni hot corner.
- Accesos a programas, accesos directos `.lnk`, carpetas y archivos arrastrados
  desde el Explorador de Windows.
- Un clic abre el elemento con la aplicación predeterminada de Windows; las
  carpetas se abren en el Explorador.
- Reordenamiento por arrastre con orden persistente y distinción entre clic y
  arrastre.
- Menú por elemento con Abrir, Abrir ubicación y Quitar del Dock.
- Preferencia «Ocultar Dock después de abrir un elemento», activada por defecto.
- Sección DOCK en el Administrador: activar, monitor, tamaño de iconos,
  opacidad, mostrar, ocultar y reubicar.
- Entradas «Mostrar Dock» y «Ocultar Dock» en el System Tray existente.
- Accesos rotos marcados como no disponibles, sin borrado automático.
- Tirador configurable en ancho, alto, opacidad, alineación izquierda/centro/derecha y desplazamiento horizontal seguro.
- Ancho automático o manual limitado por el área útil real, espaciado compacto/normal/amplio y flechas de scroll cuando existe overflow.
- Separadores persistentes, reordenables y eliminables.
- Nombre visual editable sin renombrar el archivo real, actualización manual del Dock y reparación de accesos rotos mediante selector nativo.
- Shortcut global opcional y configurable para mostrar u ocultar el Dock, con mensaje explícito si la combinación está ocupada.
- Color de fondo, radio de borde, blur opcional, tres niveles de animación y modo rendimiento.
- Feedback explícito de destino durante el reordenamiento.
### Cambiado

- `schemaVersion` pasa de 5 a 6. Los saves de Parte 5 y de `v0.1.0` se migran solos y
  conservan cajones, elementos, posiciones y preferencias.
- La caché de iconos ahora conserva también los iconos usados por el Dock.
- El Administrador se muestra de forma diferida al iniciar para evitar que Windows deje oculta una ventana creada con `visible: false` antes del loop nativo.

## 0.1.0 — 2026-09-05

### Incluye

- Cajones visuales respaldados por carpetas físicas reales.
- Drag & drop desde el Escritorio, vínculos externos seguros y restauración al Escritorio.
- Archivos, carpetas, accesos directos, ejecutables e iconos nativos de Windows.
- Subcajones, navegación por niveles, conversiones seguras y movimiento entre cajones.
- Reordenamiento persistente, scroll y tamaños de icono.
- Administrador, System Tray único, single instance e inicio opcional y silencioso con Windows.
- Known Folders nativas para Documentos y Escritorio, incluidas rutas redirigidas.
- Guardado atómico, cinco backups rotativos, importación/exportación y recuperación desde disco.
- Recuperación automática del save maestro y reconstrucción de metadata corrupta sin borrar contenido.
- Caché de iconos limitada y limpieza de archivos temporales.
- Instalador NSIS por usuario y assets de actualización firmados.

### Seguridad

- Eliminar un cajón visual nunca elimina su carpeta física.
- Los conflictos de nombre nunca sobrescriben ni fusionan contenido.
- Entre volúmenes se copia, verifica con SHA-256 y recién después se intenta retirar el origen.
- Los saves y datos personales quedan fuera del repositorio y de la carpeta de instalación.

### Pendiente

- Parte 5: Dock/barra retráctil.
