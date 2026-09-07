# Desktop Organizer v1.1.3

## El Dock ya no se abre solo

Hasta esta versión el Dock aparecía sin que nadie lo abriera: después de tocar
el escritorio, al hacer clic en un Cajón o en cualquier otra ventana, la barra
se desplegaba sola.

**Por qué pasaba.** El Dock tiene un vigilante que baja la barra cuando hay una
aplicación a pantalla completa, para no taparla. Para reconocerla medía si la
ventana de adelante ocupaba todo el monitor. El escritorio de Windows ocupa
todo el monitor: al hacer clic en el fondo pasa al frente `Progman`, con el
rectángulo exacto de la pantalla. El Dock lo tomaba por una aplicación a
pantalla completa, escondía la barra sin anotar que la había cerrado, y al
volver el foco a cualquier otra ventana la mostraba de nuevo.

**Qué cambia.**

- El escritorio, los iconos del escritorio, las barras de tareas y el Alt+Tab
  dejan de contar como aplicación a pantalla completa.
- Si la barra se esconde por un video a pantalla completa real, queda cerrada
  de verdad: al salir vuelve el tirador, no el Dock desplegado.
- El Dock se abre solamente cuando se lo pide: el tirador, el atajo de teclado,
  el menú de la bandeja o el Administrador.

Como efecto secundario desaparece otra molestia: a veces un clic en el tirador
no abría nada, porque el programa creía que el Dock ya estaba abierto.

**El tirador no cambió.** Sigue siempre visible, igual que desde la 1.1.2.

## Actualización

Automática desde cualquier versión anterior. Los Cajones, los accesos del Dock
y la configuración no se tocan.
