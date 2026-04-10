import { fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { CaseForm } from "./CaseForm";

describe("CaseForm", () => {
  it("blocks submit when context payload is invalid JSON", () => {
    const onSubmit = vi.fn();
    render(<CaseForm isPending={false} onSubmit={onSubmit} />);

    fireEvent.change(screen.getByPlaceholderText("Legacy parser walkthrough"), {
      target: { value: "Legacy parser case" },
    });
    fireEvent.change(screen.getByPlaceholderText("Analyze this old codebase module and explain..."), {
      target: { value: "Analyze parser" },
    });

    const contextLabel = screen.getByText("Context payload (JSON string)");
    const contextTextArea = contextLabel.closest("label")?.querySelector("textarea");
    if (!contextTextArea) throw new Error("context payload textarea missing");

    fireEvent.change(contextTextArea, { target: { value: "{invalid-json}" } });
    fireEvent.click(screen.getByRole("button", { name: "Save evaluation case" }));

    expect(onSubmit).not.toHaveBeenCalled();
    expect(screen.getByText("Context payload must be valid JSON.")).toBeInTheDocument();
  });
});
