# Desktop Organizer v1.0.1

Versión de corrección previa a los últimos ajustes estéticos.

## Corregido

- El inicio con Windows ahora registra el ejecutable entre comillas, por lo que
  funciona aunque la ruta contenga espacios.
- La aplicación valida que la entrada de inicio apunte al ejecutable actual, en
  lugar de considerar válida cualquier entrada antigua con el mismo nombre.
- Cuando «Iniciar con Windows» está activado, cada arranque normal repara la ruta
  registrada y vuelve a habilitar la aplicación en la lista de inicio de Windows.
- Se mantiene el argumento `--autostart` para restaurar Cajones, Dock y Paneles
  sin abrir el Administrador cuando «Inicio silencioso» está activado.

## Seguridad de datos

La actualización no mueve ni elimina archivos. El guardado maestro, los Cajones
físicos y la carpeta `Dock - Accesos` permanecen en `Documentos\Desktop Organizer`.

## Verificación

- 52 pruebas Rust aprobadas.
- TypeScript y build de producción aprobados.
- Prueba local real con una sola instancia iniciada mediante `--autostart`,
  ventanas restauradas y proceso nativo respondiendo.
