import { useEffect, useState } from "react";
import { dockApi } from "../../services/dockApi";
import type { DockState } from "../../types/dock";
import type { DrawerIconSize, MonitorInfo } from "../../types/drawer";

interface DockSettingsProps {
  monitors: MonitorInfo[];
  onError: (message: string | null) => void;
  onMessage: (message: string) => void;
}

const ICON_SIZES: Array<{ value: DrawerIconSize; label: string }> = [
  { value: "small", label: "Pequeño (32 px)" },
  { value: "medium", label: "Mediano (48 px)" },
  { value: "large", label: "Grande (64 px)" },
];

export function DockSettings({ monitors, onError, onMessage }: DockSettingsProps) {
  const [dock, setDock] = useState<DockState | null>(null);
  const [shortcutDraft, setShortcutDraft] = useState("");

  useEffect(() => {
    if (dock) setShortcutDraft(dock.shortcut);
  }, [dock?.shortcut]);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;

    // Reubicar al abrir el Administrador recupera el tirador si cambió la
    // resolución, el DPI o la cantidad de monitores mientras la app corría.
    void dockApi.relayout().then((value) => {
      if (active) setDock(value);
    }).catch(() => dockApi.getState().then((value) => {
      if (active) setDock(value);
    })).catch((reason) => onError(`No se pudo leer el estado del Dock: ${String(reason)}`));

    void dockApi.onDockChanged((value) => {
      if (active) setDock(value);
    }).then((stop) => {
      if (active) unlisten = stop;
      else stop();
    });

    return () => {
      active = false;
      unlisten?.();
    };
  }, [onError]);

  async function run(action: () => Promise<unknown>, message?: string) {
    onError(null);
    try {
      await action();
      if (message) onMessage(message);
    } catch (reason) {
      onError(String(reason));
    }
  }

  const monitorId = dock?.monitorId ?? "";
  const knownMonitor = monitors.some((monitor) => monitor.id === monitorId);

  return (
    <section className="dock-settings-panel" aria-labelledby="dock-settings-heading">
      <div>
        <p className="eyebrow">DOCK</p>
        <h2 id="dock-settings-heading">Barra retráctil</h2>
        <p>
          El Dock es un lanzador: lo que arrastres queda como acceso y el archivo original
          nunca se mueve. Se abre sólo con un clic en el tirador, nunca al pasar el mouse.
        </p>
      </div>

      <div className="preference-list">
        <label className="preference-row">
          <span>
            <strong>Activar Dock</strong>
            <small>Muestra el tirador en el borde inferior del monitor elegido.</small>
          </span>
          <input
            type="checkbox"
            checked={dock?.enabled ?? false}
            disabled={!dock}
            onChange={(event) => void run(
              async () => setDock(await dockApi.updateSettings({ enabled: event.target.checked })),
              event.target.checked ? "Dock activado." : "Dock desactivado.",
            )}
          />
        </label>

        <label className="preference-row">
          <span>
            <strong>Ancho del Dock</strong>
            <small>Automático sigue el contenido; manual respeta el área útil del monitor.</small>
          </span>
          <select
            value={dock?.widthMode ?? "automatic"}
            disabled={!dock}
            onChange={(event) => void run(async () => setDock(await dockApi.updateSettings({
              widthMode: event.target.value as DockState["widthMode"],
            })), "Modo de ancho guardado.")}
          >
            <option value="automatic">Automático</option>
            <option value="manual">Manual</option>
          </select>
        </label>

        {dock?.widthMode === "manual" && (
          <label className="preference-row">
            <span><strong>Ancho manual</strong><small>{Math.round(dock.manualWidth)} px lógicos</small></span>
            <input type="range" min={280} max={1600} step={10} value={dock.manualWidth}
              onChange={(event) => void run(async () => setDock(await dockApi.updateSettings({ manualWidth: Number(event.target.value) })))} />
          </label>
        )}

        <label className="preference-row">
          <span><strong>Espaciado</strong><small>Distancia entre iconos y separadores.</small></span>
          <select value={dock?.spacing ?? "normal"} disabled={!dock}
            onChange={(event) => void run(async () => setDock(await dockApi.updateSettings({
              spacing: event.target.value as DockState["spacing"],
            })), "Espaciado guardado.")}>
            <option value="compact">Compacto</option>
            <option value="normal">Normal</option>
            <option value="wide">Amplio</option>
          </select>
        </label>

        <label className="preference-row">
          <span><strong>Posición del tirador</strong><small>Dentro del área útil del monitor.</small></span>
          <select value={dock?.handlePosition ?? "center"} disabled={!dock}
            onChange={(event) => void run(async () => setDock(await dockApi.updateSettings({
              handlePosition: event.target.value as DockState["handlePosition"],
            })), "Posición del tirador guardada.")}>
            <option value="left">Izquierda</option>
            <option value="center">Centro</option>
            <option value="right">Derecha</option>
          </select>
        </label>

        <label className="preference-row">
          <span><strong>Ajuste fino del tirador</strong><small>{Math.round(dock?.handleOffset ?? 0)} px lógicos</small></span>
          <input type="range" min={-400} max={400} step={5} value={dock?.handleOffset ?? 0} disabled={!dock}
            onChange={(event) => void run(async () => setDock(await dockApi.updateSettings({ handleOffset: Number(event.target.value) })))} />
        </label>

        <label className="preference-row">
          <span><strong>Ancho del tirador</strong><small>{Math.round(dock?.handleWidth ?? 58)} px</small></span>
          <input type="range" min={36} max={160} step={2} value={dock?.handleWidth ?? 58} disabled={!dock}
            onChange={(event) => void run(async () => setDock(await dockApi.updateSettings({ handleWidth: Number(event.target.value) })))} />
        </label>

        <label className="preference-row">
          <span><strong>Alto del tirador</strong><small>{Math.round(dock?.handleHeight ?? 14)} px</small></span>
          <input type="range" min={8} max={36} step={1} value={dock?.handleHeight ?? 14} disabled={!dock}
            onChange={(event) => void run(async () => setDock(await dockApi.updateSettings({ handleHeight: Number(event.target.value) })))} />
        </label>

        <label className="preference-row">
          <span><strong>Opacidad del tirador</strong><small>{Math.round((dock?.handleOpacity ?? 0.42) * 100)} %</small></span>
          <input type="range" min={20} max={100} step={1} value={Math.round((dock?.handleOpacity ?? 0.42) * 100)} disabled={!dock}
            onChange={(event) => void run(async () => setDock(await dockApi.updateSettings({ handleOpacity: Number(event.target.value) / 100 })))} />
        </label>

        <label className="preference-row">
          <span><strong>Color del Dock</strong><small>{dock?.backgroundColor ?? "#10141E"}</small></span>
          <input type="color" value={dock?.backgroundColor ?? "#10141E"} disabled={!dock}
            onChange={(event) => void run(async () => setDock(await dockApi.updateSettings({ backgroundColor: event.target.value })))} />
        </label>

        <label className="preference-row">
          <span><strong>Radio de borde</strong><small>{Math.round(dock?.borderRadius ?? 14)} px</small></span>
          <input type="range" min={0} max={32} step={1} value={dock?.borderRadius ?? 14} disabled={!dock}
            onChange={(event) => void run(async () => setDock(await dockApi.updateSettings({ borderRadius: Number(event.target.value) })))} />
        </label>

        <label className="preference-row">
          <span><strong>Animaciones</strong><small>Siempre breves; nunca ralentizan la apertura.</small></span>
          <select value={dock?.animationMode ?? "normal"} disabled={!dock || dock.performanceMode}
            onChange={(event) => void run(async () => setDock(await dockApi.updateSettings({
              animationMode: event.target.value as DockState["animationMode"],
            })), "Animaciones guardadas.")}>
            <option value="normal">Normales</option>
            <option value="reduced">Reducidas</option>
            <option value="disabled">Desactivadas</option>
          </select>
        </label>

        <label className="preference-row">
          <span><strong>Blur</strong><small>Desactivado por defecto; el modo rendimiento siempre lo apaga.</small></span>
          <input type="checkbox" checked={dock?.blur ?? false} disabled={!dock || dock.performanceMode}
            onChange={(event) => void run(async () => setDock(await dockApi.updateSettings({ blur: event.target.checked })), "Efecto visual guardado.")} />
        </label>

        <label className="preference-row">
          <span><strong>Modo rendimiento</strong><small>Sin blur, sombras reducidas y animación mínima.</small></span>
          <input type="checkbox" checked={dock?.performanceMode ?? false} disabled={!dock}
            onChange={(event) => void run(async () => setDock(await dockApi.updateSettings({ performanceMode: event.target.checked })), "Modo rendimiento actualizado.")} />
        </label>

        <div className="preference-row dock-shortcut-row">
          <span><strong>Shortcut global</strong><small>Ejemplo: CommandOrControl+Alt+D. Si está ocupado, se informa y se conserva el anterior.</small></span>
          <div className="dock-shortcut-controls">
            <input aria-label="Combinación del shortcut del Dock" value={shortcutDraft} disabled={!dock}
              onChange={(event) => setShortcutDraft(event.target.value)} />
            <button type="button" className="button button-ghost" disabled={!dock}
              onClick={() => void run(async () => setDock(await dockApi.updateSettings({
                shortcut: shortcutDraft,
                shortcutEnabled: dock?.shortcutEnabled ?? false,
              })), "Shortcut guardado.")}>Guardar</button>
            <input aria-label="Activar shortcut global" type="checkbox" checked={dock?.shortcutEnabled ?? false} disabled={!dock}
              onChange={(event) => void run(async () => setDock(await dockApi.updateSettings({
                shortcut: shortcutDraft,
                shortcutEnabled: event.target.checked,
              })), event.target.checked ? "Shortcut activado." : "Shortcut desactivado.")} />
          </div>
        </div>

        <label className="preference-row">
          <span>
            <strong>Ocultar Dock después de abrir un elemento</strong>
            <small>Recomendado: el Dock se aparta solo apenas lanzás algo.</small>
          </span>
          <input
            type="checkbox"
            checked={dock?.hideAfterOpen ?? true}
            disabled={!dock}
            onChange={(event) => void run(
              async () => setDock(await dockApi.updateSettings({ hideAfterOpen: event.target.checked })),
              "Preferencia guardada.",
            )}
          />
        </label>

        <label className="preference-row">
          <span>
            <strong>Monitor</strong>
            <small>Si ese monitor se desconecta, el Dock vuelve solo al principal.</small>
          </span>
          <select
            value={knownMonitor ? monitorId : ""}
            disabled={!dock || !monitors.length}
            onChange={(event) => void run(
              async () => setDock(await dockApi.updateSettings({ monitorId: event.target.value })),
              "Dock movido de monitor.",
            )}
          >
            {!knownMonitor && <option value="">Monitor principal</option>}
            {monitors.map((monitor) => (
              <option key={monitor.id} value={monitor.id}>
                {monitor.name}{monitor.primary ? " (principal)" : ""} · {monitor.size.width}×{monitor.size.height}
              </option>
            ))}
          </select>
        </label>

        <label className="preference-row">
          <span>
            <strong>Tamaño de iconos</strong>
            <small>El alto del Dock se recalcula solo.</small>
          </span>
          <select
            value={dock?.iconSize ?? "medium"}
            disabled={!dock}
            onChange={(event) => void run(
              async () => setDock(await dockApi.setIconSize(event.target.value as DrawerIconSize)),
              "Tamaño de iconos guardado.",
            )}
          >
            {ICON_SIZES.map((size) => (
              <option key={size.value} value={size.value}>{size.label}</option>
            ))}
          </select>
        </label>

        <label className="preference-row">
          <span>
            <strong>Opacidad del fondo</strong>
            <small>{Math.round((dock?.opacity ?? 0.92) * 100)} %</small>
          </span>
          <input
            type="range"
            min={35}
            max={100}
            step={1}
            value={Math.round((dock?.opacity ?? 0.92) * 100)}
            disabled={!dock}
            onChange={(event) => void run(async () => {
              setDock(await dockApi.updateSettings({ opacity: Number(event.target.value) / 100 }));
            })}
          />
        </label>
      </div>

      <div className="dock-settings-actions">
        <button
          className="button button-secondary"
          disabled={!dock?.enabled}
          onClick={() => void run(async () => setDock(await dockApi.setVisible(true)))}
        >
          Mostrar Dock
        </button>
        <button
          className="button button-ghost"
          disabled={!dock?.enabled}
          onClick={() => void run(async () => setDock(await dockApi.setVisible(false)))}
        >
          Ocultar Dock
        </button>
        <button
          className="button button-ghost"
          disabled={!dock?.enabled}
          onClick={() => void run(
            async () => setDock(await dockApi.relayout()),
            "Dock reubicado en el área útil del monitor.",
          )}
        >
          Reubicar
        </button>
        <button className="button button-ghost" disabled={!dock?.enabled}
          onClick={() => void run(async () => setDock(await dockApi.refreshAvailability()), "Dock actualizado.")}>
          Actualizar Dock
        </button>
        <button className="button button-ghost" disabled={!dock?.enabled}
          onClick={() => void run(async () => setDock(await dockApi.addSeparator()), "Separador añadido.")}>
          Añadir separador
        </button>
        <span className="dock-settings-count">
          {dock ? `${dock.items.length} ${dock.items.length === 1 ? "acceso" : "accesos"}` : "…"}
        </span>
      </div>
    </section>
  );
}
