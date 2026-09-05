# Recuperación de Desktop Organizer

Los archivos reales no dependen de que Desktop Organizer funcione. Aunque la aplicación falle o sea desinstalada, pueden abrirse normalmente desde el Explorador de Windows.

## Dónde están los cajones

Desktop Organizer consulta la carpeta **Documentos** configurada por Windows. Desde la aplicación, usá **Abrir carpeta Cajones** o **Copiar ruta** para obtener la ubicación exacta.

La estructura habitual es:

```text
Documentos\Desktop Organizer\
├── Cajones\                 # archivos y carpetas reales
├── Backups\                 # copias recientes del save
└── desktop-organizer-save.json
```

No presupongas que Documentos está en `C:\Users\...`: puede estar redirigido a OneDrive, una red u otra unidad.

## Recuperar sin la aplicación

1. Abrí el Explorador de Windows.
2. Entrá en **Documentos → Desktop Organizer → Cajones**.
3. Cada carpeta de primer nivel es un cajón físico y su contenido puede copiarse o abrirse normalmente.
4. Los archivos `.drawer.json` son metadata interna oculta; no contienen documentos del usuario.

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

Si una `.drawer.json` está dañada, se archiva con un nombre `.corrupt-*` y se reconstruye usando el nombre real de la carpeta. El contenido físico no se modifica.

## Importar un backup manual

1. En el Administrador elegí **Importar configuración**.
2. Seleccioná un JSON exportado previamente o una copia válida de `Backups`.
3. La importación se combina con el estado existente y no borra cajones ni archivos actuales.

Los JSON inválidos, esquemas futuros incompatibles y rutas inseguras son rechazados con un mensaje comprensible.

## Después de reinstalar

No borres `Documentos\Desktop Organizer`. Instalá la aplicación y abrila normalmente:

- si existe el save, recuperará posiciones, apariencia, orden y preferencias;
- si falta el save pero existen carpetas, ofrecerá recuperación desde disco;
- siempre podés acceder a los archivos directamente desde el Explorador.

## Exportación de emergencia

Usá **Exportar configuración** y guardá el archivo versionado en otra unidad, nube o pendrive. La exportación contiene configuración y referencias; no contiene los archivos reales de los cajones.
