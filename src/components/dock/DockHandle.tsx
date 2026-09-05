import { useEffect, useState } from "react";
import { dockApi } from "../../services/dockApi";
import { dragDropService } from "../../services/dragDropService";
import type { DockState } from "../../types/dock";

/**
 * Tirador del Dock.
 *
 * Sólo el clic abre o cierra. No existe ningún manejador de `mouseenter`,
 * `pointerenter` ni `mouseover` que llame a `toggle`: el hover se resuelve
 * exclusivamente con CSS (`:hover`) y únicamente cambia opacidad y brillo.
 */
export function DockHandle() {
  const [dock, setDock] = useState<DockState | null>(null);
  const [busy, setBusy] = useState(false);
  const [dropHover, setDropHover] = useState(false);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;

    void dockApi.getState().then((dock) => {
      if (active) setDock(dock);
    }).catch(() => undefined);

    void dockApi.onDockChanged((dock) => {
      if (active) setDock(dock);
    }).then((stop) => {
      if (active) unlisten = stop;
      else stop();
    });

    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  // Soltar algo sobre el tirador también agrega accesos: si el Dock está
  // cerrado, el usuario no tiene por qué abrirlo primero para poder soltar.
  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;

    void dragDropService.listen((event) => {
      if (!active) return;
      if (event.type === "enter" || event.type === "over") {
        setDropHover(true);
      } else if (event.type === "leave") {
        setDropHover(false);
      } else if (event.type === "drop") {
        setDropHover(false);
        void dockApi.addItems(event.paths)
          .then(() => dockApi.setVisible(true))
          .catch((reason) => console.error("No se pudo agregar al Dock:", reason));
      }
    }).then((stop) => {
      if (active) unlisten = stop;
      else stop();
    });

    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  function toggle() {
    if (busy) return;
    setBusy(true);
    void dockApi.toggle()
      .catch((reason) => console.error("No se pudo alternar el Dock:", reason))
      .finally(() => setBusy(false));
  }

  return (
    <div
      className={`dock-handle-shell ${dock?.performanceMode ? "is-performance" : ""} animation-${dock?.animationMode ?? "normal"}`}
      style={{ "--dock-handle-opacity": dock?.handleOpacity ?? 0.42 } as React.CSSProperties}
    >
      <button
        type="button"
        className={`dock-handle ${dock?.visible ? "is-open" : ""} ${dropHover ? "is-drop-target" : ""}`}
        aria-label={dock?.visible ? "Ocultar el Dock" : "Mostrar el Dock"}
        aria-expanded={dock?.visible ?? false}
        title="Clic para mostrar u ocultar el Dock"
        onClick={toggle}
      >
        <span className="dock-handle-grip" aria-hidden="true" />
      </button>
    </div>
  );
}
