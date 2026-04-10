# Binding And Case Management Design

## Goal

为已绑定的 iTerm2 窗口和已保存的评测案例增加编辑、删除能力，同时保证已经被运行任务引用的数据不能删除。

## Current State

- 目标配置页目前支持创建模型配置、创建窗口绑定、刷新会话状态，但“当前绑定”仅支持只读展示。
- 案例库目前支持创建案例和查看案例摘要，但“已保存案例”仅支持只读展示。
- 运行任务会在创建时对案例和目标绑定做快照，因此历史运行不依赖后续编辑后的最新内容。
- 当前后端没有 update/delete binding 和 update/delete case 的命令、服务和前端调用。

## Proposed UX

### Window Bindings

- 在“当前绑定”列表的每张卡片上增加“编辑”和“删除”按钮。
- 点击“编辑”后，该卡片切换为行内编辑表单，允许修改：
  - `display_name`
  - `iterm_session_id`
  - `profile_id`
- 编辑时保留在线状态、最近在线、当前绑定配置等辅助信息，提交后回到只读卡片。
- 点击“删除”时先弹出浏览器确认框；确认后调用删除接口。
- 若该绑定已被任一运行任务引用，后端拒绝删除，前端展示明确错误。

### Evaluation Cases

- 在“已保存案例”列表的每张卡片上增加“编辑”和“删除”按钮。
- 点击“编辑”后，该卡片切换为行内表单，字段与创建案例保持一致：
  - `title`
  - `prompt`
  - `context_payload`
  - `notes`
- 编辑表单继续复用 JSON 校验规则。
- 点击“删除”时先弹出确认框；确认后调用删除接口。
- 若该案例已被任一运行任务引用，后端拒绝删除，前端展示明确错误。

## Backend Design

### Window Binding APIs

- 新增输入类型 `UpdateWindowBindingInput`。
- 新增服务方法：
  - `update_window_binding(id, input)`
  - `delete_window_binding(id)`
- 删除前检查 `comparison_targets.window_binding_id` 是否存在引用。
- 若存在引用，返回 `AppError::InvalidInput("window binding is referenced by comparison runs")`。
- 更新时继续校验 `profile_id` 是否存在。

### Evaluation Case APIs

- 新增输入类型 `UpdateEvaluationCaseInput`。
- 新增服务方法：
  - `update_evaluation_case(id, input)`
  - `delete_evaluation_case(id)`
- 删除前检查 `comparison_runs.evaluation_case_id` 是否存在引用。
- 若存在引用，返回 `AppError::InvalidInput("evaluation case is referenced by comparison runs")`。
- 更新时继续校验 `context_payload` 是合法 JSON。

### Commands And Frontend Bridge

- Tauri command 层新增：
  - `update_window_binding`
  - `delete_window_binding`
  - `update_evaluation_case`
  - `delete_evaluation_case`
- 前端 `src/lib/tauri.ts` 补充对应调用方法。
- `src/types/api.ts` 补充 update input 类型。

## Frontend Design

### TargetConfigPage / WindowBindingList

- 继续沿用当前“创建表单 + 列表卡片”布局，不新增路由。
- `WindowBindingList` 增加：
  - 编辑中的 `bindingId`
  - 编辑表单草稿
  - 每条记录独立的保存/取消/删除按钮
- 成功编辑或删除后刷新 `window-bindings`。
- 删除失败时在组件中显示错误文案。

### CaseLibraryPage / CaseForm

- 保持左侧创建、右侧管理的布局。
- `CaseForm` 扩展为可接收初始值、提交文案、提交后是否自动清空，从而同时服务“创建”和“编辑”。
- `CaseLibraryPage` 中右侧卡片支持切换为编辑模式。
- 成功编辑或删除后刷新 `evaluation-cases`。
- 删除失败时显示引用保护错误。

## Error Handling

- 删除受引用保护时，错误直接提示为“已被运行任务引用，暂时不能删除”。
- 更新校验失败时继续沿用现有 JSON/缺失配置等错误机制。
- 对于未知后端错误，前端保持现有 `String(error)` 透传策略。

## Testing

- Rust 服务测试：
  - 可更新窗口绑定
  - 被运行任务引用的窗口绑定不能删除
  - 可更新评测案例
  - 被运行任务引用的评测案例不能删除
- React 测试：
  - 目标配置页可触发编辑/删除绑定
  - 案例库可触发编辑/删除案例
  - 删除失败时显示中文错误文案

## Non-Goals

- 本轮不增加“被引用计数”展示。
- 本轮不增加软删除或回收站。
- 本轮不支持编辑模型配置本身。
