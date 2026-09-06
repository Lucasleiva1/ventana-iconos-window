# Recuperación de Desktop Organizer

Los archivos reales no dependen de que Desktop Organizer funcione. Aunque la aplicación falle o sea desinstalada, pueden abrirse normalmente desde el Explorador de Windows.

## Dónde están los Cajones y el Dock

Desktop Organizer consulta la carpeta **Documentos** configurada por Windows. Desde la aplicación hay acciones separadas para **Abrir carpeta Cajones**, **Abrir carpeta Dock**, **Copiar ruta Cajones** y **Copiar ruta Dock**.

La estructura habitual es:

```text
Documentos\Desktop Organizer\
├── Cajones\                 # archivos y carpetas reales
├── Dock - Accesos\          # elementos y accesos reales del Dock
├── Backups\                 # copias recientes del save
├── Logs\                    # diagnóstico local rotativo
└── desktop-organizer-save.json
```

No presupongas que Documentos está en `C:\Users\...`: puede estar redirigido a OneDrive, una red u otra unidad.

## Recuperar sin la aplicación

1. Abrí el Explorador de Windows.
2. Entrá en **Documentos → Desktop Organizer → Cajones**.
3. Cada carpeta de primer nivel es un cajón físico y su contenido puede copiarse o abrirse normalmente.
4. Los archivos `.drawer.json` son metadata interna oculta; no contienen documentos del usuario.

Los elementos del Dock están en **Documentos → Desktop Organizer → Dock - Accesos**. El archivo oculto `.dock.json` sólo conserva identidad, nombres visuales, separadores y orden.

## Si el save principal está dañado

Al iniciar, Desktop Organizer valida `desktop-organizer-save.json`. Si no es válido:

1. busca el backup válido más reciente;
2. conserva el save dañado dentro de `Backups`;
3. restaura el master mediante escritura temporal, validación y reemplazo atómico;
4. muestra un aviso discreto en el Administrador.

Si ningún backup sirve, la aplicación no sobrescribe silenciosamente los datos. Ofrece **Recuperar desde disco** cuando encuentra carpetas físicas.

La carga migra en cadena cualquier schema anterior admitido. Antes de confirmar una migración crea un backup del archivo original, migra en memoria, valida una copia temporal y recién entonces reemplaza el master. Si una sección aislada está dañada, conserva los módulos válidos y aplica defaults sólo a la sección que no puede interpretar.

## Centro de backup y recuperación

En **Configuración → Backup y recuperación** se muestra la ruta del save, su validez y la lista de copias con fecha, schema y tamaño.

- **Crear backup ahora** copia únicamente configuración, metadata, posiciones, referencias y orden.
- **Restaurar** valida primero la copia y crea otro backup del estado actual antes del reemplazo atómico.
- **Exportar** guarda una copia portable con versión visible en el nombre.
- **Eliminar** borra solamente la copia seleccionada, nunca el master ni archivos de usuario.

Se conservan hasta ocho backups normales. Los saves corruptos apartados se guardan por separado para diagnóstico.

## Recuperar desde las carpetas físicas

1. Abrí el Administrador.
2. Elegí **Recuperar desde disco**.
3. La aplicación reconstruirá la representación visual a partir de las carpetas de `Cajones`.

El Dock se reconstruye automáticamente desde el primer nivel de `Dock - Accesos`. Si copiás toda esa carpeta a otro lugar, conservás una copia autónoma de tus accesos y de su orden.

Si una `.drawer.json` está dañada, se archiva con un nombre `.corrupt-*` y se reconstruye usando el nombre real de la carpeta. El contenido físico no se modifica.

## Importar un backup manual

1. En el Administrador elegí **Importar configuración**.
2. Seleccioná un JSON exportado previamente o una copia válida de `Backups`.
3. La importación se combina con el estado existente y no borra cajones ni archivos actuales.

Los JSON inválidos y esquemas futuros incompatibles son rechazados con un mensaje comprensible. Antes de importar se crea un backup del estado actual. Cajones, Dock y Paneles se reconstruyen desde la configuración importada sin mover archivos personales.

Si la configuración viene de otra PC, una ruta absoluta que allí no exista aparece como **No disponible** y la aplicación continúa. Los Cajones managed necesitan también sus carpetas físicas: el JSON no contiene esos archivos. Copiá aparte `Cajones` y, para conservar el respaldo físico real del Dock actual, `Dock - Accesos`.

## Empezar de nuevo sin borrar archivos

Cuando no existe save, **Empezar vacío** crea una organización nueva sin eliminar `Cajones`. En una instalación normal, **Restablecer configuración visual** crea primero un backup y restablece apariencia y posiciones; conserva el contenido físico, los accesos del Dock y todas las referencias de Paneles.

## Diagnóstico y logs

**Comprobar estado** revisa save, carpetas básicas, backups y configuración del updater. **Copiar diagnóstico** produce un texto de soporte sin tokens, secretos ni contenido de documentos y reemplaza el perfil del usuario por `%USERPROFILE%` cuando corresponde.

Los logs son locales, rotan al llegar a 1 MiB y mantienen como máximo cinco archivos. **Limpiar logs** afecta exclusivamente esos archivos. Desktop Organizer no envía telemetría ni logs a Internet.

## Después de reinstalar

No borres `Documentos\Desktop Organizer`. En particular, conservá tanto `Cajones` como `Dock - Accesos`. Instalá la aplicación y abrila normalmente:

- si existe el save, recuperará posiciones, apariencia, orden y preferencias;
- si falta el save pero existen carpetas, ofrecerá recuperación desde disco;
- siempre podés acceder a los archivos directamente desde el Explorador.

## Exportación de emergencia

Usá **Exportar configuración** y guardá el archivo versionado en otra unidad, nube o pendrive. Para respaldar el contenido, copiá también las carpetas `Cajones` y `Dock - Accesos`.
