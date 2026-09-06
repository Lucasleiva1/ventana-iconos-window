# Desktop Organizer v1.0.0

Primera versión estable. Cierra el plan completo de diez partes: los tres
módulos —Cajones, Dock y Paneles— funcionan en conjunto sobre una base de
persistencia, recuperación y actualización ya probada.

## Los tres sistemas, y en qué se diferencian

Es la distinción más importante de la aplicación:

| | Cajones | Dock | Paneles |
| --- | --- | --- | --- |
| Qué es | Carpetas reales del disco | Barra de accesos | Organizadores visuales |
| Qué hace con tus archivos | **Los mueve de verdad** | Guarda accesos propios | **Sólo guarda una referencia** |
| Si borrás el elemento visual | El contenido físico queda intacto | El acceso desaparece, el original no | No se toca ningún archivo |

## Cajones

- Cada cajón visual es una carpeta real dentro de `Documentos\Desktop Organizer\Cajones`.
- Mover algo del Escritorio a un cajón lo saca del Escritorio de verdad.
- **Restaurar al Escritorio** lo devuelve físicamente, sin duplicarlo.
- Subcajones anidados, con navegación y orden manual propios de cada nivel.
- Entre volúmenes distintos copia, verifica y recién entonces borra el origen.
  Si algo falla en el medio, el original queda intacto.
- Un nombre repetido nunca sobrescribe ni fusiona: se avisa y no se toca nada.

## Dock

- Barra retráctil con tirador. **El hover nunca lo abre**: sólo el clic.
- Con el Dock oculto queda operativo únicamente el tirador.
- Acepta programas, accesos directos, carpetas, PDF, imágenes y documentos.
- Los archivos ajenos al Escritorio no se mueven: se guarda un acceso.
- Reordenamiento, separadores, reparación de accesos rotos y atajo global.
- Respeta las aplicaciones a pantalla completa, igual que la barra de tareas.

## Paneles

- Organizadores de referencias sobre el escritorio. **Nunca mueven archivos.**
- Los iconos se adaptan al tamaño del Panel entre 72 y 24 píxeles; llegado al
  mínimo aparece scroll en lugar de seguir achicándolos.
- Modo automático y manual, densidad configurable, imantado, bloqueo separado
  de posición y de contenido, duplicar y expandir al escritorio.
- La grilla se virtualiza a partir de 240 accesos.

## Sistema

- **Un único icono en el área de notificación, imposible de duplicar.**
- Inicio con Windows, con arranque silencioso opcional.
- Una sola instancia: volver a ejecutar la aplicación trae al frente la que ya
  está corriendo.
- Guardado atómico verificado, ocho backups rotativos, import/export y
  recuperación de Cajones desde el disco.
- Migraciones encadenadas desde todos los esquemas anteriores, siempre con
  backup previo.
- Diagnóstico copiable sin datos personales y logs locales rotativos.
- Actualizador integrado con firma verificada y backup antes de instalar.
- Sin telemetría: no se envía nada a Internet salvo la consulta de
  actualizaciones, que podés desactivar.

## Qué se corrigió en esta versión

**El icono del área de notificación se duplicaba.** Windows identifica un icono
de bandeja por el par ventana+número que el programa le entrega, y la librería
que usaba Tauri creaba una ventana nueva en cada arranque: los restos de
ejecuciones anteriores quedaban dibujados al lado del icono nuevo y se
acumulaban.

Ahora el icono se registra con un identificador propio y permanente
(`NIF_GUID`), y al iniciar se borra cualquier resto previo. Dos iconos ya no
pueden coexistir: comparten identificador. Además se vuelve a registrar solo si
se reinicia el Explorador de Windows y se retira al cerrar sesión.

## Instalación

Descargá `Desktop-Organizer-v1.0.0-Setup.exe` desde esta Release. La instalación
es por usuario, no pide privilegios de administrador y conserva Cajones, Dock,
Paneles, preferencias y backups existentes.

Si venís de `v0.3.0` o anterior, instalá manualmente: esas versiones están
firmadas con el par de claves anterior y no pueden actualizarse solas. Tus datos
en `Documentos\Desktop Organizer` no se tocan.

## Verificación

La Release incluye `SHA256SUMS-v1.0.0.txt`, la firma Tauri del instalador y
`latest.json`.

**Windows va a mostrar una advertencia de SmartScreen**: la aplicación no tiene
certificado comercial de firma de código. Es distinto de la firma del updater,
que sí existe y sí protege las actualizaciones. Comprobar el SHA-256 publicado
es la forma de verificar que el archivo es el original.

## Requisitos

Windows 10 o 11 de 64 bits. No hace falta instalar nada más: WebView2 lo
resuelve el propio instalador si faltara.
