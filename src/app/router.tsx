import { createBrowserRouter, Navigate } from "react-router-dom";
import App from "../App";
import { CaseLibraryPage } from "../features/cases/pages/CaseLibraryPage";
import { TargetConfigPage } from "../features/targets/pages/TargetConfigPage";
import { CreateRunPage } from "../features/runs/pages/CreateRunPage";
import { RunMonitorPage } from "../features/runs/pages/RunMonitorPage";
import { RunResultsPage } from "../features/runs/pages/RunResultsPage";
import { SettingsPage } from "../features/settings/pages/SettingsPage";

export const router = createBrowserRouter([
  {
    path: "/",
    element: <App />,
    children: [
      { index: true, element: <Navigate to="/runs/new" replace /> },
      { path: "runs/new", element: <CreateRunPage /> },
      { path: "runs/:runId", element: <RunMonitorPage /> },
      { path: "runs/:runId/results", element: <RunResultsPage /> },
      { path: "cases", element: <CaseLibraryPage /> },
      { path: "targets", element: <TargetConfigPage /> },
      { path: "settings", element: <SettingsPage /> }
    ]
  }
]);
