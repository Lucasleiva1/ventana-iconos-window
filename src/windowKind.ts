/**
 * Todas las ventanas cargan el mismo frontend y se diferencian por la query
 * string. Saber cuál es cada una desde el arranque permite que el CSS decida
 * el fondo sin tener que deducirlo del contenido ya renderizado: el
 * Administrador es la única ventana con fondo sólido, el resto deja pasar el
 * escritorio y su opacidad la define la barra de cada ventana.
 */
export type WindowKind = "admin" | "drawer" | "dock" | "dock-handle" | "panel";

export function resolveWindowKind(search = window.location.search): WindowKind {
  const params = new URLSearchParams(search);
  const view = params.get("view");
  if (view === "dock") return "dock";
  if (view === "dock-handle") return "dock-handle";
  if (params.get("panel")) return "panel";
  if (params.get("drawer")) return "drawer";
  return "admin";
}

/** Se aplica antes del primer render para que no haya un destello de fondo. */
export function markWindowKind(kind = resolveWindowKind()): WindowKind {
  document.documentElement.dataset.window = kind;
  return kind;
}
