import { useEffect } from "react";
import { AdminWindow } from "./components/admin/AdminWindow";
import { DrawerWindow } from "./components/drawer/DrawerWindow";
import { DockHandle } from "./components/dock/DockHandle";
import { DockWindow } from "./components/dock/DockWindow";
import { PanelWindow } from "./components/panel/PanelWindow";
import { drawerApi } from "./services/drawerApi";
import type { Preferences } from "./types/drawer";
import { resolveWindowKind } from "./windowKind";

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
  const kind = resolveWindowKind();

  if (kind === "dock") return <DockWindow />;
  if (kind === "dock-handle") return <DockHandle />;
  if (kind === "panel") return <PanelWindow panelId={params.get("panel")!} />;
  if (kind === "drawer") return <DrawerWindow drawerId={params.get("drawer")!} />;

  return <AdminWindow />;
}
