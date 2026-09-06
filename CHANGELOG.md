# Changelog

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
