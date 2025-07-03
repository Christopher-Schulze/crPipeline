import { render, fireEvent } from "@testing-library/svelte";
import { vi, expect, test, beforeEach } from "vitest";
import { tick } from "svelte";

import * as apiUtils from "$lib/utils/apiUtils";
const apiFetch = vi.spyOn(apiUtils, "apiFetch").mockResolvedValue({
  ok: true,
  json: async () => ({ prompt_templates: [] }),
});

import PipelineEditor from "../PipelineEditor.svelte";

vi.stubGlobal("alert", vi.fn());
vi.stubGlobal(
  "confirm",
  vi.fn(() => true),
);

const initialPipeline = {
  id: "p1",
  name: "Test",
  org_id: "org1",
  stages: [],
};

beforeEach(() => {
  apiFetch.mockClear();
});

test("adds and removes stages", async () => {
  const { getByText, getByPlaceholderText, container } = render(
    PipelineEditor,
    { props: { orgId: "org1", initialPipeline } },
  );
  await tick();
  await tick();

  await fireEvent.input(getByPlaceholderText("New Stage Type"), {
    target: { value: "parse" },
  });
  await fireEvent.click(getByText("Add Stage"));
  await tick();
  await tick();

  expect(container.querySelectorAll(".stage-item").length).toBe(1);

  const removeButton = container.querySelector(
    ".stage-item button",
  ) as HTMLElement;
  await fireEvent.click(removeButton);
  await tick();

  expect(container.querySelectorAll(".stage-item").length).toBe(0);
});

test("uses apiFetch for saving and deleting pipeline", async () => {
  const { getByText, getByPlaceholderText } = render(PipelineEditor, {
    props: { orgId: "org1", initialPipeline },
  });
  await tick();

  await fireEvent.input(getByPlaceholderText("New Stage Type"), {
    target: { value: "parse" },
  });
  await fireEvent.click(getByText("Add Stage"));
  await tick();
  await fireEvent.click(getByText("Save"));
  await tick();
  await tick();

  expect(apiFetch).toHaveBeenCalledWith(
    "/api/pipelines/p1",
    expect.objectContaining({ method: "PUT" }),
  );
  apiFetch.mockClear();

  await fireEvent.click(getByText("Delete"));
  await tick();
  await tick();
  expect(apiFetch).toHaveBeenCalledWith("/api/pipelines/p1", {
    method: "DELETE",
  });
});

test("renders delimiter regex field for table extraction", async () => {
  const { getByText, getByPlaceholderText } = render(PipelineEditor, {
    props: { orgId: "org1", initialPipeline },
  });
  await tick();
  await tick();

  await fireEvent.input(getByPlaceholderText("New Stage Type"), {
    target: { value: "parse" },
  });
  await fireEvent.click(getByText("Add Stage"));
  await tick();

  // Switch parse strategy to SimpleTableExtraction
  const strategySelect = document.querySelector("select") as HTMLSelectElement;
  strategySelect.value = "SimpleTableExtraction";
  await fireEvent.change(strategySelect);
  await tick();

  expect(
    getByPlaceholderText("optional, defaults to whitespace or '|'"),
  ).toBeTruthy();
  expect(getByText("Generate Numeric Summary")).toBeTruthy();
});
