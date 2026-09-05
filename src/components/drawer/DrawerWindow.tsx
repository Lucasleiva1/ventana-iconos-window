import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useAppState } from "../../hooks/useAppState";
import { drawerApi } from "../../services/drawerApi";
import { dragDropService } from "../../services/dragDropService";
import type { DrawerLevel } from "../../types/drawer";
import { hexToRgba } from "../../utils/color";
import { DrawerHeader } from "./DrawerHeader";
import { DrawerItemGrid } from "./DrawerItemGrid";
import { DrawerSettings } from "./DrawerSettings";

interface DrawerWindowProps {
  drawerId: string;
}

const GEOMETRY_DEBOUNCE_MS = 420;

export function DrawerWindow({ drawerId }: DrawerWindowProps) {
  const { state, error } = useAppState();
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [dropActive, setDropActive] = useState(false);
  const [feedback, setFeedback] = useState<string | null>(null);
  const [relativePath, setRelativePath] = useState("");
  const [level, setLevel] = useState<DrawerLevel | null>(null);
  const [levelError, setLevelError] = useState<string | null>(null);
  const geometryTimer = useRef<number | undefined>(undefined);
  const feedbackTimer = useRef<number | undefined>(undefined);
  const drawer = state?.drawers.find((item) => item.id === drawerId);

  function showFeedback(message: string) {
    window.clearTimeout(feedbackTimer.current);
    setFeedback(message);
    feedbackTimer.current = window.setTimeout(() => setFeedback(null), 3600);
  }

  const loadLevel = useCallback(async (nextPath = relativePath) => {
    try {
      const loaded = await drawerApi.loadLevel(drawerId, nextPath);
      setLevel(loaded);
      setRelativePath(loaded.relativePath);
      setLevelError(null);
    } catch (reason) {
      if (nextPath) {
        setRelativePath("");
        const root = await drawerApi.loadLevel(drawerId, "");
        setLevel(root);
        setLevelError("Ese nivel ya no existe. Volvimos al cajón principal.");
      } else {
        setLevelError(String(reason));
      }
    }
  }, [drawerId, relativePath]);

  useEffect(() => {
    void loadLevel(relativePath);
  }, [drawer?.updatedAt, drawerId]);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    void drawerApi.onFeedback((message) => {
      showFeedback(message);
      void loadLevel(relativePath);
    }).then((stopListening) => {
      if (active) unlisten = stopListening;
      else stopListening();
    });
    return () => {
      active = false;
      unlisten?.();
    };
  }, [drawerId, loadLevel, relativePath]);

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
          drawerApi.recordGeometry(
            drawerId,
            position.x,
            position.y,
            size.width / scaleFactor,
            size.height / scaleFactor,
          ),
        ).catch(() => undefined);
      }, GEOMETRY_DEBOUNCE_MS);
    };

    void Promise.all([appWindow.onMoved(saveGeometry), appWindow.onResized(saveGeometry)]).then(
      (listeners) => {
        if (active) unlisteners.push(...listeners);
        else listeners.forEach((unlisten) => unlisten());
      },
    );

    return () => {
      active = false;
      window.clearTimeout(geometryTimer.current);
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [drawerId]);

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
        void drawerApi.addItemsAtLevel(drawerId, relativePath, event.paths).then((result) => {
          void loadLevel(relativePath);
          if (!result.added.length && result.duplicates.length && !result.failures.length) {
            showFeedback(result.duplicates.length === 1
              ? "Ese elemento ya estaba en este nivel"
              : "Esos elementos ya estaban en este nivel");
            return;
          }
          const details: string[] = [];
          if (result.added.length) details.push(`${result.added.length} agregados`);
          if (result.duplicates.length) details.push(`${result.duplicates.length} duplicados`);
          if (result.failures.length) details.push(`${result.failures.length} sin agregar`);
          if (result.warnings.length) details.push(result.warnings.join(" · "));
          showFeedback(details.join(" · ") || "No había elementos para agregar");
        }).catch((reason) => showFeedback(String(reason)));
      }
    }).then((stopListening) => {
      if (active) unlisten = stopListening;
      else stopListening();
    });

    void drawerApi.refreshLevel(drawerId, relativePath)
      .then(setLevel)
      .catch(() => undefined);

    return () => {
      active = false;
      unlisten?.();
      window.clearTimeout(feedbackTimer.current);
    };
  }, [drawerId, loadLevel, relativePath]);

  async function createSubdrawer() {
    const name = window.prompt("Nombre del nuevo subcajón")?.trim();
    if (!name) return;
    try {
      setLevel(await drawerApi.createSubdrawer(drawerId, relativePath, name));
      showFeedback(`Subcajón “${name}” creado.`);
    } catch (reason) {
      showFeedback(String(reason));
    }
  }

  if (error) return <div className="drawer-fallback">{error}</div>;
  if (!state || !drawer || !level) return <div className="drawer-fallback">Cargando…</div>;

  return (
    <main
      className={`drawer-window ${drawer.collapsed ? "is-collapsed" : ""}`}
      style={{
        "--drawer-color": drawer.color,
        "--drawer-background": hexToRgba(drawer.color, drawer.opacity),
      } as React.CSSProperties}
    >
      <DrawerHeader
        drawer={drawer}
        itemCount={level.items.length}
        settingsOpen={settingsOpen}
        onToggleSettings={() => {
          if (drawer.collapsed) {
            void drawerApi.setCollapsed(drawer.id, false).then(() => setSettingsOpen(true));
          } else {
            setSettingsOpen((open) => !open);
          }
        }}
      />
      {!drawer.collapsed && (
        <section className="drawer-content">
          <nav className="drawer-level-toolbar" aria-label="Ruta del subcajón">
            <button
              type="button"
              className="level-back"
              disabled={!level.relativePath}
              onClick={() => {
                const parent = level.breadcrumbs.at(-2)?.relativePath ?? "";
                void loadLevel(parent);
              }}
              aria-label="Volver al nivel anterior"
            >
              ‹
            </button>
            <div className="drawer-breadcrumbs">
              {level.breadcrumbs.map((crumb, index) => (
                <span key={`${crumb.relativePath}-${index}`}>
                  {index > 0 && <i>/</i>}
                  <button type="button" onClick={() => void loadLevel(crumb.relativePath)}>
                    {crumb.name}
                  </button>
                </span>
              ))}
            </div>
            <button type="button" className="level-action" onClick={() => void createSubdrawer()} title="Nuevo subcajón">+▣</button>
            <button type="button" className="level-action" onClick={() => void drawerApi.refreshLevel(drawerId, relativePath).then(setLevel).catch((reason) => showFeedback(String(reason)))} title="Actualizar nivel">↻</button>
          </nav>
          {levelError && <div className="level-error">{levelError}</div>}
          {level.items.length ? (
            <DrawerItemGrid
              drawer={drawer}
              drawers={state.drawers}
              level={level}
              onNavigate={(path) => void loadLevel(path)}
              onLevelChanged={(updated) => updated ? setLevel(updated) : void loadLevel(relativePath)}
              onFeedback={showFeedback}
            />
          ) : (
            <div className="drawer-empty-state">
              <span className="empty-diamond" aria-hidden="true">◇</span>
              <span>Arrastrá elementos acá. Los del Escritorio se guardan físicamente; los demás quedan vinculados.</span>
            </div>
          )}
          {dropActive && (
            <div className="drawer-drop-overlay"><span>SOLTAR AQUÍ</span></div>
          )}
          {feedback && <div className="drawer-feedback" role="status">{feedback}</div>}
        </section>
      )}
      {settingsOpen && (
        <DrawerSettings drawer={drawer} onClose={() => setSettingsOpen(false)} />
      )}
    </main>
  );
}
