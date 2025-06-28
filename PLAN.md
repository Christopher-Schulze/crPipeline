# Development Plan

This document tracks remaining improvements and future work items identified across the codebase.

- [ ] Create and integrate a reusable **GlobalLoadingIndicator** component.
- [ ] Dispatch a global `pipelinesUpdated` event after saving pipelines so other pages can refresh automatically.
- [ ] Update the pipelines list page to listen for the `pipelinesUpdated` event and refresh the list.
- [ ] Show an error message in the Pipeline Editor UI when loading prompt templates fails.
- [ ] Expand Markdown to PDF conversion to support lists, tables and other elements.
- [ ] Attempt S3 cleanup if document creation fails or analysis cannot be queued due to quota.
- [ ] Add additional organization management tests in `backend/tests/org_management_tests.rs`.
