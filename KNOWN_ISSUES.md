# Limitaciones conocidas — v1.0.0

Esto no es una lista de deseos ni de mejoras futuras: son las limitaciones
reales que tiene la versión publicada. Ninguna impide usar la aplicación ni
pone en riesgo archivos del usuario.

## 1. Windows muestra una advertencia de SmartScreen al instalar

**Qué pasa.** La primera vez que ejecutás el instalador, Windows muestra
«Windows protegió su PC». Hay que elegir *Más información* → *Ejecutar de todas
formas*.

**Por qué.** No hay certificado comercial de firma de código (code signing).
Es un trámite pago que identifica al editor ante Windows. No se contrató.

**Qué NO significa.** No significa que el paquete no esté verificado. La firma
del updater sí existe y es la que protege las actualizaciones. Además cada
Release publica el SHA-256 del instalador para que puedas comprobar que el
archivo es exactamente el original.

## 2. Las versiones 0.3.0 y anteriores no se actualizan solas a 1.0.0

**Qué pasa.** Si tenés instalada una versión hasta la `0.3.0`, la aplicación no
va a ofrecerte la `1.0.0`. Hay que descargar el instalador e instalarlo a mano.

**Por qué.** El par de claves de firma del updater se cambió en la `0.4.0`. Una
aplicación sólo acepta actualizaciones firmadas con la clave que lleva grabada
adentro, y esas versiones llevan la anterior.

**Qué hacer.** Instalar `Desktop-Organizer-v1.0.0-Setup.exe` manualmente. Los
datos de `Documentos\Desktop Organizer` no se tocan. Desde la `1.0.0` en
adelante las actualizaciones vuelven a ser automáticas.

## 3. El icono de bandeja puede volver a duplicarse si movés el ejecutable

**Qué pasa.** En una instalación normal, es imposible que aparezcan dos iconos.
Pero si el mismo GUID quedó asociado a un ejecutable en otra ruta —pasa al
alternar entre una compilación de desarrollo y la instalada— Windows rechaza el
identificador fijo y la aplicación cae al modo clásico, donde un cierre brusco
puede volver a dejar un icono fantasma.

**A quién afecta.** Prácticamente sólo a desarrollo. Un usuario que instala y
usa la aplicación normalmente no entra nunca en este caso.

**Por qué se dejó así.** La alternativa era quedarse sin icono cuando Windows
rechaza el GUID, que es claramente peor.

## 4. El Dock consulta una vez por segundo si hay algo a pantalla completa

**Qué pasa.** Un hilo pregunta a Windows cada segundo si hay una aplicación
ocupando toda la pantalla, para esconder el tirador igual que hace la barra de
tareas.

**Por qué.** Windows no ofrece ningún aviso para eso; hay que preguntarlo. Es la
misma señal que usa el propio sistema para no mostrar notificaciones encima de
un video.

**Impacto medido.** Ver el uso de CPU en reposo registrado en `TESTING.md`.

## 5. Pruebas que no se pudieron hacer físicamente

El equipo donde se hizo el QA final tiene **un solo monitor de 1024x768 a
escala 100%, con Windows 10 Pro 22H2 (build 19045) de 64 bits**. Por lo tanto:

| Prueba | Estado |
| --- | --- |
| Multimonitor: mover Cajón/Dock/Panel entre pantallas | **No probado físicamente** — hay un solo monitor |
| Monitor que se desconecta y las ventanas vuelven | **No probado físicamente** — hay un solo monitor |
| Monitores con DPI distinto entre sí | **No probado físicamente** — hay un solo monitor |
| Escalado a 125% y 150% | **No probado físicamente** — el equipo está a 100% |
| Windows 11 | **No probado físicamente** — el equipo es Windows 10 |

El código sí contempla estos casos: normaliza posiciones contra la lista real de
monitores al iniciar, separa los Paneles que caen apilados cuando desaparece un
monitor, y reacciona al cambio de escala (`ScaleFactorChanged`) recolocando Dock
y Paneles. Está cubierto por pruebas automáticas de la lógica de geometría, pero
**no verificado sobre hardware real con varias pantallas**.

## 6. Recuperar un Panel sin el guardado maestro

Un Cajón se puede reconstruir desde su carpeta física porque los archivos están
ahí. Un Panel no: guarda sólo referencias, y su disposición vive en el guardado
maestro y sus backups. Si se pierden los dos, se pierde la disposición del
Panel — nunca los archivos referenciados, que siguen donde estaban.

Por eso conviene exportar la configuración de vez en cuando y guardarla aparte.
