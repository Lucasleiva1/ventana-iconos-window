import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
} from "react";
import { createPortal } from "react-dom";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useAppState } from "../../hooks/useAppState";
import { dragDropService } from "../../services/dragDropService";
import { panelApi } from "../../services/panelApi";
import type { PanelItem } from "../../types/panel";
import { hexToRgba } from "../../utils/color";
import { PanelItemView } from "./PanelItemView";
import { PANEL_GRID_GAP, usePanelLayout } from "./usePanelLayout";

interface PanelWindowProps {
  panelId: string;
}

const GEOMETRY_DEBOUNCE_MS = 420;
const DRAG_THRESHOLD_PX = 6;
const FEEDBACK_MS = 3200;

interface ContextMenuState {
  item: PanelItem;
  left: number;
  top: number;
}

export function PanelWindow({ panelId }: PanelWindowProps) {
  const { state, error } = useAppState();
  const panel = state?.panels.find((value) => value.id === panelId);

  const [feedback, setFeedback] = useState<string | null>(null);
  const [dropActive, setDropActive] = useState(false);
  const [menu, setMenu] = useState<ContextMenuState | null>(null);
  const [optionsOpen, setOptionsOpen] = useState(false);
  const [preview, setPreview] = useState<string[] | null>(null);
  const [dragId, setDragId] = useState<string | null>(null);
  const [dropTargetId, setDropTargetId] = useState<string | null>(null);

  const feedbackTimer = useRef<number | undefined>(undefined);
  const geometryTimer = useRef<number | undefined>(undefined);
  const menuRef = useRef<HTMLDivElement>(null);
  const gesture = useRef<{ id: string; pointerId: number; x: number; y: number; active: boolean } | null>(null);
  const previewRef = useRef<string[] | null>(null);
  const saving = useRef(false);

  const showFeedback = useCallback((message: string) => {
    window.clearTimeout(feedbackTimer.current);
    setFeedback(message);
    feedbackTimer.current = window.setTimeout(() => setFeedback(null), FEEDBACK_MS);
  }, []);

  const stored = [...(panel?.items ?? [])].sort((a, b) => a.order - b.order);
  const items = preview
    ? [...stored].sort((a, b) => preview.indexOf(a.id) - preview.indexOf(b.id))
    : stored;

  const { gridRef, layout } = usePanelLayout(items.length);

  // Guardado de geometría con debounce: mover o redimensionar no escribe al
  // save por cada píxel, sólo cuando el gesto se detiene.
  useEffect(() => {
    const appWindow = getCurrentWindow();
    let active = true;
    const unlisteners: Array<() => void> = [];

    const saveGeometry = () => {
      window.clearTimeout(geometryTimer.current);
      geometryTimer.current = window.setTimeout(() => {
        void Promise.all([
          appWindow.outerPosition(),
          appWindow.innerSize(),
          appWindow.scaleFactor(),
        ]).then(([position, size, scaleFactor]) =>
          panelApi.recordGeometry(
            panelId,
            position.x,
            position.y,
            size.width / scaleFactor,
            size.height / scaleFactor,
          ),
        ).catch(() => undefined);
      }, GEOMETRY_DEBOUNCE_MS);
    };

    void Promise.all([appWindow.onMoved(saveGeometry), appWindow.onResized(saveGeometry)])
      .then((listeners) => {
        if (active) unlisteners.push(...listeners);
        else listeners.forEach((stop) => stop());
      });

    return () => {
      active = false;
      window.clearTimeout(geometryTimer.current);
      unlisteners.forEach((stop) => stop());
    };
  }, [panelId]);

  // Arrastrar desde Windows agrega referencias. Nunca mueve el original.
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
        void panelApi.addItems(panelId, event.paths).then((result) => {
          const parts: string[] = [];
          if (result.added) {
            parts.push(`${result.added} ${result.added === 1 ? "acceso agregado" : "accesos agregados"}`);
          }
          if (result.duplicates) parts.push(`${result.duplicates} ya estaban`);
          if (result.failures.length) parts.push(`${result.failures.length} sin agregar`);
          showFeedback(parts.join(" · ") || "No había nada para agregar");
        }).catch((reason) => showFeedback(String(reason)));
      }
    }).then((stop) => {
      if (active) unlisten = stop;
      else stop();
    });

    return () => {
      active = false;
      unlisten?.();
      window.clearTimeout(feedbackTimer.current);
    };
  }, [panelId, showFeedback]);

  // Sin vigilantes de disco: la disponibilidad se revisa al abrir el Panel.
  useEffect(() => {
    void panelApi.refreshAvailability(panelId).catch(() => undefined);
  }, [panelId]);

  useEffect(() => {
    if (!menu && !optionsOpen) return;
    const close = () => {
      setMenu(null);
      setOptionsOpen(false);
    };
    window.addEventListener("pointerdown", close);
    window.addEventListener("blur", close);
    return () => {
      window.removeEventListener("pointerdown", close);
      window.removeEventListener("blur", close);
    };
  }, [menu, optionsOpen]);

  function openItem(item: PanelItem) {
    setMenu(null);
    void panelApi.openItem(panelId, item.id).catch((reason) => showFeedback(String(reason)));
  }

  async function runMenu(action: () => Promise<unknown>) {
    setMenu(null);
    setOptionsOpen(false);
    try {
      await action();
    } catch (reason) {
      showFeedback(String(reason));
    }
  }

  // --- Reordenamiento interno ---------------------------------------------

  function itemIdAt(clientX: number, clientY: number) {
    const cells = gridRef.current?.querySelectorAll<HTMLElement>("[data-panel-item]");
    if (!cells) return null;
    for (const cell of cells) {
      const bounds = cell.getBoundingClientRect();
      if (
        clientX >= bounds.left && clientX <= bounds.right
        && clientY >= bounds.top && clientY <= bounds.bottom
      ) {
        return cell.dataset.panelItem ?? null;
      }
    }
    return null;
  }

  function pointerDown(event: ReactPointerEvent<HTMLDivElement>) {
    if (event.button !== 0 || saving.current) return;
    const cell = (event.target as HTMLElement).closest<HTMLElement>("[data-panel-item]");
    if (!cell) return;
    gesture.current = {
      id: cell.dataset.panelItem!,
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
      if (Math.hypot(event.clientX - current.x, event.clientY - current.y) < DRAG_THRESHOLD_PX) {
        return;
      }
      current.active = true;
      event.currentTarget.setPointerCapture(event.pointerId);
      setDragId(current.id);
    }
    const target = itemIdAt(event.clientX, event.clientY);
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
    if (!wasDrag || !ids) {
      resetGesture();
      return;
    }
    saving.current = true;
    void panelApi.reorder(panelId, ids)
      .catch((reason) => showFeedback(String(reason)))
      .finally(() => {
        saving.current = false;
        resetGesture();
      });
  }

  if (error) return <div className="panel-fallback">{error}</div>;
  if (!state || !panel) return <div className="panel-fallback">Cargando…</div>;

  const otherDrawers = state.drawers.filter(
    (drawer) => !panel.items.some((item) => item.drawerId === drawer.id),
  );

  return (
    <main
      className={`panel-window ${panel.locked ? "is-locked" : ""}`}
      style={{
        "--panel-color": panel.color,
        "--panel-background": hexToRgba(panel.color, panel.opacity),
      } as React.CSSProperties}
    >
      <header
        className="panel-header"
        onPointerDown={(event) => {
          if (event.button !== 0) return;
          if ((event.target as HTMLElement).closest("button")) return;
          void panelApi.beginDrag().catch(() => undefined);
        }}
      >
        <span className="panel-title" title={panel.name}>{panel.name}</span>
        <span className="panel-count">{panel.items.length}</span>
        <button
          type="button"
          className={`panel-header-button ${panel.locked ? "is-active" : ""}`}
          title={panel.locked ? "Panel bloqueado: no se mueve ni se redimensiona" : "Bloquear panel"}
          aria-pressed={panel.locked}
          onClick={() => void runMenu(() => panelApi.update(panelId, { locked: !panel.locked }))}
        >
          {panel.locked ? "🔒" : "🔓"}
        </button>
        <button
          type="button"
          className="panel-header-button"
          title="Opciones del panel"
          aria-expanded={optionsOpen}
          onPointerDown={(event) => event.stopPropagation()}
          onClick={(event) => {
            event.stopPropagation();
            setOptionsOpen((open) => !open);
          }}
        >
          •••
        </button>
      </header>

      <section
        className={`panel-content ${layout.scroll ? "has-scroll" : ""}`}
        ref={gridRef}
        onPointerDown={pointerDown}
        onPointerMove={pointerMove}
        onPointerUp={pointerUp}
        onPointerCancel={resetGesture}
        onLostPointerCapture={() => { if (gesture.current) resetGesture(); }}
      >
        {items.length ? (
          <div
            className="panel-grid"
            style={{
              gridTemplateColumns: `repeat(auto-fill, minmax(${layout.cellWidth}px, 1fr))`,
              gap: `${PANEL_GRID_GAP}px`,
            }}
          >
            {items.map((item) => (
              <PanelItemView
                key={item.id}
                panelId={panelId}
                item={item}
                iconSize={layout.icon}
                labelMode={layout.labelMode}
                dragging={dragId === item.id}
                dropTarget={dropTargetId === item.id}
                onOpen={openItem}
                onContextMenu={(target, left, top) => setMenu({ item: target, left, top })}
              />
            ))}
          </div>
        ) : (
          <div className="panel-empty">
            <span className="panel-empty-mark" aria-hidden="true">◇</span>
            <span>
              Arrastrá programas, carpetas o archivos acá.
              Quedan como accesos: el original nunca se mueve.
            </span>
          </div>
        )}
      </section>

      {dropActive && <div className="panel-drop-overlay"><span>SOLTAR AQUÍ</span></div>}
      {feedback && <div className="panel-feedback" role="status">{feedback}</div>}

      {optionsOpen && (
        <div className="panel-options" onPointerDown={(event) => event.stopPropagation()}>
          <label className="panel-option-row">
            <span>Nombre</span>
            <input
              type="text"
              defaultValue={panel.name}
              maxLength={60}
              onBlur={(event) => {
                const next = event.target.value.trim();
                if (next && next !== panel.name) {
                  void runMenu(() => panelApi.update(panelId, { name: next }));
                }
              }}
            />
          </label>
          <label className="panel-option-row">
            <span>Color</span>
            <input
              type="color"
              value={panel.color}
              onChange={(event) => void panelApi.update(panelId, { color: event.target.value })
                .catch((reason) => showFeedback(String(reason)))}
            />
          </label>
          <label className="panel-option-row">
            <span>Opacidad · {Math.round(panel.opacity * 100)} %</span>
            <input
              type="range"
              min={35}
              max={100}
              value={Math.round(panel.opacity * 100)}
              onChange={(event) => void panelApi.update(panelId, {
                opacity: Number(event.target.value) / 100,
              }).catch((reason) => showFeedback(String(reason)))}
            />
          </label>
          {otherDrawers.length > 0 && (
            <div className="panel-option-row is-stack">
              <span>Agregar un cajón</span>
              <div className="panel-drawer-chips">
                {otherDrawers.map((drawer) => (
                  <button
                    key={drawer.id}
                    type="button"
                    onClick={() => void runMenu(async () => {
                      await panelApi.addDrawer(panelId, drawer.id);
                      showFeedback(`“${drawer.name}” agregado al panel.`);
                    })}
                  >
                    ▣ {drawer.name}
                  </button>
                ))}
              </div>
            </div>
          )}
          <button
            type="button"
            className="panel-option-action"
            onClick={() => void runMenu(() => panelApi.setHidden(panelId, true))}
          >
            Ocultar panel
          </button>
        </div>
      )}

      {menu && createPortal((
        <div
          ref={menuRef}
          className="item-context-menu"
          role="menu"
          style={{
            left: Math.min(menu.left, window.innerWidth - 236),
            top: Math.min(menu.top, window.innerHeight - 200),
          }}
          onPointerDown={(event) => event.stopPropagation()}
        >
          {menu.item.available ? (
            <>
              <button type="button" role="menuitem" onClick={() => openItem(menu.item)}>
                <strong>Abrir</strong>
                <span>{menu.item.drawerId ? "Trae el cajón al frente" : "Abre con Windows"}</span>
              </button>
              <button type="button" role="menuitem" onClick={() => void runMenu(() => panelApi.openItemLocation(panelId, menu.item.id))}>
                <strong>Abrir ubicación</strong>
                <span>{menu.item.path}</span>
              </button>
              <button type="button" role="menuitem" onClick={() => void runMenu(async () => {
                const next = window.prompt("Nombre visible en el panel", menu.item.displayName)?.trim();
                if (next && next !== menu.item.displayName) {
                  await panelApi.renameItem(panelId, menu.item.id, next);
                }
              })}>
                <strong>Renombrar en el panel</strong>
                <span>No cambia el archivo real</span>
              </button>
            </>
          ) : (
            <button type="button" role="menuitem" onClick={() => void runMenu(async () => {
              await panelApi.repairItem(panelId, menu.item.id);
              showFeedback("Acceso reparado.");
            })}>
              <strong>Buscar nueva ubicación</strong>
              <span>Actualiza sólo la referencia</span>
            </button>
          )}
          <button className="link-removal" type="button" role="menuitem" onClick={() => void runMenu(async () => {
            const removed = menu.item;
            await panelApi.removeItem(panelId, removed.id);
            panelApi.forgetIcon(removed.iconKey);
            showFeedback("Acceso quitado. El original no se tocó.");
          })}>
            <strong>Quitar del panel</strong>
            <span>Nunca borra ni mueve el original</span>
          </button>
        </div>
      ), document.body)}
    </main>
  );
}
