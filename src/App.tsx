import { useEffect } from "react";
import { AdminWindow } from "./components/admin/AdminWindow";
import { DrawerWindow } from "./components/drawer/DrawerWindow";
import { DockHandle } from "./components/dock/DockHandle";
import { DockWindow } from "./components/dock/DockWindow";
import { PanelWindow } from "./components/panel/PanelWindow";
import { drawerApi } from "./services/drawerApi";
import type { Preferences } from "./types/drawer";

export function App() {
  useEffect(() => {
    const apply = (preferences: Preferences) => {
      document.documentElement.dataset.performanceMode = preferences.performanceMode ? "on" : "off";
      document.documentElement.dataset.animationMode = preferences.animationMode;
    };
    void drawerApi.getState().then((state) => apply(state.preferences)).catch(() => undefined);
    const listener = drawerApi.onStateChanged((state) => apply(state.preferences));
    return () => { void listener.then((unlisten) => unlisten()); };
  }, []);

  const params = new URLSearchParams(window.location.search);
  const drawerId = params.get("drawer");
  const panelId = params.get("panel");
  const view = params.get("view");

  if (view === "dock") return <DockWindow />;
  if (view === "dock-handle") return <DockHandle />;
  if (panelId) return <PanelWindow panelId={panelId} />;

  return drawerId ? <DrawerWindow drawerId={drawerId} /> : <AdminWindow />;
}
