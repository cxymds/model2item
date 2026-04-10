# UI Chinese Localization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将当前项目所有用户可见的前端操作界面文案统一改为中文，同时保持内部代码标识与接口不变。

**Architecture:** 直接在现有 React 组件中替换用户可见字符串，不引入新的国际化框架。先更新依赖可见文案的 Vitest 测试，再实现组件文案翻译，最后通过测试与构建验证无回归。

**Tech Stack:** React、TypeScript、Vitest、Testing Library、Vite

---

### Task 1: 锁定中文界面文案测试

**Files:**
- Modify: `src/features/layout/AppShell.test.tsx`
- Modify: `src/features/runs/pages/CreateRunPage.test.tsx`
- Modify: `src/features/cases/components/CaseForm.test.tsx`
- Modify: `src/features/targets/pages/TargetConfigPage.test.tsx`

- [ ] **Step 1: 写出失败中的中文文案断言**

```tsx
expect(screen.getByText("运行任务")).toBeInTheDocument();
expect(screen.getByRole("button", { name: "开始运行" })).toBeInTheDocument();
expect(screen.getByText("上下文载荷必须是合法的 JSON。")).toBeInTheDocument();
```

- [ ] **Step 2: 运行相关测试确认红灯**

Run: `npm test -- src/features/layout/AppShell.test.tsx src/features/runs/pages/CreateRunPage.test.tsx src/features/cases/components/CaseForm.test.tsx src/features/targets/pages/TargetConfigPage.test.tsx`
Expected: FAIL，提示界面仍然渲染英文文案。

- [ ] **Step 3: 保留失败断言，等待实现**

```tsx
// 不回退断言，直接进入组件实现阶段
```

- [ ] **Step 4: 提交前再次运行同一组测试**

Run: `npm test -- src/features/layout/AppShell.test.tsx src/features/runs/pages/CreateRunPage.test.tsx src/features/cases/components/CaseForm.test.tsx src/features/targets/pages/TargetConfigPage.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/features/layout/AppShell.test.tsx src/features/runs/pages/CreateRunPage.test.tsx src/features/cases/components/CaseForm.test.tsx src/features/targets/pages/TargetConfigPage.test.tsx
git commit -m "test: lock chinese ui copy"
```

### Task 2: 翻译前端可见文案

**Files:**
- Modify: `src/features/layout/AppShell.tsx`
- Modify: `src/features/runs/pages/CreateRunPage.tsx`
- Modify: `src/features/runs/pages/RunMonitorPage.tsx`
- Modify: `src/features/runs/pages/RunResultsPage.tsx`
- Modify: `src/features/runs/components/RunTargetStatusCard.tsx`
- Modify: `src/features/runs/components/MetricTable.tsx`
- Modify: `src/features/cases/pages/CaseLibraryPage.tsx`
- Modify: `src/features/cases/components/CaseForm.tsx`
- Modify: `src/features/targets/pages/TargetConfigPage.tsx`
- Modify: `src/features/targets/components/ProfileForm.tsx`
- Modify: `src/features/targets/components/WindowBindingList.tsx`
- Modify: `src/features/settings/pages/SettingsPage.tsx`

- [ ] **Step 1: 翻译导航与页面标题**

```tsx
const navItems = [
  { to: "/runs/new", label: "运行任务" },
  { to: "/cases", label: "案例库" },
  { to: "/targets", label: "目标配置" },
  { to: "/settings", label: "设置" },
];
```

- [ ] **Step 2: 翻译表单标签、占位文案、按钮文案与错误前缀**

```tsx
<span>任务标题</span>
<option value="">请选择已保存案例</option>
<p className="error-text">加载案例失败。{String(casesQuery.error)}</p>
```

- [ ] **Step 3: 翻译状态卡、结果页、设置页和列表文案**

```tsx
<span className={`status-pill ${status}`}>{statusLabelMap[status]}</span>
<h2>运行结果</h2>
<span>连接状态：{isOnline ? "在线" : "离线"}</span>
```

- [ ] **Step 4: 运行受影响测试确认绿灯**

Run: `npm test -- src/features/layout/AppShell.test.tsx src/features/runs/pages/CreateRunPage.test.tsx src/features/cases/components/CaseForm.test.tsx src/features/targets/pages/TargetConfigPage.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/features/layout/AppShell.tsx src/features/runs/pages/CreateRunPage.tsx src/features/runs/pages/RunMonitorPage.tsx src/features/runs/pages/RunResultsPage.tsx src/features/runs/components/RunTargetStatusCard.tsx src/features/runs/components/MetricTable.tsx src/features/cases/pages/CaseLibraryPage.tsx src/features/cases/components/CaseForm.tsx src/features/targets/pages/TargetConfigPage.tsx src/features/targets/components/ProfileForm.tsx src/features/targets/components/WindowBindingList.tsx src/features/settings/pages/SettingsPage.tsx
git commit -m "feat: translate visible ui copy to chinese"
```

### Task 3: 完整验证

**Files:**
- Modify: `src/features/runs/pages/RunResultsPage.test.tsx`

- [ ] **Step 1: 补齐结果页中文断言**

```tsx
expect(await screen.findByText("运行结果")).toBeInTheDocument();
```

- [ ] **Step 2: 运行完整前端测试**

Run: `npm test`
Expected: PASS，所有 Vitest 用例通过。

- [ ] **Step 3: 运行构建验证**

Run: `npm run build`
Expected: PASS，TypeScript 编译与 Vite 打包成功。

- [ ] **Step 4: 记录验证结果并整理变更说明**

```text
记录测试与构建命令、退出码和是否有残留风险。
```

- [ ] **Step 5: Commit**

```bash
git add src/features/runs/pages/RunResultsPage.test.tsx docs/superpowers/plans/2026-04-10-ui-chinese-localization.md
git commit -m "docs: add ui chinese localization plan"
```
