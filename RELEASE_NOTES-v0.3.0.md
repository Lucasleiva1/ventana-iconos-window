# Desktop Organizer v0.3.0

Esta versión cierra el tercer y último gran sistema: los **Paneles organizadores
adaptativos**. Ahora los tres funcionan a la vez sin pisarse: Cajones, Dock
retráctil y Paneles.

## Qué es un Panel

Un organizador visual: una ventana propia que queda sobre el escritorio con los
accesos que le pongas. **Nunca mueve tus archivos.** Si arrastrás algo del
Escritorio a un Panel, sigue estando en el Escritorio; el Panel guarda sólo la
referencia. Para sacar cosas del Escritorio de verdad están los Cajones.

## Novedades

- **Los iconos se adaptan solos.** Al agrandar el Panel crecen, al achicarlo se
  reducen, y al llegar al mínimo de 24 px dejan de achicarse y aparece scroll.
  El máximo es 72 px.
- **Modo manual.** Si preferís fijar el tamaño, se respeta siempre: cuando no
  entra, se agregan filas y scroll en lugar de encoger los iconos.
- **Densidad** compacta, normal o amplia.
- **Imantado.** Al acercar un Panel a un borde de la pantalla o a otro Panel se
  alinea solo. Mantené **Alt** mientras lo movés para colocarlo libremente.
- **Dos bloqueos independientes.** *Bloquear posición* impide moverlo y
  redimensionarlo; *bloquear contenido* impide reordenar o quitar accesos, pero
  los sigue abriendo.
- **Duplicar Panel**, **expandir al escritorio** y volver al tamaño anterior.
- **Selección múltiple** con Ctrl y Shift, para quitar varios accesos de una vez.
- Estilos de fondo (sólido, transparente, cristal y mínimo), cabecera compacta y
  título ocultable, color y opacidad por Panel.
- **Cajones dentro de un Panel**: doble clic los muestra y los trae al frente.
- **Alinear y distribuir** Paneles desde el Administrador.
- Sección PANELES en el Administrador y en el System Tray.
- Reordenamiento por arrastre con orden persistente, renombrado visual y
  reparación de accesos rotos.

## Mejoras del Dock

- El Dock ahora respeta la misma jerarquía que la barra de tareas de Windows:
  con una aplicación a pantalla completa deja de verse, y vuelve al salir.

## Seguridad

- Los Paneles no administran almacenamiento físico. Agregar, quitar, duplicar,
  importar o eliminar un Panel **jamás** mueve, copia ni borra un archivo real.
- Eliminar un Panel borra sólo su disposición y sus referencias.
- A diferencia de los Cajones, un Panel no tiene carpeta física que permita
  reconstruirlo: su disposición vive en el guardado maestro y sus copias de
  seguridad. Los archivos referenciados no se pierden nunca.

## Actualizar desde v0.2.x

La configuración se migra sola (esquema 6 a 8) y conserva cajones, Dock,
preferencias y archivos. Exportar e importar configuración ahora incluye los
Paneles, y la importación tampoco mueve archivos.

## Instalación

1. Descargá `Desktop-Organizer-v0.3.0-Setup.exe`.
2. Verificá su SHA-256 contra `SHA256SUMS-v0.3.0.txt`.
3. Ejecutalo. La instalación es por usuario y no pide privilegios de
   administrador.

Windows puede mostrar una advertencia de SmartScreen porque el instalador no
tiene certificado comercial de firma de código. El asset del actualizador sí
está firmado criptográficamente.

## Limitaciones conocidas

- Cada Panel abre su propia vista web, así que muchos Paneles simultáneos suman
  memoria. Probado con hasta dos Paneles y 61 accesos.
- Escalas DPI de 125 % y 150 % y configuraciones con varios monitores no se
  pudieron probar en hardware real.
- Arrastrar un elemento desde un Cajón o desde el Dock hacia un Panel todavía no
  está implementado; los Cajones se agregan desde el menú del Panel.
