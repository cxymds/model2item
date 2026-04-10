import { fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { CaseForm } from "./CaseForm";

describe("CaseForm", () => {
  it("blocks submit when context payload is invalid JSON", () => {
    const onSubmit = vi.fn();
    render(<CaseForm isPending={false} onSubmit={onSubmit} />);

    fireEvent.change(screen.getByPlaceholderText("旧版解析器走查"), {
      target: { value: "Legacy parser case" },
    });
    fireEvent.change(screen.getByPlaceholderText("分析这个旧代码模块并说明其实现逻辑..."), {
      target: { value: "Analyze parser" },
    });

    const contextLabel = screen.getByText("上下文载荷（JSON 字符串）");
    const contextTextArea = contextLabel.closest("label")?.querySelector("textarea");
    if (!contextTextArea) throw new Error("context payload textarea missing");

    fireEvent.change(contextTextArea, { target: { value: "{invalid-json}" } });
    fireEvent.click(screen.getByRole("button", { name: "保存评测案例" }));

    expect(onSubmit).not.toHaveBeenCalled();
    expect(screen.getByText("上下文载荷必须是合法的 JSON。")).toBeInTheDocument();
  });
});
