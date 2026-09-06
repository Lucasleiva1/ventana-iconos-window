/**
 * Funciones que existen en el código pero están fuera de la vista.
 *
 * No son restos: el código completo sigue acá y probado. Se ocultan porque hoy
 * no aportan, y volver a mostrarlas es cambiar `false` por `true` en un solo
 * lugar, sin tocar nada más.
 */

/**
 * Subcajones (carpetas dentro de un Cajón que se abren en la misma ventana en
 * lugar de mandarte al Explorador).
 *
 * Oculto desde el 6 de septiembre de 2026: no era un cajón dentro de otro, era
 * un nivel más de la misma ventana, y para eso conviene crear otro Cajón al
 * lado. Si algún día se implementan como cajones de verdad —ventana propia,
 * color y opacidad propios— esto vuelve a `true`.
 *
 * Con esto en `false` no se pueden crear ni convertir subcajones. Los que ya
 * existan en el disco se siguen abriendo con normalidad: nada se rompe ni se
 * pierde.
 */
export const SUBDRAWERS_ENABLED = false;

/**
 * Nombre del archivo debajo de cada icono, dentro del Cajon. Visible: es
 * necesario para saber que es cada cosa. La constante queda por si algun dia
 * se quiere una vista de solo iconos.
 */
export const DRAWER_ITEM_NAMES_VISIBLE = true;

/**
 * Etiqueta de modo de guardado ("GUARDADO" / "VINCULADO") debajo de cada
 * icono. Oculta el 6 de septiembre de 2026 por el mismo motivo: ocupaba lugar
 * y no cambiaba ninguna decisión de uso.
 */
export const DRAWER_STORAGE_BADGES_VISIBLE = false;

/**
 * Texto explicativo del Cajón vacío ("Arrastrá elementos acá..."). Oculto el
 * 6 de septiembre de 2026: ya no hace falta explicarlo cada vez. El rombo
 * sigue marcando que el nivel está vacío.
 */
export const DRAWER_EMPTY_HINT_VISIBLE = false;
