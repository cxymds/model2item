# Auto Cleanup Closed Bindings Design

## Goal

在 iTerm2 session 消失时，系统自动清理失效窗口绑定；若绑定已经被运行任务引用，则保留记录但不再视为在线可用。

## Proposed Behavior

- 应用启动后在 Rust 后台启动定时同步任务。
- 同步周期初版使用 15 秒。
- 每次同步都会读取当前 iTerm2 session 列表，并和 `window_bindings.iterm_session_id` 对比。
- 若绑定对应 session 仍在线：
  - 更新 `last_seen_at`
- 若绑定对应 session 已消失：
  - 若该绑定未被任何 `comparison_targets` 引用，自动删除
  - 若该绑定已被运行任务引用，保留记录，不自动删除

## UX Impact

- 目标配置页的“当前绑定”会在后台同步后自然减少失效且未引用的绑定。
- 已被引用的绑定仍会显示，但通常表现为离线。
- 页面提示文案补充“系统会自动清理已关闭且未被运行任务引用的绑定”。

## Backend Design

- 在 `WindowBindingService` 中新增统一同步方法，例如：
  - `sync_with_online_sessions(online_session_ids: &[String])`
- 该方法负责：
  - 更新在线绑定的 `last_seen_at`
  - 删除未引用且离线的绑定
  - 保留已引用且离线的绑定
- `refresh_window_binding_presence` 命令改为复用这套同步逻辑。
- `lib.rs` 在 Tauri 启动时后台 `spawn` 定时任务，循环调用同步逻辑。
- 若 iTerm2 / Python bridge 暂时不可用，后台同步跳过本轮，不中断应用。

## Testing

- Rust 服务测试：
  - 同步时删除未引用且离线的绑定
  - 同步时保留已被运行引用的离线绑定
- 前端测试：
  - 目标配置页显示新的自动清理提示文案

## Non-Goals

- 本轮不实现引用计数展示
- 本轮不实现“失效”专门状态字段
- 本轮不实现自动恢复旧 session 与新 session 的重新绑定
