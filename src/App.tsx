import { Outlet } from "react-router-dom";
import { AppShell } from "./features/layout/AppShell";

export default function App() {
  return (
    <AppShell>
      <Outlet />
    </AppShell>
  );
}
