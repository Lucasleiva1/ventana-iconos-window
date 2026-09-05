import { AdminWindow } from "./components/admin/AdminWindow";
import { DrawerWindow } from "./components/drawer/DrawerWindow";
import { DockHandle } from "./components/dock/DockHandle";
import { DockWindow } from "./components/dock/DockWindow";
import { PanelWindow } from "./components/panel/PanelWindow";

export function App() {
  const params = new URLSearchParams(window.location.search);
  const drawerId = params.get("drawer");
  const panelId = params.get("panel");
  const view = params.get("view");

  if (view === "dock") return <DockWindow />;
  if (view === "dock-handle") return <DockHandle />;
  if (panelId) return <PanelWindow panelId={panelId} />;

  return drawerId ? <DrawerWindow drawerId={drawerId} /> : <AdminWindow />;
}
