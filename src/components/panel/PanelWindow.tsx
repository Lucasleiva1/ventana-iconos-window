import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
} from "react";
import { createPortal } from "react-dom";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useAppState } from "../../hooks/useAppState";
import { dragDropService } from "../../services/dragDropService";
import { panelApi } from "../../services/panelApi";
import type { PanelItem, PanelPatch } from "../../types/panel";
import { hexToRgba } from "../../utils/color";
import { PanelItemView } from "./PanelItemView";
import { PanelOptionsMenu } from "./PanelOptionsMenu";
import { usePanelLayout } from "./usePanelLayout";

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
  const [selection, setSelection] = useState<string[]>([]);

  const feedbackTimer = useRef<number | undefined>(undefined);
  const geometryTimer = useRef<number | undefined>(undefined);
  const gesture = useRef<{ id: string; pointerId: number; x: number; y: number; active: boolean } | null>(null);
  const previewRef = useRef<string[] | null>(null);
  const saving = useRef(false);
  const lastSelected = useRef<string | null>(null);

  const showFeedback = useCallback((message: string) => {
    window.clearTimeout(feedbackTimer.current);
    setFeedback(message);
    feedbackTimer.current = window.setTimeout(() => setFeedback(null), FEEDBACK_MS);
  }, []);

  const stored = useMemo(
    () => [...(panel?.items ?? [])].sort((a, b) => a.order - b.order),
    [panel?.items],
  );
  const items = useMemo(
    () => (preview
      ? [...stored].sort((a, b) => preview.indexOf(a.id) - preview.indexOf(b.id))
      : stored),
    [stored, preview],
  );

  const { gridRef, layout, virtual, onScroll } = usePanelLayout({
    count: items.length,
    density: panel?.density ?? "normal",
    mode: panel?.iconMode ?? "auto",
    manualIcon: panel?.manualIconSize ?? 48,
  });

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
    window.addEventListener("blur", close);
    return () => window.removeEventListener("blur", close);
  }, [menu, optionsOpen]);

  const removeSelection = useCallback(() => {
    if (!selection.length) return;
    const ids = [...selection];
    void panelApi.removeItems(panelId, ids)
      .then(() => {
        setSelection([]);
        showFeedback(`${ids.length} ${ids.length === 1 ? "acceso quitado" : "accesos quitados"}. Los originales no se tocaron.`);
      })
      .catch((reason) => showFeedback(String(reason)));
  }, [panelId, selection, showFeedback]);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setMenu(null);
        setOptionsOpen(false);
        setSelection([]);
        return;
      }
      if (event.key === "Delete" && selection.length) {
        event.preventDefault();
        removeSelection();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [selection, removeSelection]);

  function openItem(item: PanelItem) {
    setMenu(null);
    void panelApi.openItem(panelId, item.id).catch((reason) => showFeedback(String(reason)));
  }

  async function runAction(action: () => Promise<unknown>, message?: string) {
    setMenu(null);
    setOptionsOpen(false);
    try {
      await action();
      if (message) showFeedback(message);
    } catch (reason) {
      showFeedback(String(reason));
    }
  }

  function patchPanel(patch: PanelPatch) {
    void panelApi.update(panelId, patch).catch((reason) => showFeedback(String(reason)));
  }

  // --- Selección múltiple ---------------------------------------------------

  function updateSelection(id: string, event: ReactPointerEvent<HTMLDivElement>) {
    if (event.ctrlKey || event.metaKey) {
      setSelection((current) => current.includes(id)
        ? current.filter((value) => value !== id)
        : [...current, id]);
      lastSelected.current = id;
      return true;
    }
    if (event.shiftKey && lastSelected.current) {
      const ids = items.map((item) => item.id);
      const from = ids.indexOf(lastSelected.current);
      const to = ids.indexOf(id);
      if (from >= 0 && to >= 0) {
        const [start, end] = from <= to ? [from, to] : [to, from];
        setSelection(ids.slice(start, end + 1));
        return true;
      }
    }
    lastSelected.current = id;
    setSelection([id]);
    return false;
  }

  // --- Reordenamiento interno ----------------------------------------------

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
    if (!cell) {
      setSelection([]);
      setMenu(null);
      return;
    }
    const id = cell.dataset.panelItem!;
    const onlySelected = updateSelection(id, event);
    // Con Ctrl o Shift el gesto es de selección, no de arrastre.
    if (onlySelected || panel?.lockContent) return;
    gesture.current = {
      id,
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

  const visible = items.slice(virtual.from, virtual.to);
  // La barra de opacidad manda sola: el estilo de fondo ya no recorta el valor
  // elegido, sólo describe el aspecto del panel.
  const background = hexToRgba(panel.color, panel.opacity);

  return (
    <main
      className={[
        "panel-window",
        panel.locked ? "is-locked" : "",
        `bg-${panel.backgroundStyle}`,
        `header-${panel.headerMode}`,
      ].filter(Boolean).join(" ")}
      style={{
        "--panel-color": panel.color,
        "--panel-background": background,
      } as React.CSSProperties}
      onPointerDown={() => {
        if (menu) setMenu(null);
      }}
    >
      <header
        className="panel-header"
        onPointerDown={(event) => {
          if (event.button !== 0) return;
          if ((event.target as HTMLElement).closest("button")) return;
          // Alt mantenido mueve libre aunque el imantado esté activo.
          void panelApi.beginDrag(event.altKey).catch(() => undefined);
        }}
        onDoubleClick={(event) => {
          if ((event.target as HTMLElement).closest("button")) return;
          if (panel.locked) return;
          void runAction(() => panelApi.setExpanded(panelId, !panel.expanded));
        }}
      >
        {panel.showTitle && (
          <span className="panel-title" title={panel.name}>{panel.name}</span>
        )}
        {!panel.showTitle && <span className="panel-grip" aria-hidden="true">⋮⋮</span>}
        <span className="panel-count">
          {selection.length ? `${selection.length}/${panel.items.length}` : panel.items.length}
        </span>
        {(panel.locked || panel.lockContent) && (
          <span
            className="panel-lock-mark"
            title={[
              panel.locked ? "Posición bloqueada" : "",
              panel.lockContent ? "Contenido bloqueado" : "",
            ].filter(Boolean).join(" · ")}
          >
            🔒
          </span>
        )}
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
        style={{ padding: layout.metrics.padding }}
        onScroll={onScroll}
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
              gap: `${layout.metrics.gap}px`,
              paddingTop: virtual.padTop,
              paddingBottom: virtual.padBottom,
            }}
          >
            {visible.map((item) => (
              <PanelItemView
                key={item.id}
                panelId={panelId}
                item={item}
                iconSize={layout.icon}
                labelMode={layout.labelMode}
                dragging={dragId === item.id}
                dropTarget={dropTargetId === item.id}
                selected={selection.includes(item.id)}
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
        <PanelOptionsMenu
          panel={panel}
          drawers={state.drawers}
          onPatch={patchPanel}
          onClose={() => setOptionsOpen(false)}
          onRename={() => void runAction(async () => {
            const name = window.prompt("Nuevo nombre del panel", panel.name)?.trim();
            if (name && name !== panel.name) await panelApi.update(panelId, { name });
          })}
          onDuplicate={() => void runAction(
            () => panelApi.duplicate(panelId),
            "Panel duplicado. Se copiaron sólo las referencias.",
          )}
          onAddDrawer={(drawerId) => void runAction(
            () => panelApi.addDrawer(panelId, drawerId),
            "Cajón agregado al panel.",
          )}
          onToggleExpanded={() => void runAction(
            () => panelApi.setExpanded(panelId, !panel.expanded),
          )}
          onRefresh={() => void runAction(() => panelApi.refresh(panelId), "Panel actualizado.")}
          onHide={() => void runAction(() => panelApi.setHidden(panelId, true))}
          onDelete={() => void runAction(async () => {
            const confirmed = window.confirm(
              `¿Eliminar el panel “${panel.name}”?\n\nSe borran sólo el panel y sus accesos. Ningún archivo real se toca.`,
            );
            if (confirmed) await panelApi.remove(panelId);
          })}
        />
      )}

      {menu && createPortal((
        <div
          className="item-context-menu"
          role="menu"
          style={{
            left: Math.min(menu.left, window.innerWidth - 236),
            top: Math.min(menu.top, window.innerHeight - 210),
          }}
          onPointerDown={(event) => event.stopPropagation()}
        >
          {menu.item.available ? (
            <>
              <button type="button" role="menuitem" onClick={() => openItem(menu.item)}>
                <strong>Abrir</strong>
                <span>{menu.item.drawerId ? "Trae el cajón al frente" : "Abre con Windows"}</span>
              </button>
              <button type="button" role="menuitem" onClick={() => void runAction(() => panelApi.openItemLocation(panelId, menu.item.id))}>
                <strong>Abrir ubicación</strong>
                <span>{menu.item.path}</span>
              </button>
              <button type="button" role="menuitem" disabled={panel.lockContent} onClick={() => void runAction(async () => {
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
            <button type="button" role="menuitem" onClick={() => void runAction(
              () => panelApi.repairItem(panelId, menu.item.id),
              "Acceso reparado.",
            )}>
              <strong>Buscar nueva ubicación</strong>
              <span>{menu.item.drawerId ? "Ese cajón ya no existe" : "Actualiza sólo la referencia"}</span>
            </button>
          )}
          {selection.length > 1 && selection.includes(menu.item.id) ? (
            <button className="link-removal" type="button" role="menuitem" disabled={panel.lockContent} onClick={() => { setMenu(null); removeSelection(); }}>
              <strong>Quitar {selection.length} seleccionados</strong>
              <span>Nunca borra ni mueve los originales</span>
            </button>
          ) : (
            <button className="link-removal" type="button" role="menuitem" disabled={panel.lockContent} onClick={() => void runAction(async () => {
              const removed = menu.item;
              await panelApi.removeItem(panelId, removed.id);
              panelApi.forgetIcon(removed.iconKey);
              showFeedback("Acceso quitado. El original no se tocó.");
            })}>
              <strong>Quitar del panel</strong>
              <span>Nunca borra ni mueve el original</span>
            </button>
          )}
        </div>
      ), document.body)}
    </main>
  );
}
