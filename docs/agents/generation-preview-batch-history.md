# Generation Preview and Batch History

Date: 2026-07-15

Generate treats one click as a `batch`, each submitted NovelAI generation job as a `request`, and each returned image as a parallel `sample` slot. Request order comes from `request_index`; samples use `sample_index` and do not imply queue order.

The live preview store is intentionally bounded to the current 8 × 4 maximum. A slot is keyed by `batch_id / job_id / sample_index` and keeps only its newest streaming frame. A persisted final `ResourceRef` replaces that frame, which releases the stream base64 from UI state. Durable batch snapshots and history details remain in TanStack Query and SQLite rather than Zustand.

The preview has three levels: a horizontal request cursor with 2 × 2 miniature grids, the selected request's 1–4 sample grid, and an in-place single-sample focus that can open the shared `AppModal` lightbox. Stable placeholders remain for failed, stopped, skipped, or missing samples so updates do not reorder the grid.

Selection uses `follow` and `pin` modes. Live events move `follow` to the newest active request and scroll its cursor unit into view. Selecting a request, sample, or history batch enters `pin`; live events continue to update caches without stealing focus. A compact icon returns to the latest live batch and restores `follow`.

Generation history is paged and filtered by distinct batch. Batch status is aggregated, with mixed successful terminal requests reported as `partially_succeeded`. The main preview renders both active snapshots and history details through the same batch view model. Batch and request exports use host-owned ZIP writing with stable `request-01_sample-01.ext` names; sample export remains a single-image save.

Deleting history removes only run-history and run-output records. It does not delete Gallery items, artifacts, resource catalog entries, or image files.
