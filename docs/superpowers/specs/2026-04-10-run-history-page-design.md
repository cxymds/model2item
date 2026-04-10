# Run History Page Design

## Goal

Add a dedicated history page that shows the latest comparison runs, keeps active runs visible, and lets users open any historical run result.

## User Experience

- Add a new sidebar entry: `历史运行`.
- The history page shows up to 20 runs ordered by `created_at` descending.
- The top of the page has a `当前运行` section.
- Runs with status `queued` or `running` appear in `当前运行`.
- Each active run links to its monitor page at `/runs/:runId`.
- A second section, `最近结果`, lists the same recent runs for historical browsing.
- Completed or failed runs link to `/runs/:runId/results`.
- Queued or running runs in the recent list link to `/runs/:runId`.

## Data Shape

- Add a frontend Tauri binding for listing comparison runs.
- Add a backend command that returns the most recent 20 runs.
- Reuse the existing `ComparisonRunResponse` shape for the history list.
- No pagination in this iteration.

## Page Structure

### Sidebar

- Keep the existing `运行任务` entry for creating new runs.
- Add a separate `历史运行` entry pointing to the new page.

### History Page

- Header with title and a short description.
- `当前运行` section:
  - Empty state when there are no active runs.
  - Card/list rows showing title, status, created time, and a continue button.
- `最近结果` section:
  - Up to 20 rows.
  - Each row shows title, status, created time, finished time if present, and an open button.

## Behavior

- Poll the history page so active runs stay fresh without manual refresh.
- Treat `queued` and `running` as active states.
- Keep the existing recent-run shortcut logic; it remains useful as a quick return path.

## Error Handling

- Show a page-level error message if the history query fails.
- Show clear empty states for:
  - no active runs
  - no historical runs

## Testing

- Backend test for listing recent runs in descending order with a 20-item limit.
- Frontend test for rendering active runs separately from historical rows.
- Frontend test for link targets:
  - active runs open monitor pages
  - completed/failed runs open results pages
