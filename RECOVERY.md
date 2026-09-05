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

Los JSON inválidos, esquemas futuros incompatibles y rutas inseguras son rechazados con un mensaje comprensible.

## Después de reinstalar

No borres `Documentos\Desktop Organizer`. En particular, conservá tanto `Cajones` como `Dock - Accesos`. Instalá la aplicación y abrila normalmente:

- si existe el save, recuperará posiciones, apariencia, orden y preferencias;
- si falta el save pero existen carpetas, ofrecerá recuperación desde disco;
- siempre podés acceder a los archivos directamente desde el Explorador.

## Exportación de emergencia

Usá **Exportar configuración** y guardá el archivo versionado en otra unidad, nube o pendrive. Para respaldar el contenido, copiá también las carpetas `Cajones` y `Dock - Accesos`.
