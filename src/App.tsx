import { AdminWindow } from "./components/admin/AdminWindow";
import { DrawerWindow } from "./components/drawer/DrawerWindow";
import { DockHandle } from "./components/dock/DockHandle";
import { DockWindow } from "./components/dock/DockWindow";

export function App() {
  const params = new URLSearchParams(window.location.search);
  const drawerId = params.get("drawer");
  const view = params.get("view");

  if (view === "dock") return <DockWindow />;
  if (view === "dock-handle") return <DockHandle />;

  return drawerId ? <DrawerWindow drawerId={drawerId} /> : <AdminWindow />;
}
