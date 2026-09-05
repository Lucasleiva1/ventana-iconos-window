import { AdminWindow } from "./components/admin/AdminWindow";
import { DrawerWindow } from "./components/drawer/DrawerWindow";

export function App() {
  const params = new URLSearchParams(window.location.search);
  const drawerId = params.get("drawer");

  return drawerId ? <DrawerWindow drawerId={drawerId} /> : <AdminWindow />;
}

