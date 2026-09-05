import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { dockApi } from "../../services/dockApi";
import { dragDropService } from "../../services/dragDropService";
import type { DockItem, DockState } from "../../types/dock";
import { hexToRgba } from "../../utils/color";
import { DockItemView } from "./DockItemView";

/** Desplazamiento mínimo para que un clic pase a ser un arrastre de reorden. */
const DRAG_THRESHOLD_PX = 6;
const TOAST_MS = 2600;

const ICON_PIXELS = { small: 32, medium: 48, large: 64 } as const;
/** Debe coincidir con `DOCK_CELL_PADDING` del backend. */
const CELL_PADDING = 16;

export function DockWindow() {
  const [dock, setDock] = useState<DockState | null>(null);
  const [preview, setPreview] = useState<string[] | null>(null);
  const [dragId, setDragId] = useState<string | null>(null);
  const [dropTargetId, setDropTargetId] = useState<string | null>(null);
  const [dropActive, setDropActive] = useState(false);
  const [menuItem, setMenuItem] = useState<DockItem | null>(null);
  const [dockMenuOpen, setDockMenuOpen] = useState(false);
  const [scrollState, setScrollState] = useState({ left: false, right: false });
  const [toast, setToast] = useState<string | null>(null);
  const stripRef = useRef<HTMLDivElement>(null);
  const gesture = useRef<{ id: string; pointerId: number; x: number; y: number; active: boolean } | null>(null);
  const previewRef = useRef<string[] | null>(null);
  const saving = useRef(false);
  const toastTimer = useRef<number | undefined>(undefined);

  function showToast(message: string) {
    window.clearTimeout(toastTimer.current);
    setToast(message);
    toastTimer.current = window.setTimeout(() => setToast(null), TOAST_MS);
  }

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;

    void dockApi.getState().then((value) => {
      if (active) setDock(value);
    }).catch((reason) => showToast(String(reason)));

    void dockApi.onDockChanged((value) => {
      if (!active) return;
      setDock(value);
      if (!value.visible) setMenuItem(null);
    }).then((stop) => {
      if (active) unlisten = stop;
      else stop();
    });

    return () => {
      active = false;
      unlisten?.();
      window.clearTimeout(toastTimer.current);
    };
  }, []);

  function updateScrollState() {
    const strip = stripRef.current;
    if (!strip) {
      setScrollState({ left: false, right: false });
      return;
    }
    setScrollState({
      left: strip.scrollLeft > 1,
      right: strip.scrollLeft + strip.clientWidth < strip.scrollWidth - 1,
    });
  }

  useEffect(() => {
    const strip = stripRef.current;
    if (!strip) return;
    const observer = new ResizeObserver(updateScrollState);
    observer.observe(strip);
    for (const child of strip.children) observer.observe(child);
    updateScrollState();
    return () => observer.disconnect();
  }, [dock?.width, dock?.spacing, dock?.iconSize, dock?.items.length]);

  // Sin vigilantes en segundo plano: la disponibilidad se revisa al abrirse.
  useEffect(() => {
    if (!dock?.visible) return;
    void dockApi.refreshAvailability().catch(() => undefined);
  }, [dock?.visible]);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key !== "Escape") return;
      if (menuItem) {
        setMenuItem(null);
        return;
      }
      void dockApi.setVisible(false).catch(() => undefined);
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [menuItem]);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;

    void dragDropService.listen((event) => {
      if (!active) return;
      if (event.type === "enter" || event.type === "over") {
        setDropActive(true);
      } else if (event.type === "leave") {
        setDropActive(false);
      } else if (event.type === "drop") {
        setDropActive(false);
        void dockApi.addItems(event.paths).then((result) => {
          setDock(result.dock);
          const parts: string[] = [];
          if (result.added) parts.push(`${result.added} ${result.added === 1 ? "acceso agregado" : "accesos agregados"}`);
          if (result.duplicates) parts.push(`${result.duplicates} ya estaban`);
          if (result.failures.length) parts.push(`${result.failures.length} sin agregar`);
          showToast(parts.join(" · ") || "No había nada para agregar");
        }).catch((reason) => showToast(String(reason)));
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

  const stored = [...(dock?.items ?? [])].sort((a, b) => a.order - b.order);
  const items = preview
    ? [...stored].sort((a, b) => preview.indexOf(a.id) - preview.indexOf(b.id))
    : stored;

  function itemIdAt(clientX: number) {
    const cells = stripRef.current?.querySelectorAll<HTMLElement>("[data-dock-item]");
    if (!cells) return null;
    for (const cell of cells) {
      const bounds = cell.getBoundingClientRect();
      if (clientX >= bounds.left && clientX <= bounds.right) return cell.dataset.dockItem ?? null;
    }
    return null;
  }

  function pointerDown(event: ReactPointerEvent<HTMLDivElement>) {
    if (event.button !== 0 || saving.current) return;
    const cell = (event.target as HTMLElement).closest<HTMLElement>("[data-dock-item]");
    if (!cell) return;
    setMenuItem(null);
    setDockMenuOpen(false);
    gesture.current = {
      id: cell.dataset.dockItem!,
      pointerId: event.pointerId,
      x: event.clientX,
      y: event.clientY,
      active: false,
    };
    previewRef.current = stored.map((item) => item.id);
  }

  function pointerMove(event: ReactPointerEvent<HTMLDivElement>) {
    const current = gesture.current;
    if (!current || current.pointerId !== event.pointerId) return;
    if (!current.active) {
      if (Math.hypot(event.clientX - current.x, event.clientY - current.y) < DRAG_THRESHOLD_PX) return;
      current.active = true;
      event.currentTarget.setPointerCapture(event.pointerId);
      setDragId(current.id);
    }
    const target = itemIdAt(event.clientX);
    setDropTargetId(target && target !== current.id ? target : null);
    if (target && target !== current.id) {
      const ids = [...(previewRef.current ?? stored.map((item) => item.id))];
      const from = ids.indexOf(current.id);
      const to = ids.indexOf(target);
      if (from >= 0 && to >= 0 && from !== to) {
        ids.splice(from, 1);
        ids.splice(to, 0, current.id);
        previewRef.current = ids;
        setPreview(ids);
      }
    }
    event.preventDefault();
  }

  function resetGesture() {
    gesture.current = null;
    previewRef.current = null;
    setDragId(null);
    setDropTargetId(null);
    setPreview(null);
  }

  function pointerUp(event: ReactPointerEvent<HTMLDivElement>) {
    const current = gesture.current;
    if (!current || current.pointerId !== event.pointerId) return;
    const ids = previewRef.current;
    const wasDrag = current.active;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    // Un clic abre; un arrastre reordena. Nunca las dos cosas.
    if (!wasDrag) {
      const selected = stored.find((item) => item.id === current.id);
      resetGesture();
      if (selected?.kind !== "separator") openItem(current.id);
      return;
    }
    if (!ids) {
      resetGesture();
      return;
    }
    saving.current = true;
    void dockApi.reorder(ids)
      .then(setDock)
      .catch((reason) => showToast(String(reason)))
      .finally(() => {
        saving.current = false;
        resetGesture();
      });
  }

  function openItem(itemId: string) {
    void dockApi.openItem(itemId)
      .then(setDock)
      .catch((reason) => showToast(String(reason)));
  }

  async function runMenuAction(action: () => Promise<unknown>) {
    setMenuItem(null);
    setDockMenuOpen(false);
    try {
      await action();
    } catch (reason) {
      showToast(String(reason));
    }
  }

  const iconPixels = ICON_PIXELS[dock?.iconSize ?? "medium"];
  const background = hexToRgba(dock?.backgroundColor ?? "#10141E", dock?.opacity ?? 0.92);
  const effectsDisabled = dock?.performanceMode || dock?.animationMode === "disabled";

  return (
    <div
      className={`dock-shell ${dock?.visible ? "is-visible" : ""} ${dock?.blur && !dock.performanceMode ? "has-blur" : ""} ${dock?.performanceMode ? "is-performance" : ""} animation-${dock?.animationMode ?? "normal"}`}
      style={{
        "--dock-cell": `${iconPixels + CELL_PADDING}px`,
        "--dock-icon": `${iconPixels}px`,
        "--dock-background": background,
        "--dock-gap": `${dock?.spacing === "compact" ? 4 : dock?.spacing === "wide" ? 14 : 8}px`,
        "--dock-radius": `${dock?.borderRadius ?? 14}px`,
      } as React.CSSProperties}
    >
      <div
        className="dock-bar"
        onContextMenu={(event) => {
          if ((event.target as HTMLElement).closest("[data-dock-item]")) return;
          event.preventDefault();
          setMenuItem(null);
          setDockMenuOpen(true);
        }}
      >
        {items.length ? (
          <div
            className="dock-strip"
            ref={stripRef}
            onPointerDown={pointerDown}
            onPointerMove={pointerMove}
            onPointerUp={pointerUp}
            onPointerCancel={resetGesture}
            onLostPointerCapture={() => { if (gesture.current) resetGesture(); }}
            onScroll={updateScrollState}
            onWheel={(event) => {
              // La rueda desplaza la fila: con muchos iconos ninguno queda inaccesible.
              const strip = stripRef.current;
              if (!strip || strip.scrollWidth <= strip.clientWidth) return;
              event.preventDefault();
              strip.scrollLeft += event.deltaY || event.deltaX;
              requestAnimationFrame(updateScrollState);
            }}
            onKeyDown={(event) => {
              if (event.key !== "Enter") return;
              const cell = (event.target as HTMLElement).closest<HTMLElement>("[data-dock-item]");
              if (cell?.dataset.dockItem) openItem(cell.dataset.dockItem);
            }}
          >
            {items.map((item) => (
              <DockItemView
                key={item.id}
                item={item}
                dragging={dragId === item.id}
                dropTarget={dropTargetId === item.id}
                onContextMenu={setMenuItem}
              />
            ))}
          </div>
        ) : (
          <div className="dock-empty">
            <span className="dock-empty-mark" aria-hidden="true">◇</span>
            <span>Arrastrá programas, carpetas o archivos acá.</span>
          </div>
        )}

        {scrollState.left && !menuItem && !dockMenuOpen && (
          <button
            type="button"
            className="dock-scroll-arrow is-left"
            aria-label="Ver accesos anteriores"
            onClick={() => stripRef.current?.scrollBy({ left: -240, behavior: effectsDisabled ? "auto" : "smooth" })}
          >‹</button>
        )}
        {scrollState.right && !menuItem && !dockMenuOpen && (
          <button
            type="button"
            className="dock-scroll-arrow is-right"
            aria-label="Ver más accesos"
            onClick={() => stripRef.current?.scrollBy({ left: 240, behavior: effectsDisabled ? "auto" : "smooth" })}
          >›</button>
        )}

        {menuItem && (
          <div className="dock-menu" role="menu" aria-label={`Acciones de ${menuItem.displayName}`}>
            <span className="dock-menu-name" title={menuItem.path}>{menuItem.displayName}</span>
            {menuItem.kind !== "separator" && <>
              <button type="button" role="menuitem" onClick={() => void runMenuAction(async () => openItem(menuItem.id))}>
                Abrir
              </button>
              <button type="button" role="menuitem" onClick={() => void runMenuAction(() => dockApi.openItemLocation(menuItem.id))}>
                Abrir ubicación
              </button>
              <button type="button" role="menuitem" onClick={() => {
                const name = window.prompt("Nombre visual en el Dock", menuItem.displayName);
                if (name !== null) void runMenuAction(async () => setDock(await dockApi.renameItem(menuItem.id, name)));
              }}>
                Renombrar
              </button>
              {!menuItem.available && (
                <button type="button" role="menuitem" onClick={() => void runMenuAction(async () => {
                  dockApi.forgetIcon(menuItem.iconKey);
                  setDock(await dockApi.repairItem(menuItem.id));
                })}>
                  Buscar ubicación
                </button>
              )}
            </>}
            <button type="button" role="menuitem" className="dock-menu-remove" onClick={() => void runMenuAction(async () => {
              const next = await dockApi.removeItem(menuItem.id);
              dockApi.forgetIcon(menuItem.iconKey);
              setDock(next);
              showToast("Acceso quitado del Dock. El archivo original no se tocó.");
            })}>
              Quitar del Dock
            </button>
            <button type="button" className="dock-menu-close" aria-label="Cerrar menú" onClick={() => setMenuItem(null)}>
              ✕
            </button>
          </div>
        )}

        {dockMenuOpen && (
          <div className="dock-menu" role="menu" aria-label="Acciones del Dock">
            <span className="dock-menu-name">Dock</span>
            <button type="button" role="menuitem" onClick={() => void runMenuAction(async () => {
              setDock(await dockApi.addSeparator());
              showToast("Separador añadido.");
            })}>Añadir separador</button>
            <button type="button" role="menuitem" onClick={() => void runMenuAction(async () => {
              setDock(await dockApi.refreshAvailability());
              showToast("Dock actualizado.");
            })}>Actualizar Dock</button>
            <button type="button" className="dock-menu-close" aria-label="Cerrar menú" onClick={() => setDockMenuOpen(false)}>✕</button>
          </div>
        )}

        {dropActive && <div className="dock-drop-overlay"><span>SOLTAR AQUÍ</span></div>}
        {toast && <div className="dock-toast" role="status">{toast}</div>}
      </div>
    </div>
  );
}
